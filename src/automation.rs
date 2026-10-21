//! Bounded wake schedules. ICQ owns native delivery; Paperhands never guesses sockets.
use crate::{Result, research::inventory as inv, setup};
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
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
            .arg(Arg::new("report-out").long("report-out").help("Output for an allowlisted report refreshed by non-dry ticks for due/waiting schedules at most hourly; no model starts"))
            .arg(Arg::new("zoom-prompt").long("zoom-prompt").help("Freeze an exact UTF-8 premise-review brief (at most 8 KiB); substitute it within an already-due wake only when zoom reports stalled"))
            .arg(Arg::new("message").long("message").required(true).help("Operator-authorized research brief; literal text, at most 8 KiB")))
        .subcommand(Command::new("cancel").about("Retain a cancellation decision; stop future wakes without deleting evidence").arg(root()).arg(id()).arg(Arg::new("reason").long("reason").required(true)))
        .subcommand(Command::new("status").about("Read prepared schedules, claims and wake receipts under a shared lock; never initialize automation state").arg(root()))
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

#[derive(Debug)]
pub(crate) struct CommandFailure {
    message: String,
    pub diagnostics: Value,
}
impl std::fmt::Display for CommandFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for CommandFailure {}

fn command_failure(
    context: &str,
    operation: &str,
    phase: &str,
    out: &Output,
    icq_json: bool,
) -> Box<dyn std::error::Error> {
    // Local diagnostics omit argv/stdout and ICQ navigation payloads.
    let raw = String::from_utf8_lossy(&out.stderr);
    let reason = if icq_json {
        match serde_json::from_str::<Value>(&raw) {
            Ok(value) => value["error"]
                .as_str()
                .or_else(|| value["error"]["message"].as_str())
                .unwrap_or("")
                .to_owned(),
            Err(_) => raw.into_owned(),
        }
    } else {
        raw.into_owned()
    };
    let mut chars = reason.chars().map(|c| if c.is_control() { ' ' } else { c });
    let stderr = chars.by_ref().take(2048).collect::<String>();
    let truncated = chars.next().is_some();
    let diagnostics = json!({
        "operation":operation,"phase":phase,"exit_code":out.code,
        "timed_out":out.timed_out,"output_limited":out.limited,
        "stdout_bytes_retained":out.stdout.len(),"stderr_bytes_retained":out.stderr.len(),
        "stderr":stderr,"stderr_truncated":truncated
    });
    Box::new(CommandFailure {
        message: format!(
            "{context} ({operation}/{phase}: exit {}, timed_out={}, output_limited={}){}{}",
            out.code,
            out.timed_out,
            out.limited,
            if stderr.is_empty() { "" } else { ": " },
            stderr
        ),
        diagnostics,
    })
}

fn cron_readable(out: &Output) -> bool {
    !out.timed_out
        && !out.limited
        && (out.code == 0
            || (out.code == 1
                && out.stdout.is_empty()
                && String::from_utf8_lossy(&out.stderr).contains("no crontab")))
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
    let operation = match args.first().map(String::as_str) {
        Some("identity") => "icq.identity.create",
        Some("id") => "icq.id",
        Some("--as") if args.get(2).is_some_and(|arg| arg == "id") => "icq.id",
        Some("--as") if args.get(2).is_some_and(|arg| arg == "dm") => "icq.dm",
        _ => "icq",
    };
    let mut command = vec!["--home".into(), home.to_string_lossy().into_owned()];
    if mutation {
        let mut preview = command.clone();
        preview.push("--dry-run".into());
        preview.extend_from_slice(args);
        let out = bounded(binary, &preview, None)?;
        if out.code != 0 || (out.timed_out || out.limited) {
            return Err(command_failure(
                "ICQ preview refused; no messaging action attempted",
                operation,
                "preview",
                &out,
                true,
            ));
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
        return Err(command_failure(
            "ICQ outcome unknown after command deadline or output limit; not retried",
            operation,
            "execute",
            &out,
            true,
        ));
    }
    if out.code != 0 {
        return Err(command_failure(
            "ICQ command failed; not retried",
            operation,
            "execute",
            &out,
            true,
        ));
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    report_out: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    zoom_prompt: Option<String>,
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
fn read_store(root: &Path) -> Result<(PathBuf, fs::File)> {
    let directory = inv::root(&inv::checked(root, "automation")?)?;
    let path = inv::checked(&directory, "automation.lock")?;
    inv::digest(&path)?;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path)?;
    fs2::FileExt::try_lock_shared(&file)
        .map_err(|_| "automation is busy; another owned tick/bind is running")?;
    for name in ["schedules", "attempts", "cancellations", "bindings"] {
        inv::root(&inv::checked(&directory, name)?)?;
    }
    Ok((directory, file))
}

/// Direct zoom readers share the same consistency lock as status. Tick uses the
/// internal collector while holding its existing exclusive lock.
pub(crate) fn zoom_history(root: &Path) -> Result<Value> {
    let root = inv::root(root)?;
    match fs::symlink_metadata(root.join("automation")) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(json!({"complete":true,"wakes":[],"gaps":[]}))
        }
        Err(error) => Err(error.into()),
        Ok(_) => {
            let (directory, _lock) = read_store(&root)?;
            zoom_history_locked(&root, &directory)
        }
    }
}

