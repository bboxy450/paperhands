use paperhands::research::inventory::hash;
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn write(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn cli(root: &Path, args: &[&str]) -> (i32, Value) {
    let p = Command::new(env!("CARGO_BIN_EXE_paperhands"))
        .current_dir(root)
        .env("HOME", root)
        .env("PAPERHANDS_SKILLS_DIR", root.join("skill"))
        .env_remove("PAPERHANDS_WORKSPACE")
        .env_remove("TYPESAFE_API_KEY")
        .args(args)
        .output()
        .unwrap();
    let bytes = if p.stdout.is_empty() {
        &p.stderr
    } else {
        &p.stdout
    };
    (
        p.status.code().unwrap(),
        serde_json::from_slice(bytes)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(bytes))),
    )
}
fn fixture(root: &Path) -> (Value, Value) {
    let cases=(0..5).flat_map(|fold|(0..4).map(move |i|json!({"id":format!("c-{fold}-{i}"),"group":format!("event-{fold}"),"fold":fold,"text":"Synthetic claim; no real trace.","failure":i>=2}))).collect::<Vec<_>>();
    let data = json!({"schema":"paperhands.judge-dataset/v1","id":"fixture","data_use":"synthetic","folds":5,"cases":cases});
    write(&root.join("dataset.json"), &data);
    let scores = json!({"schema":"paperhands.judge-scores/v1","id":"low-range","dataset_sha256":hash(&fs::read(root.join("dataset.json")).unwrap()),"provider":"synthetic","model":"low-range","rubric":{"id":"source","failure":"Is the source unsupported?"},"pricing":null,"cases":cases.iter().map(|c|json!({"id":c["id"],"status":"scored","score":if c["failure"]==true{0.4}else{0.1}})).collect::<Vec<_>>()});
    write(&root.join("scores.json"), &scores);
    (data, scores)
}
fn analyze(root: &Path, out: &str) -> (i32, Value) {
    cli(
        root,
        &[
            "judge",
            "analyze",
            "--dataset",
            "dataset.json",
            "--scores",
            "scores.json",
            "--out",
            out,
            "--min-precision",
            "0.8",
            "--min-recall",
            "0.8",
        ],
    )
}

