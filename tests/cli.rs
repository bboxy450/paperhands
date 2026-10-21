use serde_json::Value;
use std::process::{Command, Output};

fn run(home: &std::path::Path, args: &[&str]) -> Output {
    command(home).args(args).output().unwrap()
}
fn command(home: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_paperhands"));
    command
        .current_dir(home)
        .env("HOME", home)
        .env_remove("PAPERHANDS_WORKSPACE")
        .env_remove("PAPERHANDS_SKILLS_DIR");
    command
}

#[test]
fn json_contract_includes_help_version_status_and_errors() {
    let home = tempfile::tempdir().unwrap();
    for args in [
        vec![],
        vec!["--help"],
        vec!["docs", "read", "--help"],
        vec!["help", "docs", "read"],
        vec!["--version"],
        vec!["status"],
        vec!["docs", "list"],
        vec!["docs", "read", "cli"],
    ] {
        let output = run(home.path(), &args);
        assert!(output.status.success(), "{args:?}: {:?}", output);
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["schema"], "paperhands/v1");
        assert_eq!(value["ok"], true);
        assert!(output.stderr.is_empty());
    }
    for args in [
        vec!["buy"],
        vec!["docs", "read"],
        vec!["docs", "read", "unknown"],
        vec!["how", ""],
        vec!["how", "bad\nquery"],
    ] {
        let output = run(home.path(), &args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["ok"], false);
    }
}

#[test]
fn human_is_explicit_and_global() {
    let home = tempfile::tempdir().unwrap();
    for args in [
        vec!["--human", "--help"],
        vec!["docs", "read", "cli", "--human"],
    ] {
        let output = run(home.path(), &args);
        assert!(output.status.success());
        assert!(serde_json::from_slice::<Value>(&output.stdout).is_err());
        assert!(!output.stdout.is_empty());
    }
    let output = run(home.path(), &["--human", "ask", "What should I do next?"]);
    assert!(output.status.success());
    let answer = String::from_utf8(output.stdout).unwrap();
    assert!(
        answer.len() < 2048,
        "human companion leaked the full structured context"
    );
    assert!(!answer.contains("route_candidates:") && !answer.contains("context:"));
}

fn configured_project(home: &std::path::Path, storage: &str) {
    std::fs::write(
        home.join("paperhands.toml"),
        format!(
            "schema_version = 1\n[project]\nid = \"configured-project\"\nname = \"Owned CLI fixture\"\n[storage]\npath = \"{storage}\"\n"
        ),
    )
    .unwrap();
}

