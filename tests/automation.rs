use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

fn run(home: &Path, args: &[&str]) -> (i32, Value) {
    let output = command(home).args(args).output().unwrap();
    let bytes = if output.stdout.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    };
    (
        output.status.code().unwrap(),
        serde_json::from_slice(bytes)
            .unwrap_or_else(|_| panic!("{args:?}: {}", String::from_utf8_lossy(bytes))),
    )
}
fn command(home: &Path) -> Command {
    command_at(home, env!("CARGO_BIN_EXE_paperhands"))
}
fn command_at(home: &Path, program: &str) -> Command {
    let mut command = Command::new(program);
    command
        .env("HOME", home)
        .env_remove("CODEX_HOME")
        .env_remove("CLAUDE_CONFIG_DIR")
        .env_remove("ICQ_HOME")
        .env("PAPERHANDS_SKILLS_DIR", home.join("isolated-skill"))
        .env(
            "PATH",
            format!(
                "{}:{}",
                home.join("bin").display(),
                std::env::var("PATH").unwrap()
            ),
        )
        .env("WAKE_FIXTURE", home)
        .env_remove("PAPERHANDS_WORKSPACE");
    command
}
#[test]
fn native_setup_preserves_settings_comments_and_private_backups_and_links() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    fs::create_dir_all(h.join(".codex")).unwrap();
    let original =
        b"# personal comment\nmodel = 'keep-me'\n[mcp_servers.other]\ncommand = 'other'\n";
    fs::write(h.join(".codex/config.toml"), original).unwrap();
    fs::write(
        h.join(".claude.json"),
        br#"{"private":"keep","mcpServers":{"other":{"command":"other"}}}"#,
    )
    .unwrap();
    let home_arg = h.to_str().unwrap();
    let (code, preview) = run(h, &["setup", "--home", home_arg, "--dry-run"]);
    assert_eq!(code, 0, "{preview}");
    assert_eq!(fs::read(h.join(".codex/config.toml")).unwrap(), original);
    assert!(!h.join(".config/paperhands/setup-backups").exists());
    let (code, configured) = run(h, &["setup", "--home", home_arg]);
    assert_eq!(code, 0, "{configured}");
    let codex = fs::read_to_string(h.join(".codex/config.toml")).unwrap();
    assert!(codex.contains("# personal comment"));
    let parsed: toml::Value = toml::from_str(&codex).unwrap();
    assert_eq!(parsed["model"].as_str(), Some("keep-me"));
    assert_eq!(
        parsed["mcp_servers"]["other"]["command"].as_str(),
        Some("other")
    );
    let claude: Value = serde_json::from_slice(&fs::read(h.join(".claude.json")).unwrap()).unwrap();
    assert_eq!(claude["private"], "keep");
    assert_eq!(claude["mcpServers"]["paperhands"]["args"], json!(["mcp"]));
    assert_eq!(
        fs::read_to_string(h.join(".codex/skills/paperhands/SKILL.md")).unwrap(),
        paperhands::SKILL
    );
    assert_eq!(
        h.join(".claude/skills/paperhands").canonicalize().unwrap(),
        h.join(".agents/skills/paperhands").canonicalize().unwrap()
    );
    let (code, second) = run(h, &["setup", "--home", home_arg]);
    assert_eq!(code, 0, "{second}");
    assert!(
        second["data"]["configurations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["changed"] == false)
    );
    let backup = h.join(".config/paperhands/setup-backups").join(format!(
        "{}.backup",
        paperhands::research::inventory::hash(original)
    ));
    assert_eq!(fs::read(backup).unwrap(), original);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(h.join(".claude.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o077,
            0
        );
    }
}
#[test]
fn malformed_or_linked_settings_refuse_without_overwriting_other_config() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    fs::create_dir(h.join(".codex")).unwrap();
    fs::write(h.join(".codex/config.toml"), "model='untouched'\n").unwrap();
    fs::write(h.join(".claude.json"), "malformed").unwrap();
    assert_eq!(run(h, &["setup", "--home", h.to_str().unwrap()]).0, 2);
    assert_eq!(
        fs::read_to_string(h.join(".codex/config.toml")).unwrap(),
        "model='untouched'\n"
    );
    #[cfg(unix)]
    {
        fs::remove_file(h.join(".claude.json")).unwrap();
        fs::write(h.join("outside"), "{}").unwrap();
        std::os::unix::fs::symlink(h.join("outside"), h.join(".claude.json")).unwrap();
        assert_eq!(run(h, &["setup", "--home", h.to_str().unwrap()]).0, 2);
        assert_eq!(fs::read_to_string(h.join("outside")).unwrap(), "{}");
    }
}
#[test]
fn research_bootstrap_retains_direction_task_decisions_and_project_mcp_scope() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    let project = h.join("project");
    let p = project.to_str().unwrap();
    let args = [
        "init",
        "--project-dir",
        p,
        "--name",
        "Research",
        "--research",
        "Test queue mechanisms",
        "--no-setup",
    ];
    let (code, first) = run(h, &args);
    assert_eq!(code, 0, "{first}");
    let root = project.join(".paperhands");
    let r = root.to_str().unwrap();
    assert_eq!(
        run(h, &["tasks", "list", "--root", r]).1["data"]["total"],
        4
    );
    assert_eq!(
        run(h, &["memory", "list", "--root", r]).1["data"]["total"],
        1
    );
    assert_eq!(
        run(
            h,
            &[
                "tasks",
                "update",
                "--root",
                r,
                "--id",
                "frame-question",
                "--state",
                "done",
                "--note-text",
                "Controls identified"
            ]
        )
        .0,
        0
    );
    assert_eq!(run(h, &args).0, 0);
    assert_eq!(
        run(h, &["memory", "list", "--root", r]).1["data"]["total"],
        1
    );
    assert_eq!(
        run(h, &["tasks", "read", "--root", r, "--id", "frame-question"]).1["data"]["total_events"],
        2
    );
    assert_eq!(
        run(
            h,
            &["setup", "--home", h.to_str().unwrap(), "--project-dir", p]
        )
        .0,
        0
    );
    let mcp: Value = serde_json::from_slice(&fs::read(project.join(".mcp.json")).unwrap()).unwrap();
    assert_eq!(
        mcp["mcpServers"]["paperhands"]["args"],
        json!(["mcp", "--project-dir", project.canonicalize().unwrap()])
    );
    assert_eq!(
        run(
            h,
            &[
                "init",
                "--project-dir",
                p,
                "--research",
                "different question",
                "--no-setup"
            ]
        )
        .0,
        2
    );
    assert!(
        fs::read_to_string(project.join("research/BRIEF.md"))
            .unwrap()
            .contains("Test queue mechanisms")
    );
}

