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
    let mut command = Command::new(env!("CARGO_BIN_EXE_paperhands"));
    command
        .env("HOME", home)
        .env_remove("CODEX_HOME")
        .env_remove("CLAUDE_CONFIG_DIR")
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
if 'identity' in a: print(json.dumps({'address':'b'*64}));sys.exit(0)
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
#[test]
fn cron_installer_roundtrips_only_its_owned_block() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    fs::create_dir(h.join("bin")).unwrap();
    fs::write(
        h.join("bin/crontab"),
        r#"#!/usr/bin/env python3
import os,pathlib,sys
p=pathlib.Path(os.environ['WAKE_FIXTURE'])/'crontab'
if sys.argv[1:] == ['-l']:
 if not p.exists(): print('no crontab for fixture',file=sys.stderr);sys.exit(1)
 sys.stdout.buffer.write(p.read_bytes())
elif sys.argv[1:] == ['-']: p.write_bytes(sys.stdin.buffer.read())
else: sys.exit(2)
"#,
    )
    .unwrap();
    fs::set_permissions(h.join("bin/crontab"), fs::Permissions::from_mode(0o700)).unwrap();
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
