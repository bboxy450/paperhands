//! Bounded wake schedules. ICQ owns native delivery; Paperhands never guesses sockets.
use crate::{Result, research::inventory as inv, setup};
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command as Process, Stdio},
    time::{Duration, Instant},
};

pub fn command() -> Command {
    let root = || Arg::new("root").long("root");
    let id = || Arg::new("id").long("id").required(true);
    Command::new("automation").about("Bounded project wake schedules, durable receipts and an owned cron entry; no automatic model or trading process")
        .subcommand(Command::new("bind").about("Bind this project's automation to an explicit or current ICQ recipient; create its own sender identity")
            .arg(root()).arg(Arg::new("recipient").long("recipient").help("ICQ public key; otherwise this exact registered harness")))
        .subcommand(Command::new("add").about("Register a fresh finite wake schedule; does not install cron or send a message")
            .arg(root()).arg(id()).arg(Arg::new("every").long("every").required(true).value_parser(clap::value_parser!(u64).range(60..=604800)).help("Interval in seconds, at least 60"))
            .arg(Arg::new("duration").long("duration").required(true).value_parser(clap::value_parser!(u64).range(60..=2592000)).help("Lifetime in seconds, at most 30 days"))
            .arg(Arg::new("max-wakes").long("max-wakes").required(true).value_parser(clap::value_parser!(u64).range(1..=10000)))
            .arg(Arg::new("message").long("message").required(true).help("Operator-authorized research brief; literal text, at most 8 KiB")))
        .subcommand(Command::new("cancel").about("Retain a cancellation decision; stop future wakes without deleting evidence").arg(root()).arg(id()).arg(Arg::new("reason").long("reason").required(true)))
        .subcommand(Command::new("status").about("Verify schedules, claims and wake receipts").arg(root()))
        .subcommand(Command::new("tick").about("Coalesce missed slots and send at most four due wakes; spent/uncertain slots are never replayed")
            .arg(root()).arg(Arg::new("dry-run").long("dry-run").action(ArgAction::SetTrue)))
        .subcommand(Command::new("install").about("Install or repair only this project's marked crontab block; preserve all other jobs")
            .arg(root()).arg(Arg::new("dry-run").long("dry-run").action(ArgAction::SetTrue)))
        .subcommand(Command::new("uninstall").about("Remove only this project's marked cron block; retain schedules and receipts")
            .arg(root()).arg(Arg::new("dry-run").long("dry-run").action(ArgAction::SetTrue)))
}