#[cfg(unix)]
fn fixture(home: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir(home.join("bin")).unwrap();
    let source = r##"#!/usr/bin/env python3
import json,os,pathlib,sys,time
a=sys.argv[1:];h=pathlib.Path(os.environ['WAKE_FIXTURE'])
if '--dry-run' in a: print(json.dumps({'dry_run':True}));sys.exit(0)
if 'identity' in a:
 if (h/'identity-fail').exists():
  print(json.dumps({'error':'identity capacity reached','next':[{'command':'PRIVATE_GUIDANCE_MUST_NOT_LEAK'}]}),file=sys.stderr);sys.exit(7)
 print(json.dumps({'address':'b'*64}));sys.exit(0)
if 'id' in a: print(json.dumps({'address':'a'*64}));sys.exit(0)
if 'dm' in a:
 with (h/'calls').open('a') as f: f.write(json.dumps(a)+'\n')
 if (h/'fail').exists(): sys.exit(7)
 time.sleep(.1)
 print(json.dumps({'id':'00000000-0000-0000-0000-000000000001','state':'recipient-stored'}));sys.exit(0)
sys.exit(2)
"##;
    fs::write(home.join("bin/icq"), source).unwrap();
    fs::set_permissions(home.join("bin/icq"), fs::Permissions::from_mode(0o700)).unwrap();
}

#[cfg(unix)]
fn prepared_schedule(home: &Path) -> std::path::PathBuf {
    fixture(home);
    let root = home.join("store");
    let r = root.to_str().unwrap();
    for args in [
        vec!["init", "--root", r],
        vec!["automation", "bind", "--root", r],
        vec![
            "automation",
            "add",
            "--root",
            r,
            "--id",
            "prepared",
            "--every",
            "43200",
            "--duration",
            "2592000",
            "--max-wakes",
            "60",
            "--message",
            "Fixture only",
        ],
        vec!["automation", "tick", "--root", r],
    ] {
        let (code, value) = run(home, &args);
        assert_eq!(code, 0, "{args:?}: {value}");
    }
    root.canonicalize().unwrap()
}

#[cfg(unix)]
fn control_snapshot(
    root: &Path,
) -> std::collections::BTreeMap<std::path::PathBuf, (u32, Option<Vec<u8>>)> {
    use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};
    let mut pending = vec![root.to_owned()];
    let mut members = std::collections::BTreeMap::new();
    while let Some(path) = pending.pop() {
        let metadata = fs::symlink_metadata(&path).unwrap();
        let bytes = if metadata.file_type().is_symlink() {
            Some(
                fs::read_link(&path)
                    .unwrap()
                    .as_os_str()
                    .as_bytes()
                    .to_vec(),
            )
        } else if metadata.is_dir() {
            pending.extend(
                fs::read_dir(&path)
                    .unwrap()
                    .map(|entry| entry.unwrap().path()),
            );
            None
        } else {
            Some(fs::read(&path).unwrap())
        };
        members.insert(
            path.strip_prefix(root).unwrap().to_owned(),
            (metadata.mode(), bytes),
        );
    }
    members
}

#[cfg(unix)]
#[test]
fn prepared_status_shares_read_locks_and_refuses_an_active_writer() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    let root = prepared_schedule(h);
    let r = root.to_str().unwrap();
    let before = control_snapshot(&root);
    let calls = fs::read(h.join("calls")).unwrap();
    let lock = fs::File::open(root.join("automation/automation.lock")).unwrap();
    fs2::FileExt::lock_shared(&lock).unwrap();
    let (code, status) = run(h, &["automation", "status", "--root", r]);
    assert_eq!(code, 0, "{status}");
    let row = &status["data"]["items"][0];
    assert_eq!(row["status"], "waiting");
    assert_eq!(row["attempted_wakes"], 1);
    assert_eq!(row["remaining_attempts"], 59);
    assert_eq!(row["last_receipt"]["model_delivery"], "unverified");
    let (code, busy) = run(h, &["automation", "tick", "--root", r]);
    assert_eq!(code, 2, "{busy}");
    assert!(busy["error"]["message"].as_str().unwrap().contains("busy"));
    fs2::FileExt::unlock(&lock).unwrap();
    fs2::FileExt::lock_exclusive(&lock).unwrap();
    let (code, busy) = run(h, &["automation", "status", "--root", r]);
    assert_eq!(code, 2, "{busy}");
    assert!(busy["error"]["message"].as_str().unwrap().contains("busy"));
    fs2::FileExt::unlock(&lock).unwrap();
    let (code, tick) = run(h, &["automation", "tick", "--root", r]);
    assert_eq!(code, 0, "{tick}");
    assert_eq!(tick["data"]["wake_attempts"], 0);
    assert_eq!(control_snapshot(&root), before);
    assert_eq!(fs::read(h.join("calls")).unwrap(), calls);
}

#[cfg(unix)]
#[test]
fn status_refuses_missing_or_unsafe_state_without_initializing_or_repairing_it() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    let empty = h.join("empty");
    assert_eq!(run(h, &["init", "--root", empty.to_str().unwrap()]).0, 0);
    let before = control_snapshot(&empty);
    assert_eq!(
        run(
            h,
            &["automation", "status", "--root", empty.to_str().unwrap()]
        )
        .0,
        2
    );
    assert_eq!(control_snapshot(&empty), before);
    let root = prepared_schedule(h);
    let r = root.to_str().unwrap();
    let original = control_snapshot(&root);
    for name in [
        "schedules",
        "attempts",
        "cancellations",
        "bindings",
        "automation.lock",
    ] {
        let path = root.join("automation").join(name);
        let moved = h.join("displaced");
        fs::rename(&path, &moved).unwrap();
        let missing = control_snapshot(&root);
        assert_eq!(
            run(h, &["automation", "status", "--root", r]).0,
            2,
            "{name}"
        );
        assert_eq!(control_snapshot(&root), missing);
        std::os::unix::fs::symlink(&moved, &path).unwrap();
        let linked = control_snapshot(&root);
        assert_eq!(
            run(h, &["automation", "status", "--root", r]).0,
            2,
            "{name}"
        );
        assert_eq!(control_snapshot(&root), linked);
        fs::remove_file(&path).unwrap();
        if name == "automation.lock" {
            fs::create_dir(&path).unwrap();
        } else {
            fs::write(&path, "not a directory").unwrap();
        }
        let wrong_type = control_snapshot(&root);
        assert_eq!(
            run(h, &["automation", "status", "--root", r]).0,
            2,
            "{name}"
        );
        assert_eq!(control_snapshot(&root), wrong_type);
        if name == "automation.lock" {
            fs::remove_dir(&path).unwrap();
        } else {
            fs::remove_file(&path).unwrap();
        }
        fs::rename(moved, path).unwrap();
    }
    assert_eq!(control_snapshot(&root), original);
}