#[test]
fn companion_explains_a_configured_missing_store_without_creating_it_and_honors_overrides() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    configured_project(h, "absent/store");
    let configured = h.canonicalize().unwrap().join("absent/store");
    let output = run(h, &["ask", "What should I do next?"]);
    assert!(output.status.success(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["data"]["workspace"], configured.to_str().unwrap());
    assert_eq!(value["data"]["workspace_available"], false);
    assert_eq!(value["data"]["context"], Value::Null);
    assert_eq!(value["data"]["executed_actions"], false);
    assert!(
        value["data"]["answer"]
            .as_str()
            .unwrap()
            .contains("do not have a research store")
    );
    assert!(!h.join("absent").exists());
    let human = run(h, &["--human", "ask", "What should I do next?"]);
    assert!(human.status.success(), "{human:?}");
    assert_eq!(
        String::from_utf8(human.stdout).unwrap().trim(),
        value["data"]["answer"].as_str().unwrap()
    );

    let override_root = h.join("override-missing");
    let output = command(h)
        .env("PAPERHANDS_WORKSPACE", &override_root)
        .args(["ask", "What should I do next?"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["data"]["workspace"], override_root.to_str().unwrap());
    assert_eq!(value["data"]["workspace_available"], false);
    let explicit = h.join("explicit-missing");
    let output = command(h)
        .env("PAPERHANDS_WORKSPACE", &override_root)
        .args([
            "ask",
            "What should I do next?",
            "--root",
            explicit.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["data"]["workspace"], explicit.to_str().unwrap());
    assert_eq!(value["data"]["workspace_available"], false);
    assert!(!configured.exists() && !override_root.exists() && !explicit.exists());
}

#[test]
fn companion_refuses_an_existing_configured_store_with_a_different_identity() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    configured_project(h, "store");
    let root = h.join("store");
    std::fs::create_dir(&root).unwrap();
    paperhands::research::inventory::sealed_json(
        &root.join("project.json"),
        &serde_json::json!({"schema_version":1,"project_id":"different-project"}),
    )
    .unwrap();
    let original = std::fs::read(root.join("project.json")).unwrap();
    let output = run(h, &["ask", "What should I do next?"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("project/store identity mismatch")
    );
    assert_eq!(std::fs::read(root.join("project.json")).unwrap(), original);
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 2);
}

#[cfg(unix)]
#[test]
fn companion_refuses_a_symlinked_configured_store_instead_of_claiming_it_is_missing() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    configured_project(h, "linked-store");
    let target = h.join("target");
    std::fs::create_dir(&target).unwrap();
    std::os::unix::fs::symlink(&target, h.join("linked-store")).unwrap();
    let output = run(h, &["ask", "What should I do next?"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("must not traverse symlinks")
    );
    assert_eq!(std::fs::read_dir(target).unwrap().count(), 0);
}

#[test]
fn first_run_installs_and_every_run_repairs_the_skill() {
    let home = tempfile::tempdir().unwrap();
    assert!(run(home.path(), &["--version"]).status.success());
    let skill = home.path().join(".agents/skills/paperhands/SKILL.md");
    assert_eq!(std::fs::read_to_string(&skill).unwrap(), paperhands::SKILL);
    std::fs::write(&skill, "outdated local edit").unwrap();
    let personal = skill.with_file_name("personal.md");
    std::fs::write(&personal, "keep").unwrap();
    assert!(run(home.path(), &["status"]).status.success());
    assert_eq!(std::fs::read_to_string(&skill).unwrap(), paperhands::SKILL);
    assert_eq!(std::fs::read_to_string(personal).unwrap(), "keep");
    let result: Value = serde_json::from_slice(&run(home.path(), &["skills"]).stdout).unwrap();
    assert_eq!(result["data"]["installation"]["updated"], false);
}

#[test]
fn skill_installation_failure_is_structured() {
    let home = tempfile::tempdir().unwrap();
    let blocked = home.path().join("blocked");
    std::fs::write(&blocked, "file").unwrap();
    let output = command(home.path())
        .env("PAPERHANDS_SKILLS_DIR", &blocked)
        .arg("status")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("skill installation")
    );
}

#[test]
fn semantic_search_returns_real_sources_in_the_selected_scope() {
    let home = tempfile::tempdir().unwrap();
    for (args, kind) in [
        (vec!["how", "what can this version do?"], None),
        (
            vec!["cli", "search", "show installed capabilities"],
            Some("command"),
        ),
        (
            vec!["docs", "search", "automatically refresh agent skills"],
            Some("documentation"),
        ),
    ] {
        let output = run(home.path(), &args);
        assert!(output.status.success(), "{output:?}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["data"]["offline"], true);
        assert_eq!(value["data"]["uses_inference"], true);
        let items = value["data"]["items"].as_array().unwrap();
        assert!(!items.is_empty() && items.len() <= 5);
        for item in items {
            assert!(!item["text"].as_str().unwrap().is_empty());
            if let Some(kind) = kind {
                assert_eq!(item["kind"], kind);
            }
            if item["kind"] == "command" {
                let help = item["help"]
                    .as_str()
                    .unwrap()
                    .split_whitespace()
                    .skip(1)
                    .collect::<Vec<_>>();
                assert!(run(home.path(), &help).status.success());
            } else {
                let source = item["source"].as_str().unwrap();
                let guide = paperhands::GUIDES
                    .iter()
                    .find(|(id, _, _)| *id == source)
                    .unwrap();
                let line = item["line"].as_u64().unwrap() as usize;
                assert!(
                    guide
                        .2
                        .lines()
                        .skip(line - 1)
                        .collect::<Vec<_>>()
                        .join("\n")
                        .contains(item["text"].as_str().unwrap())
                );
            }
        }
    }
}
