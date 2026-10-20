use super::{
    inventory as inv,
    plan::{self, Frozen, Plan, Step},
};
use crate::Result;
use fs2::FileExt;
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

pub struct Budget {
    pub deadline: Instant,
    pub output_limit: u64,
    pub artifact_limit: u64,
    pub output: AtomicU64,
    pub artifacts: AtomicU64,
}
impl Budget {
    pub fn exhausted(&self) -> bool {
        Instant::now() >= self.deadline
            || self.output.load(Ordering::SeqCst) > self.output_limit
            || self.artifacts.load(Ordering::SeqCst) > self.artifact_limit
    }
}

#[cfg(unix)]
fn capture<T: Read + std::os::fd::AsRawFd + Send + 'static>(
    mut stream: T,
    path: &Path,
    limit: u64,
    stop: Arc<AtomicBool>,
    overflow: Arc<AtomicBool>,
    budget: Option<Arc<Budget>>,
) -> Result<std::thread::JoinHandle<std::result::Result<Value, String>>> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let path = path.to_path_buf();
    // Only the pipe read end is made nonblocking; writers keep their normal flags.
    let flags = unsafe { libc::fcntl(stream.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(stream.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(std::thread::spawn(move || {
        let (mut observed, mut retained) = (0u64, 0u64);
        let mut buffer = [0u8; 8192];
        let mut stopped = None;
        loop {
            if stop.load(Ordering::SeqCst) {
                let since = stopped.get_or_insert_with(Instant::now);
                if since.elapsed() > Duration::from_millis(200) {
                    break;
                }
            }
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    observed += n as u64;
                    if let Some(b) = &budget {
                        b.output.fetch_add(n as u64, Ordering::SeqCst);
                    }
                    let keep = (limit.saturating_sub(retained) as usize).min(n);
                    file.write_all(&buffer[..keep]).map_err(|e| e.to_string())?;
                    retained += keep as u64;
                    if observed > limit {
                        overflow.store(true, Ordering::SeqCst);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        file.sync_all().map_err(|e| e.to_string())?;
        Ok(
            json!({"observed_bytes":observed,"retained_bytes":retained,"truncated":observed>retained,"sha256":inv::digest(&path).map_err(|e|e.to_string())?}),
        )
    }))
}

fn usage(dir: &Path) -> Result<(u64, u64)> {
    fn visit(p: &Path, total: &mut (u64, u64), depth: usize) -> Result<()> {
        if depth > 64 {
            return Err("output nesting exceeds limit".into());
        }
        for entry in fs::read_dir(p)? {
            let e = entry?;
            let m = fs::symlink_metadata(e.path())?;
            if m.file_type().is_symlink() {
                return Err("output contains a symlink".into());
            }
            if m.is_dir() {
                visit(&e.path(), total, depth + 1)?;
            } else if m.is_file() {
                total.0 = total.0.saturating_add(m.len());
                total.1 += 1;
            } else {
                return Err("output contains a special file".into());
            }
        }
        Ok(())
    }
    let mut total = (0, 0);
    visit(dir, &mut total, 0)?;
    Ok(total)
}

#[cfg(unix)]
fn step(
    dir: &Path,
    indexed: (usize, &Step),
    p: &Plan,
    frozen: &Frozen,
    deadline: Instant,
    budget: Option<Arc<Budget>>,
    last_artifacts: &mut u64,
) -> Result<Value> {
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    let (index, spec) = indexed;
    let folder = dir.join("evidence").join(format!("step-{index:03}"));
    fs::create_dir(&folder)?;
    let argv = spec
        .argv
        .iter()
        .map(|s| plan::expand(s, dir, frozen))
        .collect::<Result<Vec<_>>>()?;
    let cwd = plan::expand(&spec.cwd, dir, frozen)?;
    let mut env = p
        .env
        .iter()
        .map(|(k, v)| Ok((k.clone(), plan::expand(v, dir, frozen)?)))
        .collect::<Result<std::collections::BTreeMap<_, _>>>()?;
    env.insert("PYTHONDONTWRITEBYTECODE".into(), "1".into());
    inv::json(
        &folder.join("command.json"),
        &json!({"argv":argv,"cwd":cwd,"environment":env,"requested_at_ms":inv::now()}),
    )?;
    let mut command = Command::new(&argv[0]);
    command
        .args(&argv[1..])
        .current_dir(&cwd)
        .env_clear()
        .envs(&env)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let cpu = p.limits.cpu_time_seconds;
    let file_bytes = p.limits.artifact_bytes;
    // setrlimit is async-signal-safe. These are process limits, not a security sandbox.
    unsafe {
        command.pre_exec(move || {
            for (resource, amount) in [(libc::RLIMIT_CPU, cpu), (libc::RLIMIT_FSIZE, file_bytes)] {
                let limit = libc::rlimit {
                    rlim_cur: amount as libc::rlim_t,
                    rlim_max: amount as libc::rlim_t,
                };
                if libc::setrlimit(resource, &limit) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    let began = Instant::now();
    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            let result = json!({"process_status":"spawn_failed","pid":null,"error":e.to_string(),"argv":argv});
            inv::json(&folder.join("result.json"), &result)?;
            return Ok(result);
        }
    };
    let pid = child.id();
    let stop = Arc::new(AtomicBool::new(false));
    struct OwnedGroup {
        pid: libc::pid_t,
        stop: Arc<AtomicBool>,
    }
    impl Drop for OwnedGroup {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            unsafe {
                libc::kill(-self.pid, libc::SIGKILL);
                libc::waitpid(self.pid, std::ptr::null_mut(), libc::WNOHANG);
            }
        }
    }
    let _owned = OwnedGroup {
        pid: pid as libc::pid_t,
        stop: stop.clone(),
    };
    inv::json(
        &folder.join("start.json"),
        &json!({"pid":pid,"process_group":pid,"started_at_ms":inv::now()}),
    )?;
    let overflow = Arc::new(AtomicBool::new(false));
    let stdout = capture(
        child.stdout.take().ok_or("missing stdout")?,
        &folder.join("stdout"),
        p.limits.stdout_bytes,
        stop.clone(),
        overflow.clone(),
        budget.clone(),
    )?;
    let stderr = capture(
        child.stderr.take().ok_or("missing stderr")?,
        &folder.join("stderr"),
        p.limits.stderr_bytes,
        stop.clone(),
        overflow.clone(),
        budget.clone(),
    )?;
    let (mut timed_out, mut resource_limit, mut output_error) = (false, false, None);
    let (raw_status, rusage) = loop {
        let mut status = 0;
        let mut rusage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
        // wait4 collects actual child exit and OS-reported CPU/peak RSS without guessing from PID existence.
        let waited = unsafe {
            libc::wait4(
                pid as libc::pid_t,
                &mut status,
                libc::WNOHANG,
                rusage.as_mut_ptr(),
            )
        };
        if waited > 0 {
            break (status, unsafe { rusage.assume_init() });
        }
        if waited < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if Instant::now() >= deadline {
            timed_out = true;
        }
        if overflow.load(Ordering::SeqCst) || budget.as_ref().is_some_and(|b| b.exhausted()) {
            resource_limit = true;
        }
        match usage(&dir.join("output")) {
            Ok((bytes, files)) => {
                if bytes > p.limits.artifact_bytes || files > p.limits.artifact_files {
                    resource_limit = true;
                }
                if let Some(b) = &budget
                    && bytes > *last_artifacts
                {
                    b.artifacts
                        .fetch_add(bytes - *last_artifacts, Ordering::SeqCst);
                    *last_artifacts = bytes;
                }
            }
            Err(e) => {
                resource_limit = true;
                output_error = Some(e.to_string());
            }
        }
        if timed_out || resource_limit {
            // Negative PID targets only the fresh process group created for this child.
            unsafe {
                libc::kill(-(pid as libc::pid_t), libc::SIGKILL);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    // Close any descendants still holding pipes in this owned group. Escaped groups are outside this trusted-command boundary.
    unsafe {
        libc::kill(-(pid as libc::pid_t), libc::SIGKILL);
    }
    stop.store(true, Ordering::SeqCst);
    let stdout = stdout
        .join()
        .map_err(|_| "stdout capture panicked")?
        .map_err(|e| format!("stdout capture: {e}"))?;
    let stderr = stderr
        .join()
        .map_err(|_| "stderr capture panicked")?
        .map_err(|e| format!("stderr capture: {e}"))?;
    let status = std::process::ExitStatus::from_raw(raw_status);
    if let Ok((bytes, files)) = usage(&dir.join("output")) {
        if bytes > p.limits.artifact_bytes || files > p.limits.artifact_files {
            resource_limit = true;
        }
        if let Some(b) = &budget
            && bytes > *last_artifacts
        {
            b.artifacts
                .fetch_add(bytes - *last_artifacts, Ordering::SeqCst);
            *last_artifacts = bytes;
        }
    }
    resource_limit |= budget.as_ref().is_some_and(|b| b.exhausted())
        || overflow.load(Ordering::SeqCst)
        || status.signal() == Some(libc::SIGXFSZ)
        || status.signal() == Some(libc::SIGXCPU);
    let cpu_ms = (rusage.ru_utime.tv_sec + rusage.ru_stime.tv_sec) as u64 * 1000
        + (rusage.ru_utime.tv_usec + rusage.ru_stime.tv_usec) as u64 / 1000;
    let rss = rusage.ru_maxrss as u64;
    #[cfg(not(target_os = "macos"))]
    let rss = rss * 1024;
    let state = if timed_out {
        "timed_out"
    } else if resource_limit {
        "resource_limited"
    } else if status.success() {
        "command_succeeded"
    } else {
        "failed"
    };
    let result = json!({"process_status":state,"pid":pid,"process_group":pid,"exit_code":status.code(),"signal":status.signal(),"timed_out":timed_out,"resource_limited":resource_limit,"output_error":output_error,"wall_time_ms":began.elapsed().as_millis(),"cpu_time_ms":cpu_ms,"reported_peak_rss_bytes":rss,"usage_scope":"OS child usage, including descendants waited by the command; not a sandbox-wide accounting guarantee","stdout":stdout,"stderr":stderr});
    inv::json(&folder.join("result.json"), &result)?;
    Ok(result)
}
#[cfg(not(unix))]
fn step(
    _: &Path,
    _: (usize, &Step),
    _: &Plan,
    _: &Frozen,
    _: Instant,
    _: Option<Arc<Budget>>,
    _: &mut u64,
) -> Result<Value> {
    Err("bounded execution currently requires Unix process groups".into())
}

pub fn run(dir: &Path, budget: Option<Arc<Budget>>, batch: Option<&str>) -> Result<Value> {
    let (dir, p, frozen) = plan::load(dir)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(dir.join("run.lock"))?;
    lock.try_lock_exclusive()
        .map_err(|_| "attempt is already running")?;
    if dir.join("queue.json").exists() {
        let q: Value = inv::read_sealed(&dir.join("queue.json"))?;
        if q["batch"].as_str() != batch {
            return Err("attempt is reserved by a batch".into());
        }
    }
    inv::json(
        &dir.join("start.json"),
        &json!({"runner_pid":std::process::id(),"started_at_ms":inv::now(),"batch":batch}),
    )?;
    let began = Instant::now();
    let mut deadline = began + Duration::from_millis(p.limits.wall_time_ms);
    if let Some(b) = &budget {
        deadline = deadline.min(b.deadline);
    }
    fs::create_dir(dir.join("work"))?;
    fs::create_dir(dir.join("output"))?;
    fs::create_dir(dir.join("evidence"))?;
    let mut steps = Vec::new();
    let mut gaps = Vec::new();
    let mut state = "command_succeeded".to_owned();
    let mut last_artifacts = 0;
    for (i, s) in p.commands.iter().enumerate() {
        if Instant::now() >= deadline || budget.as_ref().is_some_and(|b| b.exhausted()) {
            state = "resource_limited".into();
            break;
        }
        match step(
            &dir,
            (i, s),
            &p,
            &frozen,
            deadline,
            budget.clone(),
            &mut last_artifacts,
        ) {
            Ok(result) => {
                let successful = result["process_status"] == "command_succeeded";
                state = result["process_status"].as_str().unwrap_or("failed").into();
                steps.push(result);
                if !successful {
                    break;
                }
            }
            Err(e) => {
                state = "runner_failed".into();
                gaps.push(e.to_string());
                break;
            }
        }
    }
    if let Err(e) = plan::load(&dir) {
        gaps.push(format!("post-run frozen dependency verification: {e}"));
    }
    let output = match inv::scan(&dir.join("output"), "artifacts", None) {
        Ok(v) => Some(v),
        Err(e) => {
            gaps.push(e.to_string());
            None
        }
    };
    let evidence = match inv::scan(&dir.join("evidence"), "process-evidence", None) {
        Ok(v) => Some(v),
        Err(e) => {
            gaps.push(e.to_string());
            None
        }
    };
    if state == "command_succeeded" {
        for path in &p.required_artifacts {
            if inv::checked(&dir.join("output"), path)
                .and_then(|p| inv::digest(&p))
                .is_err()
            {
                gaps.push(format!("missing/unsafe required artifact: {path}"));
            }
        }
    }
    let success = state == "command_succeeded" && gaps.is_empty();
    let result = json!({"id":p.id,"process_status":state,"evidence_status":if gaps.is_empty(){"unreviewed"}else{"invalid"},"finished_at_ms":inv::now(),"wall_time_ms":began.elapsed().as_millis(),"steps":steps,"not_started_commands":p.commands.len()-steps.len(),"gaps":gaps,"output_inventory":output,"process_inventory":evidence,"plan_sha256":frozen.plan_sha256,"exit_code":if success{0}else{3},"financial_validation":"supplied evaluator only; no independent fill, accounting or chain verification","promotion":"none"});
    inv::sealed_json(&dir.join("result.json"), &result)?;
    Ok(result)
}
pub fn verified_result(dir: &Path) -> Result<Value> {
    plan::load(dir)?;
    let result: Value = inv::read_sealed(&dir.join("result.json"))?;
    for (field, folder) in [
        ("output_inventory", "output"),
        ("process_inventory", "evidence"),
    ] {
        let inventory: inv::Inventory = serde_json::from_value(result[field].clone())
            .map_err(|_| "run lacks a verifiable artifact inventory")?;
        inv::verify(&dir.join(folder), &inventory)?;
    }
    Ok(result)
}
pub fn inspect(dir: &Path) -> Result<Value> {
    let (dir, p, frozen) = plan::load(dir)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(dir.join("run.lock"))?;
    let active = lock.try_lock_exclusive().is_err();
    let result = if dir.join("result.json").exists() {
        Some(verified_result(&dir)?)
    } else {
        None
    };
    let mut process_status = if active {
        "running".to_owned()
    } else if let Some(r) = &result {
        r["process_status"].as_str().unwrap_or("invalid").into()
    } else if dir.join("start.json").exists() {
        "interrupted".into()
    } else if dir.join("queue.json").exists() {
        "queued".into()
    } else {
        "planned".into()
    };
    if process_status == "queued" {
        let q: Value = inv::read_sealed(&dir.join("queue.json"))?;
        let batch = q["batch"].as_str().ok_or("invalid queue owner")?;
        inv::name(batch)?;
        let member = dir
            .parent()
            .and_then(Path::parent)
            .ok_or("invalid store")?
            .join("batches")
            .join(batch)
            .join("members")
            .join(format!("{}.json", p.id));
        if member.exists() {
            let m: Value = inv::read_sealed(&member)?;
            if let Some(s) = m["process_status"].as_str() {
                process_status = s.into();
            }
        }
    }
    let review = if dir.join("review.json").exists() {
        Some(inv::read_sealed::<Value>(&dir.join("review.json"))?)
    } else {
        None
    };
    let lifecycle = match review.as_ref().and_then(|r| r["disposition"].as_str()) {
        Some("retire") => "retired",
        Some(_) => "reviewed",
        None => process_status.as_str(),
    };
    Ok(
        json!({"id":p.id,"state":lifecycle,"process_status":process_status,"evidence_status":result.as_ref().map(|r|r["evidence_status"].clone()).unwrap_or(json!("unreviewed")),"result":result,"review":review,"plan_sha256":frozen.plan_sha256,"interruption_scope":"an abandoned runner is never resumed or automatically killed; retained child outcomes may be unknown"}),
    )
}