#[cfg(target_os = "macos")]
#[test]
fn prepared_automation_status_works_when_project_writes_and_network_are_denied() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    let root = prepared_schedule(h);
    let r = root.to_str().unwrap();
    // Catch permission repair as well as file creation/content writes.
    fs::set_permissions(root.join("automation"), fs::Permissions::from_mode(0o750)).unwrap();
    let before = control_snapshot(&root);
    let calls = fs::read(h.join("calls")).unwrap();
    let profile = format!(
        "(version 1)(allow default)(deny network*)(deny file-write* (subpath {}))",
        serde_json::to_string(r).unwrap()
    );
    let mut reader = command_at(h, "/usr/bin/sandbox-exec");
    reader.args([
        "-p",
        &profile,
        env!("CARGO_BIN_EXE_paperhands"),
        "automation",
        "status",
        "--root",
        r,
    ]);
    let output = reader.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let status: Value = serde_json::from_slice(&output.stdout).unwrap();
    let row = &status["data"]["items"][0];
    assert_eq!(row["status"], "waiting");
    assert_eq!(row["attempted_wakes"], 1);
    assert_eq!(row["remaining_attempts"], 59);
    assert_eq!(row["next_due_reason"], "waiting");
    assert_eq!(row["last_receipt"]["model_delivery"], "unverified");
    assert_eq!(control_snapshot(&root), before);
    assert_eq!(fs::read(h.join("calls")).unwrap(), calls);
}
#[cfg(unix)]
#[test]
fn wake_slots_are_exclusive_bounded_literal_and_retained_when_unknown() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    fixture(h);
    let root = h.join("store");
    let r = root.to_str().unwrap();
    assert_eq!(run(h, &["init", "--root", r]).0, 0);
    let (code, bound) = run(h, &["automation", "bind", "--root", r]);
    assert_eq!(code, 0, "{bound}");
    assert!(
        bound["data"]["binding"]["sender"]
            .as_str()
            .unwrap()
            .starts_with("paperhands-")
    );
    let brief = "inspect $(touch SHOULD_NOT_EXIST) and `literal` text";
    assert_eq!(
        run(
            h,
            &[
                "automation",
                "add",
                "--root",
                r,
                "--id",
                "one",
                "--every",
                "60",
                "--duration",
                "3600",
                "--max-wakes",
                "1",
                "--message",
                brief
            ]
        )
        .0,
        0
    );
    assert_eq!(
        run(h, &["automation", "tick", "--root", r, "--dry-run"]).1["data"]["wake_attempts"],
        0
    );
    assert!(!h.join("calls").exists());
    let first = command(h)
        .args(["automation", "tick", "--root", r])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let second = command(h)
        .args(["automation", "tick", "--root", r])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let outputs = [
        first.wait_with_output().unwrap(),
        second.wait_with_output().unwrap(),
    ];
    assert!(outputs.iter().any(|o| o.status.success()));
    let calls = fs::read_to_string(h.join("calls")).unwrap();
    assert_eq!(calls.lines().count(), 1);
    assert!(calls.contains("SHOULD_NOT_EXIST"));
    assert_eq!(
        run(h, &["automation", "tick", "--root", r]).1["data"]["wake_attempts"],
        0
    );
    assert_eq!(
        run(h, &["automation", "status", "--root", r]).1["data"]["items"][0]["status"],
        "budget_exhausted"
    );
    fs::write(h.join("fail"), "").unwrap();
    let replacement = "c".repeat(64);
    assert_eq!(
        run(
            h,
            &[
                "automation",
                "bind",
                "--root",
                r,
                "--recipient",
                &replacement
            ]
        )
        .0,
        0
    );
    assert_eq!(
        run(
            h,
            &[
                "automation",
                "add",
                "--root",
                r,
                "--id",
                "unknown",
                "--every",
                "60",
                "--duration",
                "3600",
                "--max-wakes",
                "1",
                "--message",
                "Keep failure evidence"
            ]
        )
        .0,
        0
    );
    assert_eq!(run(h, &["automation", "tick", "--root", r]).0, 0);
    let (_, status) = run(h, &["automation", "status", "--root", r]);
    let row = status["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "unknown")
        .unwrap();
    assert_eq!(row["last_receipt"]["status"], "failed_or_unknown");
    assert_eq!(row["last_receipt"]["diagnostics"]["operation"], "icq.dm");
    assert_eq!(row["last_receipt"]["diagnostics"]["exit_code"], 7);
    assert_eq!(
        run(h, &["automation", "tick", "--root", r]).1["data"]["wake_attempts"],
        0
    );
    assert_eq!(
        fs::read_to_string(h.join("calls")).unwrap().lines().count(),
        2
    );
    let calls = fs::read_to_string(h.join("calls")).unwrap();
    assert!(calls.lines().next().unwrap().contains(&"a".repeat(64)));
    assert!(calls.lines().nth(1).unwrap().contains(&replacement));
    let result = root.join("automation/attempts/one/0.result.json");
    fs::write(result, "{}").unwrap();
    assert_eq!(run(h, &["automation", "status", "--root", r]).0, 2);
}

#[cfg(unix)]
fn cron_fixture(h: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir(h.join("bin")).unwrap();
    fs::write(
        h.join("bin/crontab"),
        r#"#!/usr/bin/env python3
import os,pathlib,sys
h=pathlib.Path(os.environ['WAKE_FIXTURE']);p=h/'crontab'
if sys.argv[1:] == ['-l']:
 if (h/'cron-read-fail').exists(): print('crontab: read denied',file=sys.stderr);sys.exit(13)
 if not p.exists(): print('no crontab for fixture',file=sys.stderr);sys.exit(1)
 sys.stdout.buffer.write(p.read_bytes())
elif sys.argv[1:] == ['-']:
 if (h/'cron-write-fail').exists(): print('crontab: update denied',file=sys.stderr);sys.exit(9)
 p.write_bytes(sys.stdin.buffer.read())
else: sys.exit(2)
"#,
    )
    .unwrap();
    fs::set_permissions(h.join("bin/crontab"), fs::Permissions::from_mode(0o700)).unwrap();
}