pub(crate) struct Output {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
    pub limited: bool,
}
/// Exact argv, concurrent bounded pipe draining, deadline, and only owned child groups.
pub(crate) fn bounded(program: &Path, args: &[String], input: Option<&[u8]>) -> Result<Output> {
    bounded_for(program, args, input, Duration::from_secs(10))
}
fn bounded_for(
    program: &Path,
    args: &[String],
    input: Option<&[u8]>,
    budget: Duration,
) -> Result<Output> {
    let mut command = Process::new(program);
    command
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    if input.is_some_and(|bytes| bytes.len() > 65536) {
        return Err("subprocess input exceeds 64 KiB".into());
    }
    let deadline = Instant::now() + budget;
    let mut child = command.spawn()?;
    let read = |pipe: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut out = Vec::new();
            pipe.take(65537).read_to_end(&mut out).map(|_| out)
        })
    };
    let stdout = read(Box::new(child.stdout.take().unwrap()));
    let stderr = read(Box::new(child.stderr.take().unwrap()));
    let writer = input.map(|bytes| {
        let bytes = bytes.to_vec();
        let mut stdin = child.stdin.take().unwrap();
        std::thread::spawn(move || stdin.write_all(&bytes))
    });
    let mut status = None;
    let mut timed_out = false;
    loop {
        if Instant::now() >= deadline {
            timed_out = true;
            #[cfg(unix)]
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            if status.is_none() {
                let _ = child.kill();
                status = Some(child.wait()?);
            }
            break;
        }
        if status.is_none() {
            status = child.try_wait()?;
        }
        if status.is_some()
            && stdout.is_finished()
            && stderr.is_finished()
            && writer.as_ref().is_none_or(|w| w.is_finished())
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    timed_out |= Instant::now() >= deadline;
    let status = status.ok_or("owned subprocess did not finish")?;
    let mut stdout = stdout.join().map_err(|_| "stdout reader failed")??;
    let mut stderr = stderr.join().map_err(|_| "stderr reader failed")??;
    if let Some(writer) = writer {
        let _ = writer.join().map_err(|_| "stdin writer failed")?;
    }
    let limited = stdout.len() > 65536 || stderr.len() > 65536;
    stdout.truncate(65536);
    stderr.truncate(65536);
    Ok(Output {
        code: status.code().unwrap_or(-1),
        stdout,
        stderr,
        timed_out,
        limited,
    })
}
fn executable(name: &str) -> Result<PathBuf> {
    for directory in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        let path = directory.join(name);
        if path.is_file() {
            return Ok(fs::canonicalize(path)?);
        }
    }
    Err(format!("{name} is required; install it before binding automation").into())
}
fn icq(binary: &Path, home: &Path, args: &[String], mutation: bool) -> Result<Value> {
    icq_until(binary, home, args, mutation, None)
}
fn icq_until(
    binary: &Path,
    home: &Path,
    args: &[String],
    mutation: bool,
    expires_at: Option<u64>,
) -> Result<Value> {
    let mut command = vec!["--home".into(), home.to_string_lossy().into_owned()];
    if mutation {
        let mut preview = command.clone();
        preview.push("--dry-run".into());
        preview.extend_from_slice(args);
        let out = bounded(binary, &preview, None)?;
        if out.code != 0 || (out.timed_out || out.limited) {
            return Err("ICQ preview refused; no messaging action attempted".into());
        }
        let value: Value = serde_json::from_slice(&out.stdout)?;
        if value["dry_run"] != true {
            return Err("ICQ preview did not validate the action".into());
        }
    }
    if expires_at.is_some_and(|deadline| inv::now() >= deadline) {
        return Err("schedule expired before ICQ mutation; no message send attempted".into());
    }
    command.extend_from_slice(args);
    let out = bounded(binary, &command, None)?;
    if out.timed_out || out.limited {
        return Err(
            "ICQ outcome unknown after command deadline or output limit; not retried".into(),
        );
    }
    if out.code != 0 {
        return Err("ICQ command failed; inspect ICQ readiness or mute state; not retried".into());
    }
    Ok(serde_json::from_slice(&out.stdout)?)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    schema_version: u32,
    root: PathBuf,
    project_id: String,
    icq_binary: PathBuf,
    icq_home: PathBuf,
    sender: String,
    recipient: String,
    created_at_ms: u64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Schedule {
    schema_version: u32,
    id: String,
    binding_sha256: String,
    start_at_ms: u64,
    expires_at_ms: u64,
    interval_ms: u64,
    max_wakes: u64,
    message: String,
}
fn store(root: &Path) -> Result<(PathBuf, fs::File)> {
    let root = inv::root(root)?;
    let directory = inv::subdirectory(&root, "automation")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    }
    for name in ["schedules", "attempts", "cancellations", "bindings"] {
        inv::subdirectory(&directory, name)?;
    }
    let lock = directory.join("automation.lock");
    if fs::symlink_metadata(&lock).is_ok() {
        inv::digest(&lock)?;
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock)?;
    fs2::FileExt::try_lock_exclusive(&file)
        .map_err(|_| "automation is busy; another owned tick/bind is running")?;
    Ok((directory, file))
}
fn binding_id(directory: &Path) -> Result<String> {
    let id = String::from_utf8(inv::read_bytes(&directory.join("binding-head"))?)?;
    if !inv::valid_hash(&id) {
        return Err("invalid local binding head".into());
    }
    Ok(id)
}
fn binding_for(directory: &Path, root: &Path, id: &str) -> Result<Binding> {
    if !inv::valid_hash(id) {
        return Err("invalid binding hash".into());
    }
    let path = directory.join("bindings").join(format!("{id}.json"));
    if inv::digest(&path)? != id {
        return Err("binding bytes changed".into());
    }
    let value: Binding = inv::read_sealed(&path)?;
    if value.schema_version != 1
        || value.root != root
        || !inv::valid_hash(&value.recipient)
        || !value.icq_binary.is_absolute()
        || !value.icq_home.is_absolute()
    {
        return Err(
            "automation binding mismatch; live controls cannot be copied between machines/stores"
                .into(),
        );
    }
    inv::name(&value.sender)?;
    Ok(value)
}
fn binding(directory: &Path, root: &Path) -> Result<Binding> {
    binding_for(directory, root, &binding_id(directory)?)
}
fn schedules(directory: &Path) -> Result<Vec<Schedule>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(directory.join("schedules"))? {
        let name = entry?
            .file_name()
            .to_str()
            .ok_or("non-UTF-8 schedule")?
            .to_owned();
        if name.ends_with(".sha256") {
            continue;
        }
        let id = name
            .strip_suffix(".json")
            .ok_or("unexpected schedule member")?;
        inv::name(id)?;
        names.push(id.to_owned());
        if names.len() > 128 {
            return Err("automation exceeds 128 schedules".into());
        }
    }
    names.sort();
    names
        .into_iter()
        .map(|id| {
            let value: Schedule =
                inv::read_sealed(&directory.join("schedules").join(format!("{id}.json")))?;
            if value.id != id
                || value.schema_version != 1
                || value.interval_ms < 60000
                || value.interval_ms > 604800000
                || !(1..=10000).contains(&value.max_wakes)
                || value.expires_at_ms <= value.start_at_ms
                || value.expires_at_ms - value.start_at_ms > 2592000000
                || value.message.trim().is_empty()
                || value.message.len() > 8192
            {
                return Err("invalid retained schedule".into());
            }
            Ok(value)
        })
        .collect()
}
fn state(directory: &Path, schedule: &Schedule, now: u64) -> Result<Value> {
    let store_root = directory.parent().ok_or("missing automation store root")?;
    binding_for(directory, store_root, &schedule.binding_sha256)?;
    let attempts = directory.join("attempts").join(&schedule.id);
    let mut claimed = Vec::new();
    for entry in fs::read_dir(&attempts)? {
        let name = entry?
            .file_name()
            .to_str()
            .ok_or("invalid wake member")?
            .to_owned();
        if name.ends_with(".sha256") {
            continue;
        }
        if let Some(slot) = name.strip_suffix(".intent.json") {
            let number = slot.parse::<u64>()?;
            let value: Value = inv::read_sealed(&entry_path(&attempts, &name)?)?;
            if value["slot"] != number
                || value["schedule_sha256"]
                    != inv::digest(
                        &directory
                            .join("schedules")
                            .join(format!("{}.json", schedule.id)),
                    )?
            {
                return Err("wake intent binding changed".into());
            }
            claimed.push(number);
        } else if name.ends_with(".result.json") {
            let _: Value = inv::read_sealed(&entry_path(&attempts, &name)?)?;
        } else {
            return Err("unexpected wake evidence member".into());
        }
        if claimed.len() > 10000 {
            return Err("too many wake intents".into());
        }
    }
    claimed.sort();
    let cancelled = directory
        .join("cancellations")
        .join(format!("{}.json", schedule.id));
    let cancelled = if cancelled.exists() {
        let _: Value = inv::read_sealed(&cancelled)?;
        true
    } else {
        false
    };
    let slot = now.saturating_sub(schedule.start_at_ms) / schedule.interval_ms;
    let status = if cancelled {
        "cancelled"
    } else if now >= schedule.expires_at_ms {
        "expired"
    } else if claimed.len() as u64 >= schedule.max_wakes {
        "budget_exhausted"
    } else if now < schedule.start_at_ms || claimed.contains(&slot) {
        "waiting"
    } else {
        "due"
    };
    let last = if let Some(last) = claimed.last() {
        let path = attempts.join(format!("{last}.result.json"));
        if path.exists() {
            inv::read_sealed::<Value>(&path)?
        } else {
            json!({"status":"interrupted_or_unknown","automatic_retry":false})
        }
    } else {
        Value::Null
    };
    Ok(
        json!({"id":schedule.id,"status":status,"due_slot":slot,"attempted_wakes":claimed.len(),"max_wakes":schedule.max_wakes,"expires_at_ms":schedule.expires_at_ms,"interval_ms":schedule.interval_ms,"last_receipt":last}),
    )
}
fn entry_path(root: &Path, name: &str) -> Result<PathBuf> {
    inv::checked(root, name)
}

