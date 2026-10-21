use paperhands::{research::inventory as inv, runtime};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn invoke(args: &[&str]) -> Value {
    let result = runtime::invoke(
        &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        &json!({}),
    );
    assert_eq!(result.exit_code, 0, "{}", result.value);
    result.value["data"].clone()
}

fn stalled(root: &Path) {
    paperhands::desk::initialize(root).unwrap();
    let root = root.to_str().unwrap();
    invoke(&[
        "memory",
        "add",
        "--root",
        root,
        "--kind",
        "source",
        "--title",
        "Owned fixture observation",
        "--text",
        "Synthetic retained observation; no market claim.",
    ]);
    invoke(&[
        "tasks",
        "add",
        "--root",
        root,
        "--id",
        "same-gate",
        "--title",
        "Review admission premise",
        "--kind",
        "research",
    ]);
    for _ in 0..3 {
        invoke(&[
            "tasks",
            "update",
            "--root",
            root,
            "--id",
            "same-gate",
            "--state",
            "blocked",
            "--note-text",
            "The same retained fixture gate is refused.",
        ]);
    }
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, String> {
    let mut result = BTreeMap::new();
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    inv::digest(&path).unwrap(),
                );
            }
        }
    }
    result
}

fn binary(home: &Path, root: &Path) -> Command {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("/usr/bin/sandbox-exec");
        command.arg("-p").arg(format!(
            "(version 1)(allow default)(deny network*)(deny file-write* (subpath {}))",
            serde_json::to_string(root.to_str().unwrap()).unwrap()
        ));
        command.arg(env!("CARGO_BIN_EXE_paperhands"));
        command
    };
    #[cfg(not(target_os = "macos"))]
    let mut command = Command::new(env!("CARGO_BIN_EXE_paperhands"));
    command
        .current_dir(home)
        .env("HOME", home)
        .env("PAPERHANDS_SKILLS_DIR", home.join("isolated-skill"))
        .env_remove("PAPERHANDS_WORKSPACE");
    command
}

#[test]
fn cli_human_ask_report_and_stdio_mcp_share_the_retained_stall() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("store");
    stalled(&root);
    let before = snapshot(&root);
    let output = binary(home.path(), &root)
        .args(["zoom", "--root", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let zoom: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(zoom["data"]["schema"], "paperhands.zoom/v1");
    assert_eq!(zoom["data"]["verdict"], "stalled");
    assert!(
        zoom["data"]["signals_fired"]
            .as_array()
            .unwrap()
            .contains(&json!("repeated_refusal"))
    );
    let human = binary(home.path(), &root)
        .args(["--human", "zoom", "--root", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(human.status.success());
    let text = String::from_utf8(human.stdout).unwrap();
    assert!(text.starts_with("Zoom: stalled\n"), "{text}");
    assert!(
        text.contains("repeated_refusal: fired; evidence 3"),
        "{text}"
    );
    assert!(
        text.lines()
            .filter(|line| line.contains("; evidence "))
            .count()
            == 5
    );
    assert!(text.len() < 2500, "human zoom dumped its private ledger");

    let answer = invoke(&[
        "ask",
        "--root",
        root.to_str().unwrap(),
        "What should I do next?",
    ]);
    assert!(
        answer["answer"]
            .as_str()
            .unwrap()
            .starts_with("Zoom: stalled.")
    );
    assert_eq!(answer["zoom"]["verdict"], "stalled");
    assert_eq!(answer["executed_actions"], false);
    assert_eq!(answer["suggested_actions"][0]["argv"][1], "zoom");

    let out = home.path().join("public");
    paperhands::report::write(&root, &out).unwrap();
    let public: Value =
        serde_json::from_slice(&fs::read(out.join("report.json")).unwrap()).unwrap();
    assert_eq!(public["zoom"]["verdict"], "stalled");
    assert_eq!(public["zoom"].as_object().unwrap().len(), 3);
    assert!(public["zoom"].get("evidence").is_none());
    assert!(!public.to_string().contains("same-gate"));
    assert!(
        !public
            .to_string()
            .contains("The same retained fixture gate")
    );

    let mut child = binary(home.path(), &root)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    for request in [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"zoom-fixture","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"paperhands","arguments":{"argv":["zoom","--root",root]}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"paperhands_ask","arguments":{"root":root,"question":"What should I do next?"}}}),
    ] {
        writeln!(input, "{request}").unwrap();
    }
    drop(input);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let responses = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(responses.len(), 3);
    let actual = &responses[1]["result"]["structuredContent"]["data"];
    assert_eq!(actual["verdict"], zoom["data"]["verdict"]);
    assert_eq!(actual["signals_fired"], zoom["data"]["signals_fired"]);
    assert_eq!(actual["thresholds"], zoom["data"]["thresholds"]);
    assert!(
        responses[2]["result"]["structuredContent"]["data"]["answer"]
            .as_str()
            .unwrap()
            .starts_with("Zoom: stalled.")
    );
    assert_eq!(snapshot(&root), before);
}

#[test]
fn human_unknown_is_not_zero_or_a_state_dump() {
    let value = json!({"schema":"paperhands.zoom/v1","verdict":null,"signals":[{"kind":"no_new_evidence","fired":null,"evidence":null}],"questions":[]});
    assert_eq!(
        paperhands::human(&value),
        "Zoom: unknown\nno_new_evidence: unknown; evidence unknown"
    );
}