#[cfg(unix)]
#[test]
fn cron_installer_roundtrips_only_its_owned_block() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    cron_fixture(h);
    let old = "MAILTO=fixture\n5 * * * * unrelated-collector\n";
    fs::write(h.join("crontab"), old).unwrap();
    let root = h.join("store with spaces");
    let r = root.to_str().unwrap();
    assert_eq!(run(h, &["init", "--root", r]).0, 0);
    let (code, preview) = run(h, &["automation", "install", "--root", r, "--dry-run"]);
    assert_eq!(code, 0, "{preview}");
    assert_eq!(fs::read_to_string(h.join("crontab")).unwrap(), old);
    let (code, installed) = run(h, &["automation", "install", "--root", r]);
    assert_eq!(code, 0, "{installed}");
    let actual = fs::read_to_string(h.join("crontab")).unwrap();
    assert!(actual.starts_with(old));
    assert!(actual.contains("automation tick --root"));
    assert_eq!(run(h, &["automation", "install", "--root", r]).0, 0);
    assert_eq!(fs::read_to_string(h.join("crontab")).unwrap(), actual);
    assert_eq!(run(h, &["automation", "uninstall", "--root", r]).0, 0);
    assert_eq!(fs::read_to_string(h.join("crontab")).unwrap(), old);
}

#[cfg(unix)]
#[test]
fn identity_capacity_failure_retains_the_child_reason_without_a_binding() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    fixture(h);
    fs::write(h.join("identity-fail"), "").unwrap();
    let root = h.join("store");
    let r = root.to_str().unwrap();
    assert_eq!(run(h, &["init", "--root", r]).0, 0);
    let (code, failure) = run(h, &["automation", "bind", "--root", r]);
    assert_eq!(code, 2, "{failure}");
    let details = &failure["error"]["diagnostics"];
    assert_eq!(details["operation"], "icq.identity.create");
    assert_eq!(details["phase"], "execute");
    assert_eq!(details["exit_code"], 7);
    assert_eq!(details["timed_out"], false);
    assert_eq!(details["stderr"], "identity capacity reached");
    assert!(
        failure["error"]["message"]
            .as_str()
            .unwrap()
            .contains("identity capacity reached")
    );
    assert!(
        !failure
            .to_string()
            .contains("PRIVATE_GUIDANCE_MUST_NOT_LEAK")
    );
    assert!(!root.join("automation/binding-head").exists());
    assert_eq!(
        fs::read_dir(root.join("automation/bindings"))
            .unwrap()
            .count(),
        0
    );
}

