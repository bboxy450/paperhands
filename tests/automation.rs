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
    // Explicitly expire only this owned synthetic schedule, before any tick.
    // Reporting must not change the wake's expired decision or claim an intent.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    schedule["start_at_ms"] = (now - 7_200_000).into();
    schedule["expires_at_ms"] = (now - 3_600_000).into();
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
            .all(|row| { row["status"] == "expired" && row["attempted_wakes"] == 0 })
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

    let (code, fresh) = run(h, &["automation", "tick", "--root", r]);
    assert_eq!(code, 0, "{fresh}");
    assert_eq!(fresh["data"]["reports"][0]["report"], "skipped");
    assert_eq!(fresh["data"]["reports"][0]["reason"], "fresh");
    assert_eq!(fs::read(out.join("report.json")).unwrap(), json);
    assert_eq!(fs::read(out.join("report.md")).unwrap(), markdown);
    assert_eq!(private_report_receipts(&root).len(), 2);

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
    assert_eq!(private_report_receipts(&root).len(), 3);
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
            .any(|row| row["reason"] == "write_failed" && row["error"].as_str().is_some())
    );
    assert_eq!(failure["data"]["wake_attempts"], 0);
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