fn zoom_history_locked(root: &Path, directory: &Path) -> Result<Value> {
    let declared = zoom_record_ids(directory, "schedules")?;
    let cancellations = zoom_record_ids(directory, "cancellations")?;
    if !cancellations.is_subset(&declared) {
        return Err("zoom cancellation references an undeclared schedule".into());
    }
    let attempts_root = inv::root(&inv::checked(directory, "attempts")?)?;
    let mut attempt_ids = BTreeSet::new();
    for entry in fs::read_dir(&attempts_root)? {
        let entry = entry?;
        let id = entry
            .file_name()
            .into_string()
            .map_err(|_| "invalid attempt directory")?;
        inv::name(&id)?;
        inv::root(&entry_path(&attempts_root, &id)?)?;
        attempt_ids.insert(id);
        if attempt_ids.len() > 128 {
            return Err("zoom automation exceeds 128 attempt directories".into());
        }
    }
    if attempt_ids != declared {
        return Err("zoom attempt directories do not match retained schedules".into());
    }
    let mut wakes = Vec::new();
    let mut members = 0usize;
    for schedule in schedules(directory)? {
        binding_for(directory, root, &schedule.binding_sha256)?;
        let schedule_sha256 = inv::digest(
            &directory
                .join("schedules")
                .join(format!("{}.json", schedule.id)),
        )?;
        let attempts = inv::root(&inv::checked(
            directory,
            &format!("attempts/{}", schedule.id),
        )?)?;
        let mut intents = BTreeMap::new();
        let mut results = BTreeMap::new();
        for entry in fs::read_dir(&attempts)? {
            let entry = entry?;
            members += 1;
            if members > 4096 * 4 {
                return Err("zoom automation history exceeds 16384 retained members".into());
            }
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "invalid wake member")?;
            if let Some(stem) = name.strip_suffix(".sha256") {
                let slot = stem
                    .strip_suffix(".intent")
                    .or_else(|| stem.strip_suffix(".result"))
                    .ok_or("unexpected wake checksum member")?;
                let number = slot.parse::<u64>()?;
                if slot != number.to_string() {
                    return Err("noncanonical wake slot in zoom history".into());
                }
                // A leftover seal is evidence of an incomplete history, never
                // proof that no attempt happened.
                inv::digest(&entry_path(&attempts, &format!("{stem}.json"))?)?;
                continue;
            }
            let (slot, target) = if let Some(slot) = name.strip_suffix(".intent.json") {
                (slot, &mut intents)
            } else if let Some(slot) = name.strip_suffix(".result.json") {
                (slot, &mut results)
            } else {
                return Err("unexpected wake evidence member".into());
            };
            let number = slot.parse::<u64>()?;
            if slot != number.to_string() {
                return Err("noncanonical wake slot in zoom history".into());
            }
            let value: Value = inv::read_sealed(&entry_path(&attempts, &name)?)?;
            target.insert(number, value);
        }
        if results.keys().any(|slot| !intents.contains_key(slot)) {
            return Err("wake result has no retained intent".into());
        }
        for (slot, intent) in intents {
            if wakes.len() >= 4096 {
                return Err("zoom automation history exceeds 4096 wake intents".into());
            }
            if intent["slot"] != slot || intent["schedule_sha256"] != schedule_sha256 {
                return Err("wake intent binding changed".into());
            }
            let claimed_at_ms = intent["claimed_at_ms"]
                .as_u64()
                .ok_or("wake intent has no valid claim clock")?;
            wakes.push(json!({"id":format!("{}/{slot}",schedule.id),"schedule_sha256":schedule_sha256,"slot":slot,"claimed_at_ms":claimed_at_ms,"result":results.remove(&slot)}));
        }
    }
    Ok(json!({"complete":true,"wakes":wakes,"gaps":[]}))
}