#[cfg(unix)]
#[test]
fn waiting_schedule_exposes_its_next_slot_and_remaining_budget() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    fixture(h);
    let root = h.join("store");
    let r = root.to_str().unwrap();
    assert_eq!(run(h, &["init", "--root", r]).0, 0);
    assert_eq!(run(h, &["automation", "bind", "--root", r]).0, 0);
    let (code, _) = run(
        h,
        &[
            "automation",
            "add",
            "--root",
            r,
            "--id",
            "twelve-hours",
            "--every",
            "43200",
            "--duration",
            "2592000",
            "--max-wakes",
            "60",
            "--message",
            "Fixture only",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(run(h, &["automation", "tick", "--root", r]).0, 0);
    let schedule: Value = serde_json::from_slice(
        &fs::read(root.join("automation/schedules/twelve-hours.json")).unwrap(),
    )
    .unwrap();
    let (code, status) = run(h, &["automation", "status", "--root", r]);
    assert_eq!(code, 0, "{status}");
    let row = &status["data"]["items"][0];
    assert_eq!(row["status"], "waiting");
    assert_eq!(row["remaining_attempts"], 59);
    assert_eq!(
        row["next_due_at_ms"].as_u64().unwrap(),
        schedule["start_at_ms"].as_u64().unwrap() + 43_200_000
    );
    assert_eq!(row["next_due_reason"], "waiting");
    assert_eq!(row["last_receipt"]["model_delivery"], "unverified");
    assert_eq!(
        run(h, &["automation", "tick", "--root", r]).1["data"]["wake_attempts"],
        0
    );
    assert_eq!(
        fs::read_to_string(h.join("calls")).unwrap().lines().count(),
        1
    );
}

#[cfg(unix)]
fn report_clock_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(unix)]
fn report_schedule(home: &Path, root: &Path, id: &str, out: &Path) {
    let (code, value) = run(
        home,
        &[
            "automation",
            "add",
            "--root",
            root.to_str().unwrap(),
            "--id",
            id,
            "--every",
            "60",
            "--duration",
            "3600",
            "--max-wakes",
            "1",
            "--message",
            "Private fixture; never publish this brief",
            "--report-out",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{value}");
    let path = root.join("automation/schedules").join(format!("{id}.json"));
    let mut schedule: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(schedule["report_out"], out.to_str().unwrap());
    // A future-start owned schedule stays waiting, without claiming or sending a wake.
    let now = report_clock_ms();
    schedule["start_at_ms"] = (now + 3_600_000).into();
    schedule["expires_at_ms"] = (now + 7_200_000).into();
    let bytes = serde_json::to_vec_pretty(&schedule).unwrap();
    fs::write(&path, &bytes).unwrap();
    fs::write(
        path.with_extension("sha256"),
        paperhands::research::inventory::hash(&bytes),
    )
    .unwrap();
}

#[cfg(unix)]
fn private_report_receipts(root: &Path) -> Vec<Value> {
    let mut values = fs::read_dir(root.join("automation/reports"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.file_name().unwrap() != "latest-skip.json")
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .map(|path| paperhands::research::inventory::read_sealed(&path).unwrap())
        .collect::<Vec<Value>>();
    values.sort_by(|a, b| {
        a["recorded_at_ms"]
            .as_u64()
            .cmp(&b["recorded_at_ms"].as_u64())
    });
    values
}

#[cfg(unix)]
#[test]
fn ticks_refresh_missing_and_stale_reports_once_per_directory_without_wakes() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    fixture(h);
    let root = h.join("store");
    let r = root.to_str().unwrap();
    assert_eq!(run(h, &["init", "--root", r, "--no-setup"]).0, 0);
    assert_eq!(run(h, &["automation", "bind", "--root", r]).0, 0);
    let out = h.join("private-report-output");
    report_schedule(h, &root, "private-first", &out);
    report_schedule(h, &root, "private-second", &out);
    let before = control_snapshot(&root);
    assert_eq!(run(h, &["automation", "status", "--root", r]).0, 0);
    assert_eq!(
        run(h, &["automation", "tick", "--root", r, "--dry-run"]).0,
        0
    );
    assert!(!out.exists());
    assert!(!root.join("automation/reports").exists());
    assert_eq!(control_snapshot(&root), before);

    let (code, missing) = run(h, &["automation", "tick", "--root", r]);
    assert_eq!(code, 0, "{missing}");
    assert_eq!(missing["data"]["wake_attempts"], 0);
    assert_eq!(missing["data"]["reports"].as_array().unwrap().len(), 1);
    assert_eq!(missing["data"]["reports"][0]["report"], "written");
    assert_eq!(missing["data"]["reports"][0]["reason"], "missing");
    let json = fs::read(out.join("report.json")).unwrap();
    let markdown = fs::read(out.join("report.md")).unwrap();
    let public: Value = serde_json::from_slice(&json).unwrap();
    assert_eq!(public["schema"], "paperhands.report/v1");
    assert_eq!(public["automation"]["model_or_evaluator_started"], false);
    assert!(
        missing["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| { row["status"] == "waiting" && row["attempted_wakes"] == 0 })
    );
    for private in [
        "private-report-output",
        "private-first",
        "private-second",
        "Private fixture",
    ] {
        assert!(!String::from_utf8_lossy(&json).contains(private));
        assert!(!String::from_utf8_lossy(&markdown).contains(private));
    }
    let receipts = private_report_receipts(&root);
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0]["report_out"], out.to_str().unwrap());
    assert_eq!(receipts[0]["report"], "written");
    assert_eq!(receipts[0]["generated_at_ms"], public["generated_at_ms"]);
    assert!(
        receipts[0]["recorded_at_ms"].as_u64().unwrap()
            >= public["generated_at_ms"].as_u64().unwrap()
    );
    let reports = root.join("automation/reports");
    let sealed_before_skip = control_snapshot(&reports);

    for _ in 0..4 {
        let tick_started = report_clock_ms();
        let (code, fresh) = run(h, &["automation", "tick", "--root", r]);
        let tick_completed = report_clock_ms();
        assert_eq!(code, 0, "{fresh}");
        assert_eq!(fresh["data"]["reports"][0]["report"], "skipped");
        assert_eq!(fresh["data"]["reports"][0]["reason"], "fresh");
        assert!(fresh["data"]["reports"][0].get("receipt_sha256").is_none());
        assert_eq!(
            fresh["data"]["reports"][0]["latest_skip_sha256"],
            paperhands::research::inventory::hash(
                &fs::read(reports.join("latest-skip.json")).unwrap()
            )
        );
        let skip: Value =
            serde_json::from_slice(&fs::read(reports.join("latest-skip.json")).unwrap()).unwrap();
        assert_eq!(skip["report"], "skipped");
        assert_eq!(skip["reason"], "fresh");
        assert_eq!(skip["generated_at_ms"], public["generated_at_ms"]);
        assert!(
            skip["recorded_at_ms"].as_u64().unwrap() >= skip["generated_at_ms"].as_u64().unwrap()
        );
        assert!(
            (tick_started..=tick_completed).contains(&skip["recorded_at_ms"].as_u64().unwrap())
        );
        assert!(!reports.join("latest-skip.sha256").exists());
        let mut after_skip = control_snapshot(&reports);
        assert!(after_skip.remove(Path::new("latest-skip.json")).is_some());
        assert_eq!(after_skip, sealed_before_skip);
        assert_eq!(private_report_receipts(&root).len(), 1);
    }
    assert_eq!(fs::read(out.join("report.json")).unwrap(), json);
    assert_eq!(fs::read(out.join("report.md")).unwrap(), markdown);

    let mut stale = public;
    stale["generated_at_ms"] = (stale["generated_at_ms"].as_u64().unwrap() - 3_600_001).into();
    fs::write(out.join("report.json"), serde_json::to_vec(&stale).unwrap()).unwrap();
    let (code, refreshed) = run(h, &["automation", "tick", "--root", r]);
    assert_eq!(code, 0, "{refreshed}");
    assert_eq!(refreshed["data"]["reports"][0]["report"], "written");
    assert_eq!(refreshed["data"]["reports"][0]["reason"], "stale");
    let current: Value =
        serde_json::from_slice(&fs::read(out.join("report.json")).unwrap()).unwrap();
    assert!(
        current["generated_at_ms"].as_u64().unwrap() > stale["generated_at_ms"].as_u64().unwrap()
    );
    let receipts = private_report_receipts(&root);
    assert_eq!(receipts.len(), 2);
    let refreshed_receipt = receipts
        .iter()
        .find(|receipt| receipt["reason"] == "stale")
        .unwrap();
    assert_eq!(
        refreshed_receipt["generated_at_ms"],
        current["generated_at_ms"]
    );
    assert!(
        refreshed_receipt["recorded_at_ms"].as_u64().unwrap()
            >= current["generated_at_ms"].as_u64().unwrap()
    );
    for id in ["private-first", "private-second"] {
        let (code, value) = run(
            h,
            &[
                "automation",
                "cancel",
                "--root",
                r,
                "--id",
                id,
                "--reason",
                "Owned report fixture complete",
            ],
        );
        assert_eq!(code, 0, "{value}");
    }
    let private_evidence = control_snapshot(&reports);
    let public_files = control_snapshot(&out);
    let (code, terminal) = run(h, &["automation", "tick", "--root", r]);
    assert_eq!(code, 0, "{terminal}");
    assert!(
        terminal["data"]["reports"]
            .as_array()
            .is_none_or(|rows| rows.is_empty())
    );
    assert!(
        terminal["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["status"] == "cancelled")
    );
    assert_eq!(control_snapshot(&reports), private_evidence);
    assert_eq!(control_snapshot(&out), public_files);
    assert!(!h.join("calls").exists());
    assert!(
        fs::read_dir(root.join("automation/attempts/private-first"))
            .unwrap()
            .next()
            .is_none()
    );
    assert!(
        fs::read_dir(root.join("automation/attempts/private-second"))
            .unwrap()
            .next()
            .is_none()
    );
}

#[cfg(unix)]
#[test]
fn tick_report_failures_are_retained_without_overwriting_bad_reports_or_starting_wakes() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    fixture(h);
    let root = h.join("store");
    let r = root.to_str().unwrap();
    assert_eq!(run(h, &["init", "--root", r, "--no-setup"]).0, 0);
    assert_eq!(run(h, &["automation", "bind", "--root", r]).0, 0);
    let out = h.join("invalid-report");
    fs::create_dir(&out).unwrap();
    fs::write(out.join("report.json"), b"{invalid retained report").unwrap();
    report_schedule(h, &root, "invalid", &out);
    let (code, invalid) = run(h, &["automation", "tick", "--root", r]);
    assert_eq!(code, 0, "{invalid}");
    assert_eq!(invalid["data"]["reports"][0]["report"], "failed");
    assert_eq!(
        invalid["data"]["reports"][0]["reason"],
        "existing_report_invalid"
    );
    assert_eq!(
        fs::read(out.join("report.json")).unwrap(),
        b"{invalid retained report"
    );
    assert!(!out.join("report.md").exists());
    let receipts = private_report_receipts(&root);
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0]["report"], "failed");
    assert!(receipts[0]["error"].as_str().is_some());
    assert_eq!(receipts[0]["generated_at_ms"], Value::Null);
    assert!(receipts[0]["recorded_at_ms"].as_u64().is_some());

    let blocked = h.join("blocked-report");
    fs::create_dir(&blocked).unwrap();
    // A conflicting markdown directory makes the report writer refuse publication.
    fs::create_dir(blocked.join("report.md")).unwrap();
    report_schedule(h, &root, "blocked", &blocked);
    let (code, failure) = run(h, &["automation", "tick", "--root", r]);
    assert_eq!(code, 0, "{failure}");
    assert!(
        failure["data"]["reports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| { row["report"] == "failed" && row["reason"] == "write_failed" })
    );
    let receipts = private_report_receipts(&root);
    assert_eq!(receipts.len(), 3);
    assert!(
        receipts
            .iter()
            .all(|receipt| receipt["generated_at_ms"] == Value::Null
                && receipt["recorded_at_ms"].as_u64().is_some())
    );
    assert!(
        receipts
            .iter()
            .any(|row| row["reason"] == "write_failed" && row["error"].as_str().is_some())
    );
    assert_eq!(failure["data"]["wake_attempts"], 0);
    assert!(!h.join("calls").exists());
}