pub fn execute(args: &ArgMatches) -> Result<Value> {
    let (action, args) = args.subcommand().ok_or("choose an automation operation")?;
    let root = inv::root(&crate::project::resolve(
        args.get_one::<String>("root").map(String::as_str),
    )?)?;
    let (directory, _lock) = store(&root)?;
    let now = inv::now();
    if action == "bind" {
        let binary = executable("icq")?;
        let home = std::env::var_os("ICQ_HOME")
            .map(PathBuf::from)
            .unwrap_or(PathBuf::from(std::env::var_os("HOME").ok_or("HOME unset")?).join(".icq"));
        let recipient = if let Some(key) = args.get_one::<String>("recipient") {
            key.clone()
        } else {
            icq(&binary, &home, &["id".into()], true)?["address"]
                .as_str()
                .ok_or("ICQ did not return this harness identity")?
                .to_owned()
        };
        if !inv::valid_hash(&recipient) {
            return Err("recipient must be an ICQ public key".into());
        }
        let path = directory.join("binding-head");
        let previous = setup::optional(&path)?;
        let old = if previous.is_some() {
            Some(binding(&directory, &root)?)
        } else {
            None
        };
        if let Some(old) = &old
            && old.recipient == recipient
        {
            return Ok(
                json!({"binding":old,"reused":true,"receiver_acceptance":"unverified","model_delivery":"unverified"}),
            );
        }
        let project_id = if root.join("project.json").exists() {
            let project: Value = inv::read_sealed(&root.join("project.json"))?;
            project["project_id"]
                .as_str()
                .ok_or("missing project ID")?
                .to_owned()
        } else {
            inv::hash(root.to_string_lossy().as_bytes())
        };
        let sender = old.as_ref().map(|b| b.sender.clone()).unwrap_or_else(|| {
            format!(
                "paperhands-{}",
                &inv::hash(format!("{}:{now}:{}", root.display(), std::process::id()).as_bytes())
                    [..24]
            )
        });
        let identity = if old.is_some() {
            icq(
                &binary,
                &home,
                &["--as".into(), sender.clone(), "id".into()],
                false,
            )?
        } else {
            icq(
                &binary,
                &home,
                &["identity".into(), "create".into(), sender.clone()],
                true,
            )?
        };
        if !identity["address"].as_str().is_some_and(inv::valid_hash) {
            return Err("ICQ sender identity unverified".into());
        }
        let value = Binding {
            schema_version: 1,
            root: root.clone(),
            project_id,
            icq_binary: binary,
            icq_home: home,
            sender,
            recipient,
            created_at_ms: now,
        };
        let bytes = serde_json::to_vec_pretty(&value)?;
        let sha = inv::hash(&bytes);
        inv::sealed_json(
            &directory.join("bindings").join(format!("{sha}.json")),
            &value,
        )?;
        setup::replace(&path, previous.as_deref(), sha.as_bytes(), None)?;
        return Ok(
            json!({"binding":value,"sender_address":identity["address"],"execution_started":false,"receiver_acceptance":"unverified","model_delivery":"unverified"}),
        );
    }
    if ["install", "uninstall"].contains(&action) {
        return cron(
            &root,
            &directory,
            action == "install",
            args.get_flag("dry-run"),
        );
    }
    if action == "add" {
        binding(&directory, &root)?;
        if schedules(&directory)?.len() >= 128 {
            return Err("automation schedule limit reached".into());
        }
        let id = args.get_one::<String>("id").unwrap();
        inv::name(id)?;
        let message = args.get_one::<String>("message").unwrap();
        if message.trim().is_empty() || message.len() > 8192 || message.contains('\0') {
            return Err("wake brief must contain 1..8192 UTF-8 bytes without NUL".into());
        }
        let value = Schedule {
            schema_version: 1,
            id: id.clone(),
            binding_sha256: binding_id(&directory)?,
            start_at_ms: now,
            expires_at_ms: now + args.get_one::<u64>("duration").unwrap() * 1000,
            interval_ms: args.get_one::<u64>("every").unwrap() * 1000,
            max_wakes: *args.get_one::<u64>("max-wakes").unwrap(),
            message: message.clone(),
        };
        fs::create_dir(directory.join("attempts").join(id))?;
        inv::sealed_json(
            &directory.join("schedules").join(format!("{id}.json")),
            &value,
        )?;
        return Ok(json!({"schedule":value,"sent":false,"scheduler_installed":false}));
    }
    if action == "cancel" {
        let id = args.get_one::<String>("id").unwrap();
        inv::name(id)?;
        let _: Schedule =
            inv::read_sealed(&directory.join("schedules").join(format!("{id}.json")))?;
        let reason = args.get_one::<String>("reason").unwrap();
        if reason.trim().is_empty() || reason.len() > 8192 {
            return Err("cancellation reason must contain 1..8192 bytes".into());
        }
        inv::sealed_json(
            &directory.join("cancellations").join(format!("{id}.json")),
            &json!({"id":id,"reason":reason,"recorded_at_ms":now}),
        )?;
        return Ok(json!({"id":id,"cancelled":true,"retained_evidence":true}));
    }
    let schedules = schedules(&directory)?;
    // Verify every selected schedule before sending; give least-served schedules
    // priority so a short cadence cannot indefinitely starve later names.
    let mut pending = schedules
        .into_iter()
        .map(|schedule| state(&directory, &schedule, now).map(|row| (schedule, row)))
        .collect::<Result<Vec<_>>>()?;
    pending.sort_by(|(a, x), (b, y)| {
        x["attempted_wakes"]
            .as_u64()
            .cmp(&y["attempted_wakes"].as_u64())
            .then(a.id.cmp(&b.id))
    });
    let tick_start = Instant::now();
    let mut items = Vec::new();
    let mut sends = 0;
    for (schedule, mut row) in pending {
        if action == "tick"
            && row["status"] == "due"
            && !args.get_flag("dry-run")
            && sends < 4
            && tick_start.elapsed() < Duration::from_secs(30)
        {
            let owner = binding_for(&directory, &root, &schedule.binding_sha256)?;
            let slot = row["due_slot"].as_u64().unwrap();
            let attempt = directory.join("attempts").join(&schedule.id);
            let intent = json!({"schedule_sha256":inv::digest(&directory.join("schedules").join(format!("{}.json",schedule.id)))?,"slot":slot,"claimed_at_ms":inv::now(),"automatic_retry":false});
            inv::sealed_json(&attempt.join(format!("{slot}.intent.json")), &intent)?;
            let text = format!(
                "Paperhands research wake: {} / slot {slot}\nProject store: {}\nBrief: {}\nRead the Paperhands skill and current desk/tasks. Work within the operator's research and spending scope. This wake is not proof of new evidence and does not authorize orders, account actions or restarting collectors. Record evidence and the next useful question.",
                schedule.id,
                root.display(),
                schedule.message
            );
            let receipt = if inv::now() >= schedule.expires_at_ms {
                json!({"status":"expired_before_send"})
            } else {
                match icq_until(
                    &owner.icq_binary,
                    &owner.icq_home,
                    &[
                        "--as".into(),
                        owner.sender,
                        "dm".into(),
                        owner.recipient,
                        text,
                    ],
                    true,
                    Some(schedule.expires_at_ms),
                ) {
                    Ok(value) => {
                        json!({"status":"submitted_to_icq","icq":value,"receiver_acceptance":"unverified","model_delivery":"unverified"})
                    }
                    Err(e) => {
                        json!({"status":"failed_or_unknown","error":e.to_string(),"automatic_retry":false})
                    }
                }
            };
            inv::sealed_json(&attempt.join(format!("{slot}.result.json")), &receipt)?;
            sends += 1;
            row = state(&directory, &schedule, inv::now())?;
        }
        items.push(row);
    }
    Ok(
        json!({"root":root,"items":items,"wake_attempts":sends,"generated_at_ms":now,"missed_slots":"coalesced to latest; no catch-up burst","model_or_evaluator_started":false}),
    )
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
fn cron(root: &Path, directory: &Path, install: bool, dry: bool) -> Result<Value> {
    let _global_lock = if !dry {
        let parent =
            PathBuf::from(std::env::var_os("HOME").ok_or("HOME unset")?).join(".config/paperhands");
        fs::create_dir_all(&parent)?;
        let path = parent.join("cron.lock");
        if path.exists() {
            inv::digest(&path)?;
        }
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;
        fs2::FileExt::lock_exclusive(&file)?;
        Some(file)
    } else {
        None
    };
    let key = inv::hash(root.to_string_lossy().as_bytes());
    let marker = format!("paperhands:{}", &key[..24]);
    let binary = std::env::current_exe()?;
    let path = root.to_str().ok_or("non-UTF-8 store")?;
    let exe = binary.to_str().ok_or("non-UTF-8 executable")?;
    if path.contains(['%', '\n', '\r']) || exe.contains(['%', '\n', '\r']) {
        return Err("cron paths must not contain percent or newlines".into());
    }
    let crontab = executable("crontab")?;
    let previous = bounded(&crontab, &["-l".into()], None)?;
    if (previous.timed_out || previous.limited)
        || (previous.code != 0
            && !(previous.code == 1
                && previous.stdout.is_empty()
                && String::from_utf8_lossy(&previous.stderr).contains("no crontab")))
    {
        return Err("could not inspect crontab; no jobs changed".into());
    }
    let old = String::from_utf8(previous.stdout)?;
    let new = cron_text(
        &old,
        &marker,
        if install {
            Some(format!(
                "* * * * * {} automation tick --root {} >/dev/null 2>&1",
                quote(exe),
                quote(path)
            ))
        } else {
            None
        },
    )?;
    if !dry && new != old {
        // A fresh read prevents overwriting an already-observed external edit.
        let current = bounded(&crontab, &["-l".into()], None)?;
        if current.stdout != old.as_bytes() {
            return Err("crontab changed during setup; retry without overwriting it".into());
        }
        let backup = directory.join(format!("crontab-{}.backup", inv::hash(old.as_bytes())));
        if !backup.exists() {
            setup::private_create(&backup, old.as_bytes())?;
        }
        let out = bounded(&crontab, &["-".into()], Some(new.as_bytes()))?;
        if out.code != 0 || (out.timed_out || out.limited) {
            return Err("crontab installation failed or outcome unknown; inspect actual crontab before retry".into());
        }
        let actual = bounded(&crontab, &["-l".into()], None)?;
        if actual.code != 0 || actual.stdout != new.as_bytes() {
            return Err(
                "crontab verification failed; inspect retained backup and actual jobs".into(),
            );
        }
    }
    Ok(
        json!({"backend":"cron","installed":install&&!dry,"dry_run":dry,"changed":old!=new,"marker":marker,"tick_argv":[binary,"automation","tick","--root",root],"other_jobs_preserved":true,"wake_budgets":"each registered schedule must declare interval, lifetime and max wakes"}),
    )
}
fn cron_text(old: &str, marker: &str, line: Option<String>) -> Result<String> {
    let begin = format!("# BEGIN {marker}");
    let end = format!("# END {marker}");
    let mut out = Vec::new();
    let mut inside = false;
    let mut found = false;
    for row in old.lines() {
        if row == begin {
            if inside || found {
                return Err("duplicate or nested Paperhands cron marker".into());
            }
            inside = true;
            found = true;
        } else if row == end {
            if !inside {
                return Err("unmatched Paperhands cron marker".into());
            }
            inside = false;
        } else if !inside {
            out.push(row.to_owned());
        }
    }
    if inside {
        return Err("unclosed Paperhands cron marker".into());
    }
    if let Some(line) = line {
        out.extend([begin, line, end]);
    }
    Ok(if out.is_empty() {
        String::new()
    } else {
        out.join("\n") + "\n"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pipe_writes_and_owned_children_obey_the_deadline() {
        let start = Instant::now();
        let result = bounded_for(
            Path::new("/bin/sleep"),
            &["5".into()],
            Some(&vec![b'x'; 65536]),
            Duration::from_millis(30),
        )
        .unwrap();
        assert!(result.timed_out);
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    #[test]
    fn schedule_state_coalesces_missed_slots_and_honors_expiry() {
        let root = tempfile::tempdir().unwrap();
        let root = inv::root(root.path()).unwrap();
        let (directory, _lock) = store(&root).unwrap();
        let owner = Binding {
            schema_version: 1,
            root: root.clone(),
            project_id: "fixture".into(),
            icq_binary: "/bin/false".into(),
            icq_home: "/tmp/fixture-icq".into(),
            sender: "fixture-owned".into(),
            recipient: "a".repeat(64),
            created_at_ms: 0,
        };
        let sha = inv::hash(&serde_json::to_vec_pretty(&owner).unwrap());
        inv::sealed_json(
            &directory.join("bindings").join(format!("{sha}.json")),
            &owner,
        )
        .unwrap();
        let schedule = Schedule {
            schema_version: 1,
            id: "fixture".into(),
            binding_sha256: sha,
            start_at_ms: 1_000_000,
            expires_at_ms: 4_600_000,
            interval_ms: 60_000,
            max_wakes: 3,
            message: "Synthetic".into(),
        };
        fs::create_dir(directory.join("attempts/fixture")).unwrap();
        assert_eq!(
            state(&directory, &schedule, 999_999).unwrap()["status"],
            "waiting"
        );
        assert_eq!(
            state(&directory, &schedule, 1_420_000).unwrap()["due_slot"],
            7
        );
        assert_eq!(
            state(&directory, &schedule, 4_600_000).unwrap()["status"],
            "expired"
        );
    }
    #[test]
    fn cron_repair_is_idempotent_and_preserves_other_jobs() {
        let old = "MAILTO=me\n5 * * * * other-job\n";
        let first = cron_text(old, "paperhands:test", Some("* * * * * owned".into())).unwrap();
        assert!(first.starts_with(old));
        assert_eq!(
            first,
            cron_text(&first, "paperhands:test", Some("* * * * * owned".into())).unwrap()
        );
        assert_eq!(old, cron_text(&first, "paperhands:test", None).unwrap());
        assert!(cron_text("# BEGIN paperhands:test\n", "paperhands:test", None).is_err());
    }
}
