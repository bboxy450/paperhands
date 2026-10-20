use serde_json::Value;
use std::process::{Command, Output};

fn run(home: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_paperhands"))
        .env("HOME", home)
        .env_remove("PAPERHANDS_SKILLS_DIR")
        .args(args)
        .output()
        .unwrap()
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
    let output = Command::new(env!("CARGO_BIN_EXE_paperhands"))
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