#[cfg(unix)]
#[test]
fn terminal_schedules_keep_status_coverage_without_creating_reports() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    fixture(h);
    let root = h.join("store");
    let r = root.to_str().unwrap();
    assert_eq!(run(h, &["init", "--root", r, "--no-setup"]).0, 0);
    assert_eq!(run(h, &["automation", "bind", "--root", r]).0, 0);
    let now = report_clock_ms();
    for id in ["cancelled", "expired", "exhausted"] {
        report_schedule(h, &root, id, &h.join(format!("{id}-output")));
        if id == "cancelled" {
            let (code, value) = run(
                h,
                &[
                    "automation",
                    "cancel",
                    "--root",
                    r,
                    "--id",
                    id,
                    "--reason",
                    "Owned cancellation fixture",
                ],
            );
            assert_eq!(code, 0, "{value}");
            continue;
        }
        let path = root.join("automation/schedules").join(format!("{id}.json"));
        let mut schedule: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        if id == "expired" {
            schedule["start_at_ms"] = (now - 7_200_000).into();
            schedule["expires_at_ms"] = (now - 3_600_000).into();
        } else {
            schedule["start_at_ms"] = (now - 5_000).into();
            schedule["expires_at_ms"] = (now + 3_600_000).into();
        }
        let bytes = serde_json::to_vec_pretty(&schedule).unwrap();
        let digest = paperhands::research::inventory::hash(&bytes);
        fs::write(&path, bytes).unwrap();
        fs::write(path.with_extension("sha256"), &digest).unwrap();
        if id == "exhausted" {
            paperhands::research::inventory::sealed_json(
                &root.join("automation/attempts/exhausted/0.intent.json"),
                &serde_json::json!({"schedule_sha256":digest,"slot":0,"claimed_at_ms":now,"automatic_retry":false}),
            ).unwrap();
        }
    }
    let before = control_snapshot(&root);
    let (code, tick) = run(h, &["automation", "tick", "--root", r]);
    assert_eq!(code, 0, "{tick}");
    assert_eq!(tick["data"]["wake_attempts"], 0);
    assert!(
        tick["data"]["reports"]
            .as_array()
            .is_none_or(|rows| rows.is_empty())
    );
    let rows = tick["data"]["items"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    for (id, status) in [
        ("cancelled", "cancelled"),
        ("expired", "expired"),
        ("exhausted", "budget_exhausted"),
    ] {
        let row = rows.iter().find(|row| row["id"] == id).unwrap();
        assert_eq!(row["status"], status);
        assert!(!h.join(format!("{id}-output")).exists());
    }
    assert_eq!(control_snapshot(&root), before);
    assert!(!root.join("automation/reports").exists());
    assert!(!h.join("calls").exists());
    let (code, status) = run(h, &["automation", "status", "--root", r]);
    assert_eq!(code, 0, "{status}");
    assert_eq!(status["data"]["items"], tick["data"]["items"]);
    assert_eq!(control_snapshot(&root), before);
}

#[cfg(unix)]
#[test]
fn a_due_schedule_reports_once_when_its_wake_consumes_the_final_budget() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    fixture(h); // Every ICQ command resolves to this owned local fake, never the real client.
    let root = h.join("store");
    let r = root.to_str().unwrap();
    assert_eq!(run(h, &["init", "--root", r, "--no-setup"]).0, 0);
    assert_eq!(run(h, &["automation", "bind", "--root", r]).0, 0);
    let out = h.join("due-output");
    report_schedule(h, &root, "due", &out);
    let path = root.join("automation/schedules/due.json");
    let mut schedule: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let now = report_clock_ms();
    schedule["start_at_ms"] = (now - 5_000).into();
    schedule["expires_at_ms"] = (now + 3_600_000).into();
    let bytes = serde_json::to_vec_pretty(&schedule).unwrap();
    fs::write(&path, &bytes).unwrap();
    fs::write(
        path.with_extension("sha256"),
        paperhands::research::inventory::hash(&bytes),
    )
    .unwrap();
    let (code, tick) = run(h, &["automation", "tick", "--root", r]);
    assert_eq!(code, 0, "{tick}");
    assert_eq!(tick["data"]["wake_attempts"], 1);
    assert_eq!(tick["data"]["items"][0]["status"], "budget_exhausted");
    assert_eq!(tick["data"]["reports"][0]["report"], "written");
    assert_eq!(tick["data"]["reports"][0]["reason"], "missing");
    assert_eq!(private_report_receipts(&root).len(), 1);
    assert_eq!(
        fs::read_to_string(h.join("calls")).unwrap().lines().count(),
        1
    );
    let evidence = control_snapshot(&root);
    let output = control_snapshot(&out);
    let calls = fs::read(h.join("calls")).unwrap();
    let (code, repeated) = run(h, &["automation", "tick", "--root", r]);
    assert_eq!(code, 0, "{repeated}");
    assert_eq!(repeated["data"]["wake_attempts"], 0);
    assert!(
        repeated["data"]["reports"]
            .as_array()
            .is_none_or(|rows| rows.is_empty())
    );
    assert_eq!(control_snapshot(&root), evidence);
    assert_eq!(control_snapshot(&out), output);
    assert_eq!(fs::read(h.join("calls")).unwrap(), calls);
}