fn zoom_record_ids(directory: &Path, collection: &str) -> Result<BTreeSet<String>> {
    let path = inv::root(&inv::checked(directory, collection)?)?;
    let mut json_ids = BTreeSet::new();
    let mut seal_ids = BTreeSet::new();
    let mut members = 0;
    for entry in fs::read_dir(&path)? {
        let entry = entry?;
        members += 1;
        if members > 256 {
            return Err("zoom automation record collection exceeds 256 members".into());
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "invalid automation record")?;
        let (id, target) = if let Some(id) = name.strip_suffix(".json") {
            (id, &mut json_ids)
        } else if let Some(id) = name.strip_suffix(".sha256") {
            (id, &mut seal_ids)
        } else {
            return Err("unexpected automation record member".into());
        };
        inv::name(id)?;
        inv::digest(&entry_path(&path, &name)?)?;
        target.insert(id.to_owned());
    }
    if json_ids != seal_ids {
        return Err("zoom automation record has an orphan JSON or SHA member".into());
    }
    for id in &json_ids {
        let _: Value = inv::read_sealed(&path.join(format!("{id}.json")))?;
    }
    Ok(json_ids)
}

fn inspect_zoom_locked(root: &Path, directory: &Path) -> Value {
    match zoom_history_locked(root, directory).and_then(|history| {
        crate::zoom::inspect_with_automation(
            root,
            None,
            &crate::zoom::Thresholds::default(),
            &history,
        )
    }) {
        Ok(mut value) => {
            if value["verdict"].is_null() && value.get("error").is_none() {
                let reasons = value["limitations"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("; ");
                value["error"] = json!(format!(
                    "zoom verdict is unknown; routine wake retained. {reasons}"
                ));
            }
            value
        }
        Err(error) => {
            json!({"schema":"paperhands.zoom/v1","generated_at_ms":inv::now(),"verdict":null,"signals_fired":[],"error":error.to_string()})
        }
    }
}

fn zoom_receipt(zoom: &Value) -> Value {
    let mut summary = json!({"verdict":zoom["verdict"],"signals_fired":zoom["signals_fired"]});
    if let Some(error) = zoom.get("error") {
        summary["error"] = error.clone();
    }
    summary
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
                || value
                    .report_out
                    .as_ref()
                    .is_some_and(|out| !out.is_absolute())
                || value.zoom_prompt.as_ref().is_some_and(|brief| {
                    brief.trim().is_empty() || brief.len() > 8192 || brief.contains('\0')
                })
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
    let remaining_attempts = schedule.max_wakes.saturating_sub(claimed.len() as u64);
    let valid_slots =
        (schedule.expires_at_ms - schedule.start_at_ms).div_ceil(schedule.interval_ms);
    let prior_slots = slot.min(valid_slots);
    let claimed_prior = claimed
        .iter()
        .filter(|&&number| number < prior_slots)
        .collect::<BTreeSet<_>>()
        .len() as u64;
    let missed_slots = prior_slots.saturating_sub(claimed_prior);
    let (next_due_at_ms, next_due_reason) = match status {
        "cancelled" | "expired" | "budget_exhausted" => (None, status),
        _ => {
            // Coalesce to the current slot, then skip every already-spent slot.
            let mut next_slot = slot;
            while claimed.binary_search(&next_slot).is_ok() {
                next_slot = next_slot.checked_add(1).ok_or("wake slot overflow")?;
            }
            let next = next_slot
                .checked_mul(schedule.interval_ms)
                .and_then(|offset| schedule.start_at_ms.checked_add(offset));
            match next.filter(|due| *due < schedule.expires_at_ms) {
                Some(due) => (Some(due), if due > now { "waiting" } else { "due" }),
                None => (None, "expires_before_next_due"),
            }
        }
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
        json!({"id":schedule.id,"status":status,"due_slot":slot,"missed_slots":missed_slots,"attempted_wakes":claimed.len(),"max_wakes":schedule.max_wakes,"remaining_attempts":remaining_attempts,"next_due_at_ms":next_due_at_ms,"next_due_reason":next_due_reason,"expires_at_ms":schedule.expires_at_ms,"interval_ms":schedule.interval_ms,"last_receipt":last}),
    )
}
fn entry_path(root: &Path, name: &str) -> Result<PathBuf> {
    inv::checked(root, name)
}

fn report_path(value: &Path) -> Result<PathBuf> {
    use std::path::Component;
    let absolute = if value.is_absolute() {
        value.to_owned()
    } else {
        std::env::current_dir()?.join(value)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err("report output escapes filesystem root".into());
                }
            }
            part => normalized.push(part),
        }
    }
    Ok(normalized)
}