#[test]
fn saved_score_cli_and_mcp_are_offline_and_retained_views_detect_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let (code, result) = analyze(root, "analysis");
    assert_eq!(code, 0, "{result}");
    let model = &result["data"]["result"]["models"][0];
    assert_eq!(model["default_validation"]["recall"], 0.0);
    assert_eq!(model["tuned_validation"]["f_beta"], 1.0);
    assert!(
        model["folds"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["training_selected_threshold"] == 0.4)
    );
    assert!(model["estimated_cost_usd"].is_null());
    let (code, view) = cli(root, &["view", "judge", "--dir", "analysis"]);
    assert_eq!(code, 0, "{view}");
    assert_eq!(view["data"]["kind"], "judge");
    assert_eq!(
        view["data"]["datasets"][1]["rows"]
            .as_array()
            .unwrap()
            .len(),
        101
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_paperhands"))
        .current_dir(root)
        .env("PAPERHANDS_SKILLS_DIR", root.join("skill"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    for request in [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"judge-test","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"paperhands_view","arguments":{"kind":"judge","dir":"analysis"}}}),
    ] {
        writeln!(input, "{request}").unwrap();
    }
    drop(input);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let responses = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str::<Value>(s).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(responses[1]["result"]["isError"], false);
    assert_eq!(
        responses[1]["result"]["structuredContent"]["data"]["datasets"],
        view["data"]["datasets"]
    );
    let original = fs::read(root.join("analysis/evidence/result.json")).unwrap();
    assert_eq!(analyze(root, "analysis").0, 2);
    assert_eq!(
        fs::read(root.join("analysis/evidence/result.json")).unwrap(),
        original
    );
    fs::write(root.join("analysis/evidence/scores-00.json"), "{}").unwrap();
    assert_eq!(cli(root, &["judge", "inspect", "--dir", "analysis"]).0, 2);
    assert_eq!(cli(root, &["view", "judge", "--dir", "analysis"]).0, 2);
}

#[test]
fn unsupported_thresholds_and_unknown_cases_are_not_forced_into_success() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let (_, mut scores) = fixture(root);
    for row in scores["cases"].as_array_mut().unwrap() {
        row["score"] = json!(0.4);
    }
    write(&root.join("scores.json"), &scores);
    let (code, result) = analyze(root, "infeasible");
    assert_eq!(code, 0);
    let model = &result["data"]["result"]["models"][0];
    assert_eq!(model["validation_eligible"], false);
    assert!(model["tuned_validation"].is_null());
    assert!(
        model["folds"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["no_qualifying_threshold"] == true)
    );
    scores["cases"][0]["status"] = json!("failed");
    scores["cases"][0]["score"] = Value::Null;
    write(&root.join("scores.json"), &scores);
    let (code, result) = analyze(root, "partial");
    assert_eq!(code, 3);
    assert_eq!(
        result["data"]["result"]["models"][0]["status"],
        "unsupported_incomplete_cases"
    );
    assert_eq!(
        result["data"]["result"]["models"][0]["gaps"][0]["id"],
        "c-0-0"
    );
    assert!(root.join("partial/evidence/report.md").exists());
    assert_eq!(cli(root, &["judge", "inspect", "--dir", "partial"]).0, 0);
}

#[test]
fn invalid_score_sets_and_leaking_groups_refuse_without_reserving_output() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let (data, scores) = fixture(root);
    for (name, mut bad) in [
        ("missing", scores.clone()),
        ("duplicate", scores.clone()),
        ("null", scores.clone()),
        ("range", scores.clone()),
        ("hash", scores.clone()),
    ] {
        match name {
            "missing" => {
                bad["cases"].as_array_mut().unwrap().pop();
            }
            "duplicate" => {
                bad["cases"][1]["id"] = bad["cases"][0]["id"].clone();
            }
            "null" => bad["cases"][0]["score"] = Value::Null,
            "range" => bad["cases"][0]["score"] = json!(1.1),
            "hash" => bad["dataset_sha256"] = json!("0".repeat(64)),
            _ => unreachable!(),
        }
        write(&root.join("scores.json"), &bad);
        assert_eq!(analyze(root, name).0, 2, "{name}");
        assert!(!root.join(name).exists());
    }
    fs::write(
        root.join("scores.json"),
        serde_json::to_string(&scores)
            .unwrap()
            .replacen("0.1", "NaN", 1),
    )
    .unwrap();
    assert_eq!(analyze(root, "nonfinite").0, 2);
    write(&root.join("scores.json"), &scores);
    let mut leak = data.clone();
    leak["cases"][4]["group"] = leak["cases"][0]["group"].clone();
    write(&root.join("dataset.json"), &leak);
    let (code, error) = analyze(root, "leak");
    assert_eq!(code, 2);
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("crosses")
    );
    write(&root.join("dataset.json"), &data);
    assert_eq!(
        cli(
            root,
            &[
                "judge",
                "analyze",
                "--dataset",
                "dataset.json",
                "--scores",
                "scores.json",
                "--out",
                "nan",
                "--min-precision",
                "NaN",
                "--min-recall",
                "0.8"
            ]
        )
        .0,
        2
    );
    #[cfg(unix)]
    {
        fs::remove_file(root.join("dataset.json")).unwrap();
        fs::write(root.join("real.json"), serde_json::to_vec(&data).unwrap()).unwrap();
        std::os::unix::fs::symlink(root.join("real.json"), root.join("dataset.json")).unwrap();
        assert_eq!(analyze(root, "symlink").0, 2);
    }
}

#[test]
fn plan_validation_is_local_and_credentials_are_not_a_dry_run_dependency() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let (data, scores) = fixture(root);
    let mut plan = json!({"schema":"paperhands.judge-plan/v1","id":"run","model":"jev-1.13.0","dataset":{"path":"dataset.json","sha256":hash(&serde_json::to_vec(&data).unwrap())},"rubric":scores["rubric"],"limits":{"max_calls":20,"wall_time_ms":60000,"request_timeout_ms":1000,"max_total_request_bytes":65536,"max_response_bytes":4096,"max_total_response_bytes":65536},"pricing":null});
    write(&root.join("plan.json"), &plan);
    assert_eq!(
        cli(root, &["judge", "validate", "--plan", "plan.json"]).0,
        0
    );
    assert!(!root.join("judges").exists());
    assert_eq!(
        cli(
            root,
            &[
                "judge",
                "run",
                "--plan",
                "plan.json",
                "--root",
                root.to_str().unwrap()
            ]
        )
        .0,
        2
    );
    assert!(!root.join("judges").exists());
    plan["model"] = json!("jev-latest");
    write(&root.join("plan.json"), &plan);
    assert_eq!(
        cli(root, &["judge", "validate", "--plan", "plan.json"]).0,
        2
    );
    plan["model"] = json!("jev-1.13.0");
    plan["dataset"]["path"] = json!("../escape");
    write(&root.join("plan.json"), &plan);
    assert_eq!(
        cli(root, &["judge", "validate", "--plan", "plan.json"]).0,
        2
    );
}