#[cfg(unix)]
fn zoom_fixture(home: &Path) -> std::path::PathBuf {
    fixture(home);
    let root = home.join("store");
    let r = root.to_str().unwrap();
    assert_eq!(run(home, &["init", "--root", r, "--no-setup"]).0, 0);
    assert_eq!(run(home, &["automation", "bind", "--root", r]).0, 0);
    assert_eq!(
        run(
            home,
            &[
                "memory",
                "add",
                "--root",
                r,
                "--kind",
                "source",
                "--title",
                "Owned fresh source",
                "--text",
                "Synthetic fresh source; no market evidence or execution."
            ]
        )
        .0,
        0
    );
    root
}

#[cfg(unix)]
fn zoom_schedule(home: &Path, root: &Path, prompt: &Path, max_wakes: u64, age_ms: u64) {
    let max = max_wakes.to_string();
    let (code, value) = run(
        home,
        &[
            "automation",
            "add",
            "--root",
            root.to_str().unwrap(),
            "--id",
            "zoom",
            "--every",
            "60",
            "--duration",
            "3600",
            "--max-wakes",
            &max,
            "--message",
            "ROUTINE_FIXTURE_BRIEF",
            "--zoom-prompt",
            prompt.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{value}");
    let path = root.join("automation/schedules/zoom.json");
    let mut schedule: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let now = report_clock_ms();
    schedule["start_at_ms"] = (now - age_ms).into();
    schedule["expires_at_ms"] = (now + 3_600_000).into();
    let bytes = serde_json::to_vec_pretty(&schedule).unwrap();
    fs::write(&path, &bytes).unwrap();
    fs::write(
        path.with_extension("sha256"),
        paperhands::research::inventory::hash(&bytes),
    )
    .unwrap();
}

#[cfg(unix)]
fn zoom_claim(root: &Path, slot: u64, claimed_at_ms: Value) {
    paperhands::research::inventory::sealed_json(
        &root.join("automation/attempts/zoom").join(format!("{slot}.intent.json")),
        &serde_json::json!({"slot":slot,"schedule_sha256":paperhands::research::inventory::digest(&root.join("automation/schedules/zoom.json")).unwrap(),"claimed_at_ms":claimed_at_ms,"automatic_retry":false}),
    ).unwrap();
}

#[cfg(unix)]
#[test]
fn zoom_prompt_is_frozen_and_a_stalled_due_wake_uses_one_original_attempt() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    let root = zoom_fixture(h);
    let r = root.to_str().unwrap();
    assert_eq!(
        run(
            h,
            &[
                "tasks",
                "add",
                "--root",
                r,
                "--id",
                "refusal",
                "--title",
                "Owned refused question",
                "--kind",
                "research"
            ]
        )
        .0,
        0
    );
    for _ in 0..3 {
        assert_eq!(
            run(
                h,
                &[
                    "tasks",
                    "update",
                    "--root",
                    r,
                    "--id",
                    "refusal",
                    "--state",
                    "blocked",
                    "--note-text",
                    "Identical owned admission refusal"
                ]
            )
            .0,
            0
        );
    }
    let prompt = h.join("zoom-brief.txt");
    let frozen = "PREMISE_REVIEW_FIXTURE\r\nLook again — exact UTF-8.\n";
    fs::write(&prompt, frozen).unwrap();
    zoom_schedule(h, &root, &prompt, 1, 5_000);
    let schedule: Value =
        paperhands::research::inventory::read_sealed(&root.join("automation/schedules/zoom.json"))
            .unwrap();
    assert_eq!(schedule["zoom_prompt"], frozen);
    assert!(!schedule.to_string().contains("zoom-brief.txt"));
    let path = root.join("automation/schedules/zoom.json");
    let original = fs::read(&path).unwrap();
    let mut waiting = schedule;
    waiting["start_at_ms"] = (report_clock_ms() + 3_600_000).into();
    waiting["expires_at_ms"] = (report_clock_ms() + 7_200_000).into();
    let waiting_bytes = serde_json::to_vec_pretty(&waiting).unwrap();
    fs::write(&path, &waiting_bytes).unwrap();
    fs::write(
        path.with_extension("sha256"),
        paperhands::research::inventory::hash(&waiting_bytes),
    )
    .unwrap();
    fs::write(&prompt, "MUTATED_FILE_MUST_NOT_BE_USED").unwrap();
    let before = control_snapshot(&root);
    assert_eq!(run(h, &["automation", "status", "--root", r]).0, 0);
    assert_eq!(
        run(h, &["automation", "tick", "--root", r, "--dry-run"]).0,
        0
    );
    let (code, waiting_tick) = run(h, &["automation", "tick", "--root", r]);
    assert_eq!(code, 0, "{waiting_tick}");
    assert_eq!(waiting_tick["data"]["wake_attempts"], 0);
    assert_eq!(waiting_tick["data"]["items"][0]["status"], "waiting");
    assert_eq!(control_snapshot(&root), before);
    assert!(!h.join("calls").exists());
    fs::write(&path, &original).unwrap();
    fs::write(
        path.with_extension("sha256"),
        paperhands::research::inventory::hash(&original),
    )
    .unwrap();
    fs::remove_file(prompt).unwrap();
    let (code, tick) = run(h, &["automation", "tick", "--root", r]);
    assert_eq!(code, 0, "{tick}");
    assert_eq!(tick["data"]["wake_attempts"], 1);
    assert_eq!(tick["data"]["items"][0]["attempted_wakes"], 1);
    assert_eq!(tick["data"]["items"][0]["status"], "budget_exhausted");
    let calls = fs::read_to_string(h.join("calls")).unwrap();
    assert_eq!(calls.lines().count(), 1);
    let argv: Vec<String> = serde_json::from_str(calls.lines().next().unwrap()).unwrap();
    assert!(argv.last().unwrap().contains(frozen));
    assert!(!argv.last().unwrap().contains("ROUTINE_FIXTURE_BRIEF"));
    assert!(
        !argv
            .last()
            .unwrap()
            .contains("MUTATED_FILE_MUST_NOT_BE_USED")
    );
    let receipt: Value = paperhands::research::inventory::read_sealed(
        &root.join("automation/attempts/zoom/0.result.json"),
    )
    .unwrap();
    assert_eq!(receipt["zoom"]["verdict"], "stalled");
    assert!(
        receipt["zoom"]["signals_fired"]
            .as_array()
            .unwrap()
            .iter()
            .any(|kind| kind == "repeated_refusal")
    );
    let intent: Value = paperhands::research::inventory::read_sealed(
        &root.join("automation/attempts/zoom/0.intent.json"),
    )
    .unwrap();
    assert_eq!(intent["zoom"], receipt["zoom"]);
    let after = control_snapshot(&root);
    assert_eq!(
        run(h, &["automation", "tick", "--root", r]).1["data"]["wake_attempts"],
        0
    );
    assert_eq!(control_snapshot(&root), after);
    assert_eq!(fs::read_to_string(h.join("calls")).unwrap(), calls);
}

#[cfg(unix)]
#[test]
fn routine_zoom_wakes_preserve_moving_drifting_and_unknown_history_without_retries() {
    for expected in ["moving", "drifting", "unknown", "unknown-collection"] {
        let home = tempfile::tempdir().unwrap();
        let h = home.path();
        let root = zoom_fixture(h);
        let r = root.to_str().unwrap();
        let prompt = h.join("premise.txt");
        fs::write(&prompt, "PREMISE_MUST_NOT_REPLACE_ROUTINE").unwrap();
        let (max, age, slot) = match expected {
            "drifting" => (3, 125_000, 2),
            "unknown" => (2, 65_000, 1),
            _ => (1, 5_000, 0),
        };
        zoom_schedule(h, &root, &prompt, max, age);
        if expected == "drifting" {
            zoom_claim(&root, 0, (report_clock_ms() - 120_000).into());
            zoom_claim(&root, 1, (report_clock_ms() - 60_000).into());
        } else if expected == "unknown" {
            // Existing status retains this spent ambiguity; zoom cannot invent its clock.
            zoom_claim(&root, 0, Value::Null);
        } else if expected == "unknown-collection" {
            let note = fs::read_dir(root.join("memory"))
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .find(|path| {
                    path.extension()
                        .is_some_and(|extension| extension == "json")
                })
                .unwrap();
            fs::write(note, b"{}").unwrap();
        }
        let before = control_snapshot(&root);
        let (code, tick) = run(h, &["automation", "tick", "--root", r]);
        assert_eq!(code, 0, "{expected}: {tick}");
        assert_eq!(tick["data"]["wake_attempts"], 1);
        let calls = fs::read_to_string(h.join("calls")).unwrap();
        assert_eq!(calls.lines().count(), 1);
        let argv: Vec<String> = serde_json::from_str(calls.lines().next().unwrap()).unwrap();
        assert!(argv.last().unwrap().contains("ROUTINE_FIXTURE_BRIEF"));
        assert!(
            !argv
                .last()
                .unwrap()
                .contains("PREMISE_MUST_NOT_REPLACE_ROUTINE")
        );
        let receipt: Value = paperhands::research::inventory::read_sealed(
            &root
                .join("automation/attempts/zoom")
                .join(format!("{slot}.result.json")),
        )
        .unwrap();
        if expected.starts_with("unknown") {
            assert_eq!(receipt["zoom"]["verdict"], Value::Null);
            let error = receipt["zoom"]["error"].as_str().unwrap();
            assert!(error.contains(if expected == "unknown" {
                "claim clock"
            } else {
                "verdict is unknown"
            }));
        } else {
            assert_eq!(receipt["zoom"]["verdict"], expected);
        }
        let after = control_snapshot(&root);
        assert!(
            before
                .iter()
                .all(|(path, value)| after.get(path) == Some(value))
        );
        assert_eq!(
            run(h, &["automation", "tick", "--root", r]).1["data"]["wake_attempts"],
            0
        );
        assert_eq!(control_snapshot(&root), after);
        assert_eq!(fs::read_to_string(h.join("calls")).unwrap(), calls);
    }
}

#[cfg(unix)]
#[test]
fn invalid_zoom_prompt_files_refuse_before_creating_schedule_or_attempt_members() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    let root = zoom_fixture(h);
    let prompt = h.join("prompt.txt");
    for bytes in [
        vec![b'x'; 8193],
        vec![0xff],
        vec![b' ', b'\n'],
        b"invalid\0brief".to_vec(),
    ] {
        fs::write(&prompt, bytes).unwrap();
        let before = control_snapshot(&root);
        let (code, value) = run(
            h,
            &[
                "automation",
                "add",
                "--root",
                root.to_str().unwrap(),
                "--id",
                "invalid",
                "--every",
                "60",
                "--duration",
                "3600",
                "--max-wakes",
                "1",
                "--message",
                "Routine",
                "--zoom-prompt",
                prompt.to_str().unwrap(),
            ],
        );
        assert_eq!(code, 2, "{value}");
        assert_eq!(control_snapshot(&root), before);
    }
    fs::remove_file(&prompt).unwrap();
    std::os::unix::fs::symlink(h.join("missing-target"), &prompt).unwrap();
    let before = control_snapshot(&root);
    assert_eq!(
        run(
            h,
            &[
                "automation",
                "add",
                "--root",
                root.to_str().unwrap(),
                "--id",
                "invalid",
                "--every",
                "60",
                "--duration",
                "3600",
                "--max-wakes",
                "1",
                "--message",
                "Routine",
                "--zoom-prompt",
                prompt.to_str().unwrap()
            ]
        )
        .0,
        2
    );
    assert_eq!(control_snapshot(&root), before);
    assert!(!h.join("calls").exists());
}