fn frozen_zoom_prompt(path: &Path) -> Result<String> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 8192 {
        return Err("zoom prompt must be a regular UTF-8 file of at most 8 KiB, not a link".into());
    }
    let text = String::from_utf8(inv::read_bytes(path)?)?;
    if text.trim().is_empty() || text.len() > 8192 || text.contains('\0') {
        return Err("zoom prompt must contain 1..8192 UTF-8 bytes without NUL".into());
    }
    Ok(text)
}

fn report_decision(out: &Path, now: u64) -> Result<(&'static str, Option<u64>)> {
    let path = out.join("report.json");
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(("missing", None)),
        Err(error) => Err(error.into()),
        Ok(_) => {
            let report: Value = inv::read(&path)?;
            if report["schema"] != "paperhands.report/v1" {
                return Err("existing report has an unsupported schema".into());
            }
            let generated = report["generated_at_ms"]
                .as_u64()
                .ok_or("existing report has no valid generated_at_ms")?;
            if generated > now {
                return Err("existing report generation time is in the future".into());
            }
            Ok((
                if now - generated > 3_600_000 {
                    "stale"
                } else {
                    "fresh"
                },
                Some(generated),
            ))
        }
    }
}

fn refresh_reports(
    root: &Path,
    directory: &Path,
    outputs: BTreeSet<PathBuf>,
    automation: &Value,
    zoom: &Value,
) -> Result<Vec<Value>> {
    let mut receipts = Vec::new();
    let reports = inv::subdirectory(directory, "reports")?;
    for (index, out) in outputs.into_iter().enumerate() {
        let checked_at_ms = inv::now();
        let mut previous_generated_at_ms = None;
        let mut generated_at_ms = None;
        let mut error = None;
        let (report, reason) = match report_decision(&out, checked_at_ms) {
            Ok(("fresh", generated)) => {
                previous_generated_at_ms = generated;
                generated_at_ms = generated;
                ("skipped", "fresh")
            }
            Ok((reason, generated)) => {
                previous_generated_at_ms = generated;
                match crate::report::write_with_automation(root, &out, automation, zoom) {
                    Ok(value) => {
                        generated_at_ms = value["generated_at_ms"].as_u64();
                        ("written", reason)
                    }
                    Err(failure) => {
                        error = Some(failure.to_string());
                        ("failed", "write_failed")
                    }
                }
            }
            Err(failure) => {
                error = Some(failure.to_string());
                ("failed", "existing_report_invalid")
            }
        };
        let recorded_at_ms = inv::now();
        let value = json!({"schema":"paperhands.automation-report/v1","recorded_at_ms":recorded_at_ms,"generated_at_ms":generated_at_ms,"report_out":out,"report":report,"reason":reason,"previous_generated_at_ms":previous_generated_at_ms,"error":error,"model_or_evaluator_started":false});
        if report == "skipped" {
            let path = reports.join("latest-skip.json");
            let old = crate::setup::optional(&path)?;
            let bytes = serde_json::to_vec_pretty(&value)?;
            crate::setup::replace(&path, old.as_deref(), &bytes, None)?;
            receipts.push(
                json!({"report":report,"reason":reason,"latest_skip_sha256":inv::hash(&bytes)}),
            );
            continue;
        }
        let mut suffix = index;
        let path = loop {
            let candidate = reports.join(format!(
                "{recorded_at_ms}-{}-{suffix}.json",
                std::process::id()
            ));
            if !candidate.exists() && !candidate.with_extension("sha256").exists() {
                break candidate;
            }
            suffix = suffix
                .checked_add(1)
                .ok_or("report receipt identity exhausted")?;
        };
        let sha = inv::sealed_json(&path, &value)?;
        receipts.push(json!({"report":report,"reason":reason,"receipt_sha256":sha}));
    }
    Ok(receipts)
}