#[cfg(unix)]
#[test]
fn cron_failures_expose_subprocess_diagnostics_without_changing_jobs() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    cron_fixture(h);
    let old = "MAILTO=fixture\n5 * * * * unrelated-collector\n";
    fs::write(h.join("crontab"), old).unwrap();
    fs::write(h.join("cron-write-fail"), "").unwrap();
    let root = h.join("store");
    let r = root.to_str().unwrap();
    assert_eq!(run(h, &["init", "--root", r]).0, 0);
    let (code, failure) = run(h, &["automation", "install", "--root", r]);
    assert_eq!(code, 2, "{failure}");
    let details = &failure["error"]["diagnostics"];
    assert_eq!(details["operation"], "crontab");
    assert_eq!(details["phase"], "write");
    assert_eq!(details["exit_code"], 9);
    assert_eq!(details["timed_out"], false);
    assert!(
        details["stderr"]
            .as_str()
            .unwrap()
            .contains("crontab: update denied")
    );
    assert!(
        failure["error"]["message"]
            .as_str()
            .unwrap()
            .contains("crontab: update denied")
    );
    assert_eq!(fs::read_to_string(h.join("crontab")).unwrap(), old);
    fs::write(h.join("cron-read-fail"), "").unwrap();
    let (code, failure) = run(h, &["automation", "install", "--root", r]);
    assert_eq!(code, 2, "{failure}");
    assert_eq!(failure["error"]["diagnostics"]["phase"], "read");
    assert_eq!(failure["error"]["diagnostics"]["exit_code"], 13);
    assert_eq!(fs::read_to_string(h.join("crontab")).unwrap(), old);
}