pub fn execute(args: &ArgMatches) -> Result<Value> {
    let (action, args) = args.subcommand().ok_or("choose an automation operation")?;
    let root = inv::root(&crate::project::resolve(
        args.get_one::<String>("root").map(String::as_str),
    )?)?;
    let (directory, _lock) = if action == "status" {
        read_store(&root)?
    } else {
        store(&root)?
    };
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
            report_out: args
                .get_one::<String>("report-out")
                .map(|value| report_path(Path::new(value)))
                .transpose()?,
            zoom_prompt: args
                .get_one::<String>("zoom-prompt")
                .map(|value| frozen_zoom_prompt(Path::new(value)))
                .transpose()?,
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
    let report_outputs = if action == "tick" && !args.get_flag("dry-run") {
        pending
            .iter()
            .filter(|(_, row)| matches!(row["status"].as_str(), Some("due" | "waiting")))
            .filter_map(|(schedule, _)| schedule.report_out.as_deref())
            .map(report_path)
            .collect::<Result<BTreeSet<_>>>()?
    } else {
        BTreeSet::new()
    };
    let zoom = if action == "tick"
        && !args.get_flag("dry-run")
        && (!report_outputs.is_empty()
            || pending
                .iter()
                .any(|(schedule, row)| row["status"] == "due" && schedule.zoom_prompt.is_some()))
    {
        Some(inspect_zoom_locked(&root, &directory))
    } else {
        None
    };
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
            let wake_zoom = schedule.zoom_prompt.as_ref().and(zoom.as_ref());
            let brief = if wake_zoom.is_some_and(|value| value["verdict"] == "stalled") {
                schedule.zoom_prompt.as_deref().unwrap()
            } else {
                &schedule.message
            };
            let mut intent = json!({"schedule_sha256":inv::digest(&directory.join("schedules").join(format!("{}.json",schedule.id)))?,"slot":slot,"claimed_at_ms":inv::now(),"automatic_retry":false});
            if let Some(zoom) = wake_zoom {
                intent["zoom"] = zoom_receipt(zoom);
            }
            inv::sealed_json(&attempt.join(format!("{slot}.intent.json")), &intent)?;
            let text = format!(
                "Paperhands research wake: {} / slot {slot}\nProject store: {}\nBrief: {}\nRead the Paperhands skill and current desk/tasks. Work within the operator's research and spending scope. This wake is not proof of new evidence and does not authorize orders, account actions or restarting collectors. Record evidence and the next useful question.",
                schedule.id,
                root.display(),
                brief
            );
            let mut receipt = if inv::now() >= schedule.expires_at_ms {
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
                        json!({"status":"failed_or_unknown","error":e.to_string(),"diagnostics":e.downcast_ref::<CommandFailure>().map(|failure|&failure.diagnostics),"automatic_retry":false})
                    }
                }
            };
            if let Some(zoom) = wake_zoom {
                receipt["zoom"] = zoom_receipt(zoom);
            }
            inv::sealed_json(&attempt.join(format!("{slot}.result.json")), &receipt)?;
            sends += 1;
            row = state(&directory, &schedule, inv::now())?;
        }
        items.push(row);
    }
    let mut result = json!({"root":root,"items":items,"wake_attempts":sends,"generated_at_ms":now,"missed_slots":"coalesced to latest; no catch-up burst","model_or_evaluator_started":false});
    if !report_outputs.is_empty() {
        result["reports"] = refresh_reports(
            &root,
            &directory,
            report_outputs,
            &result,
            zoom.as_ref().unwrap(),
        )?
        .into();
    }
    Ok(result)
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
    if !cron_readable(&previous) {
        return Err(command_failure(
            "could not inspect crontab; no jobs changed",
            "crontab",
            "read",
            &previous,
            false,
        ));
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
        if !cron_readable(&current) {
            return Err(command_failure(
                "could not recheck crontab; no jobs changed",
                "crontab",
                "recheck",
                &current,
                false,
            ));
        }
        if current.stdout != old.as_bytes() {
            return Err("crontab changed during setup; retry without overwriting it".into());
        }
        let backup = directory.join(format!("crontab-{}.backup", inv::hash(old.as_bytes())));
        if !backup.exists() {
            setup::private_create(&backup, old.as_bytes())?;
        }
        let out = bounded(&crontab, &["-".into()], Some(new.as_bytes()))?;
        if out.code != 0 || (out.timed_out || out.limited) {
            return Err(command_failure(
                "crontab installation failed or outcome unknown; inspect actual crontab before retry",
                "crontab",
                "write",
                &out,
                false,
            ));
        }
        let actual = bounded(&crontab, &["-l".into()], None)?;
        if actual.code != 0 || actual.timed_out || actual.limited || actual.stdout != new.as_bytes()
        {
            return Err(command_failure(
                "crontab verification failed; inspect retained backup and actual jobs",
                "crontab",
                "verify",
                &actual,
                false,
            ));
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
    fn child_diagnostics_are_bounded_and_keep_unknown_outcomes_explicit() {
        let output = Output {
            code: 7,
            stdout: b"PRIVATE_STDOUT_MUST_NOT_LEAK".to_vec(),
            stderr: serde_json::to_vec(&json!({
                "error":{"message":format!("identity capacity reached\n{}", "x".repeat(3000))},
                "next":[{"command":"PRIVATE_GUIDANCE_MUST_NOT_LEAK"}]
            }))
            .unwrap(),
            timed_out: true,
            limited: true,
        };
        let error = command_failure(
            "unknown; not retried",
            "icq.identity.create",
            "execute",
            &output,
            true,
        );
        let details = &error.downcast_ref::<CommandFailure>().unwrap().diagnostics;
        assert_eq!(details["timed_out"], true);
        assert_eq!(details["output_limited"], true);
        assert_eq!(details["stderr_truncated"], true);
        let stderr = details["stderr"].as_str().unwrap();
        assert_eq!(stderr.chars().count(), 2048);
        assert!(!stderr.chars().any(char::is_control));
        assert!(!details.to_string().contains("MUST_NOT_LEAK"));
    }
    #[test]
    fn cron_rechecks_refuse_failed_or_overdue_reads_even_with_empty_stdout() {
        let mut output = Output {
            code: 0,
            stdout: vec![],
            stderr: vec![],
            timed_out: false,
            limited: false,
        };
        assert!(cron_readable(&output));
        output.timed_out = true;
        assert!(!cron_readable(&output));
        output.timed_out = false;
        output.limited = true;
        assert!(!cron_readable(&output));
        output.limited = false;
        output.code = 1;
        assert!(!cron_readable(&output));
        output.stderr = b"no crontab for fixture".to_vec();
        assert!(cron_readable(&output));
        output.code = 7;
        assert!(!cron_readable(&output));
    }
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
    fn schedule_navigation_preserves_spent_slots_coalescing_and_terminal_states() {
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
            report_out: None,
            zoom_prompt: None,
        };
        let retain = |schedule: &Schedule| {
            fs::create_dir(directory.join("attempts").join(&schedule.id)).unwrap();
            inv::sealed_json(
                &directory
                    .join("schedules")
                    .join(format!("{}.json", schedule.id)),
                schedule,
            )
            .unwrap();
        };
        let claim = |schedule: &Schedule, slot: u64| {
            let schedule_sha256 = inv::digest(
                &directory
                    .join("schedules")
                    .join(format!("{}.json", schedule.id)),
            )
            .unwrap();
            inv::sealed_json(
                &directory
                    .join("attempts")
                    .join(&schedule.id)
                    .join(format!("{slot}.intent.json")),
                &json!({"slot":slot,"schedule_sha256":schedule_sha256,"automatic_retry":false}),
            )
            .unwrap();
        };
        retain(&schedule);
        let before = state(&directory, &schedule, 999_999).unwrap();
        assert_eq!(before["status"], "waiting");
        assert_eq!(before["next_due_at_ms"], 1_000_000);
        assert_eq!(before["remaining_attempts"], 3);
        assert_eq!(before["next_due_reason"], "waiting");
        assert_eq!(before["missed_slots"], 0);
        claim(&schedule, 0);
        let waiting = state(&directory, &schedule, 1_000_001).unwrap();
        assert_eq!(waiting["next_due_at_ms"], 1_060_000);
        assert_eq!(waiting["remaining_attempts"], 2);
        assert_eq!(waiting["last_receipt"]["status"], "interrupted_or_unknown");
        assert_eq!(waiting["missed_slots"], 0);
        // Clock rollback cannot make an already-claimed future slot eligible again.
        claim(&schedule, 1);
        assert_eq!(
            state(&directory, &schedule, 1_000_001).unwrap()["next_due_at_ms"],
            1_120_000
        );
        let due = state(&directory, &schedule, 1_420_001).unwrap();
        assert_eq!(due["due_slot"], 7);
        assert_eq!(due["next_due_at_ms"], 1_420_000);
        assert_eq!(due["next_due_reason"], "due");
        assert_eq!(due["missed_slots"], 5);
        let expired = state(&directory, &schedule, 4_600_000).unwrap();
        assert_eq!(expired["status"], "expired");
        assert_eq!(expired["next_due_at_ms"], Value::Null);
        assert_eq!(expired["next_due_reason"], "expired");
        assert_eq!(expired["remaining_attempts"], 1);
        assert_eq!(expired["missed_slots"], 58);
        assert_eq!(
            state(&directory, &schedule, 9_999_999).unwrap()["missed_slots"],
            58
        );
        let exhausted = Schedule {
            id: "exhausted".into(),
            max_wakes: 1,
            ..schedule.clone()
        };
        retain(&exhausted);
        claim(&exhausted, 0);
        let row = state(&directory, &exhausted, 1_000_001).unwrap();
        assert_eq!(row["status"], "budget_exhausted");
        assert_eq!(row["remaining_attempts"], 0);
        assert_eq!(row["next_due_at_ms"], Value::Null);
        assert_eq!(row["next_due_reason"], "budget_exhausted");
        for (id, expires_at_ms) in [("short", 1_060_000), ("equal-expiry", 1_120_000)] {
            let short = Schedule {
                id: id.into(),
                interval_ms: 120_000,
                expires_at_ms,
                ..schedule.clone()
            };
            retain(&short);
            claim(&short, 0);
            let row = state(&directory, &short, 1_000_001).unwrap();
            assert_eq!(row["status"], "waiting");
            assert_eq!(row["next_due_at_ms"], Value::Null);
            assert_eq!(row["next_due_reason"], "expires_before_next_due");
        }
        let cancelled = Schedule {
            id: "cancelled".into(),
            ..schedule
        };
        retain(&cancelled);
        inv::sealed_json(
            &directory.join("cancellations/cancelled.json"),
            &json!({"reason":"fixture cancelled"}),
        )
        .unwrap();
        let row = state(&directory, &cancelled, 1_000_001).unwrap();
        assert_eq!(row["remaining_attempts"], 3);
        assert_eq!(row["next_due_at_ms"], Value::Null);
        assert_eq!(row["next_due_reason"], "cancelled");
    }
    #[test]
    fn optional_report_configuration_preserves_legacy_schedule_shape_and_finite_freshness() {
        let legacy = json!({"schema_version":1,"id":"legacy","binding_sha256":"a".repeat(64),"start_at_ms":1,"expires_at_ms":60001,"interval_ms":60000,"max_wakes":1,"message":"private"});
        let schedule: Schedule = serde_json::from_value(legacy.clone()).unwrap();
        assert!(schedule.report_out.is_none());
        assert!(schedule.zoom_prompt.is_none());
        assert_eq!(serde_json::to_value(schedule).unwrap(), legacy);
        let out = tempfile::tempdir().unwrap();
        assert_eq!(
            report_decision(out.path(), 5_000_000).unwrap(),
            ("missing", None)
        );
        let path = out.path().join("report.json");
        let write = |value: Value| fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        write(json!({"schema":"paperhands.report/v1","generated_at_ms":1_400_000}));
        assert_eq!(
            report_decision(out.path(), 5_000_000).unwrap(),
            ("fresh", Some(1_400_000))
        );
        assert_eq!(
            report_decision(out.path(), 5_000_001).unwrap(),
            ("stale", Some(1_400_000))
        );
        write(json!({"schema":"paperhands.report/v1","generated_at_ms":5_000_001}));
        assert!(report_decision(out.path(), 5_000_000).is_err());
        write(json!({"schema":"paperhands.report/v1","generated_at_ms":"unknown"}));
        assert!(report_decision(out.path(), 5_000_000).is_err());
        write(json!({"schema":"unknown-report/v1","generated_at_ms":1}));
        assert!(report_decision(out.path(), 5_000_000).is_err());
    }
    #[test]
    fn zoom_history_verifies_ambiguous_claims_under_the_existing_read_only_lock() {
        let temporary = tempfile::tempdir().unwrap();
        let root = inv::root(temporary.path()).unwrap();
        assert_eq!(
            zoom_history(&root).unwrap(),
            json!({"complete":true,"wakes":[],"gaps":[]})
        );
        assert!(!root.join("automation").exists());
        let (directory, lock) = store(&root).unwrap();
        let owner = Binding {
            schema_version: 1,
            root: root.clone(),
            project_id: "fixture".into(),
            icq_binary: "/bin/false".into(),
            icq_home: "/tmp/owned-no-send".into(),
            sender: "fixture".into(),
            recipient: "a".repeat(64),
            created_at_ms: 1,
        };
        let binding = inv::hash(&serde_json::to_vec_pretty(&owner).unwrap());
        inv::sealed_json(
            &directory.join("bindings").join(format!("{binding}.json")),
            &owner,
        )
        .unwrap();
        let schedule = Schedule {
            schema_version: 1,
            id: "fixture".into(),
            binding_sha256: binding,
            start_at_ms: 1,
            expires_at_ms: 3_600_001,
            interval_ms: 60_000,
            max_wakes: 3,
            message: "Routine".into(),
            report_out: None,
            zoom_prompt: None,
        };
        let sha = inv::sealed_json(&directory.join("schedules/fixture.json"), &schedule).unwrap();
        fs::create_dir(directory.join("attempts/fixture")).unwrap();
        let path = directory.join("attempts/fixture/0.intent.json");
        inv::sealed_json(
            &path,
            &json!({"slot":0,"schedule_sha256":sha,"claimed_at_ms":2,"automatic_retry":false}),
        )
        .unwrap();
        let bytes = fs::read(&path).unwrap();
        let snapshot = zoom_history_locked(&root, &directory).unwrap();
        assert_eq!(snapshot["wakes"].as_array().unwrap().len(), 1);
        assert_eq!(snapshot["wakes"][0]["claimed_at_ms"], 2);
        assert_eq!(snapshot["wakes"][0]["result"], Value::Null);
        assert!(zoom_history(&root).is_err());
        drop(lock);
        assert_eq!(zoom_history(&root).unwrap(), snapshot);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        let attempts = directory.join("attempts/fixture");
        let displaced = root.join("owned-displaced-attempts");
        fs::rename(&attempts, &displaced).unwrap();
        assert!(zoom_history(&root).is_err());
        assert!(!attempts.exists());
        fs::rename(displaced, attempts).unwrap();
        fs::write(&path, b"{}").unwrap();
        assert!(zoom_history(&root).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"{}");
        fs::write(&path, &bytes).unwrap();
        let seal = path.with_extension("sha256");
        let seal_bytes = fs::read(&seal).unwrap();
        fs::remove_file(&path).unwrap();
        assert!(zoom_history(&root).is_err());
        assert!(!path.exists());
        assert_eq!(fs::read(&seal).unwrap(), seal_bytes);
    }
    #[test]
    fn zoom_history_refuses_orphan_schedules_and_unregistered_control_members_without_repair() {
        let temporary = tempfile::tempdir().unwrap();
        let root = inv::root(temporary.path()).unwrap();
        let (directory, lock) = store(&root).unwrap();
        drop(lock);
        let reservation = directory.join("attempts/reserved");
        fs::create_dir(&reservation).unwrap();
        let orphan = directory.join("schedules/reserved.sha256");
        let seal = "a".repeat(64);
        fs::write(&orphan, &seal).unwrap();
        let error = zoom_history(&root).unwrap_err().to_string();
        assert!(error.contains("orphan JSON or SHA"), "{error}");
        assert_eq!(fs::read_to_string(&orphan).unwrap(), seal);
        assert!(!directory.join("schedules/reserved.json").exists());
        assert!(reservation.is_dir());
        fs::remove_file(&orphan).unwrap();
        let error = zoom_history(&root).unwrap_err().to_string();
        assert!(
            error.contains("attempt directories do not match"),
            "{error}"
        );
        assert!(reservation.is_dir());
        assert_eq!(fs::read_dir(&reservation).unwrap().count(), 0);
        fs::remove_dir(&reservation).unwrap();
        let cancellation = directory.join("cancellations/undeclared.json");
        inv::sealed_json(
            &cancellation,
            &json!({"reason":"owned undeclared cancellation"}),
        )
        .unwrap();
        let bytes = fs::read(&cancellation).unwrap();
        let error = zoom_history(&root).unwrap_err().to_string();
        assert!(
            error.contains("cancellation references an undeclared"),
            "{error}"
        );
        assert_eq!(fs::read(&cancellation).unwrap(), bytes);
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
