use paperhands::{
    report,
    research::{comparison, inventory as inv, plan, runner},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn task(root: &Path, id: &str, title: &str) {
    let args = paperhands::tasks::command()
        .try_get_matches_from([
            "tasks",
            "add",
            "--root",
            root.to_str().unwrap(),
            "--id",
            id,
            "--title",
            title,
            "--kind",
            "research",
        ])
        .unwrap();
    paperhands::tasks::execute(&args).unwrap();
}

fn snapshot(out: &Path) -> Value {
    let value: Value = serde_json::from_slice(&fs::read(out.join("report.json")).unwrap()).unwrap();
    let markdown = fs::read_to_string(out.join("report.md")).unwrap();
    let embedded = markdown
        .split("```json\n")
        .nth(1)
        .unwrap()
        .split("\n```")
        .next()
        .unwrap();
    assert_eq!(serde_json::from_str::<Value>(embedded).unwrap(), value);
    value
}

fn assert_catalog_pagination_scope(value: &Value) {
    assert_eq!(value["frontier"]["catalog_fully_paginated"], true);
    assert!(value["frontier"].get("all_evidence_inspected").is_none());
    let limitations = value["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line.as_str().unwrap())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    for scope in [
        "native",
        "catalog",
        "pagination",
        "source review",
        "success",
        "financial validation",
    ] {
        assert!(
            limitations.contains(scope),
            "missing {scope}: {limitations}"
        );
    }
}

#[test]
fn public_schema_redacts_titles_preserves_counts_and_mirrors_markdown() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    paperhands::desk::initialize(&root).unwrap();
    task(
        &root,
        "private-id",
        "Review /Users/private/key https://private.invalid/secret .kb/private 0123456789abcdef0123456789abcdef sk-supersecret Bearer tokenvalue",
    );
    let out = temp.path().join("public");
    report::write(&root, &out).unwrap();
    let value = snapshot(&out);
    let mut keys = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "automation",
            "binary_version",
            "experiments",
            "frontier",
            "generated_at_ms",
            "limitations",
            "schema",
            "tasks",
            "visibility",
            "zoom"
        ]
    );
    assert_eq!(value["schema"], "paperhands.report/v1");
    assert_eq!(value["binary_version"], paperhands::VERSION);
    assert_eq!(value["visibility"], "public-allowlist");
    assert_eq!(value["frontier"]["notes_total"], 0);
    assert_eq!(value["frontier"]["tasks_total"], 1);
    assert_eq!(value["frontier"]["tasks_ready"], 1);
    assert!(value["experiments"].as_array().unwrap().is_empty());
    assert_catalog_pagination_scope(&value);
    assert_eq!(
        value["tasks"][0],
        json!({"title":"Review","state":"open","ready":true,"depends_count":0,"updated_at_ms":value["tasks"][0]["updated_at_ms"]})
    );
    assert_eq!(
        value["automation"],
        json!({"schedules":[],"missed_slots":0,"model_or_evaluator_started":false})
    );
    let frontier = value["frontier"].as_object().unwrap();
    assert_eq!(
        frontier.keys().map(String::as_str).collect::<Vec<_>>(),
        vec![
            "catalog_fully_paginated",
            "gaps_count",
            "memory_coverage",
            "note_kinds",
            "notes_total",
            "questions_open",
            "task_states",
            "tasks_ready",
            "tasks_total"
        ]
    );
    for file in ["report.json", "report.md"] {
        let text = fs::read_to_string(out.join(file)).unwrap();
        for secret in [
            "private-id",
            "/Users",
            "private.invalid",
            ".kb/",
            "0123456789abcdef",
            "sk-supersecret",
            "tokenvalue",
        ] {
            assert!(!text.contains(secret), "{file}: {secret}");
        }
    }
    // A remaining forbidden token refuses publication, keeping both prior files.
    let before = [
        fs::read(out.join("report.json")).unwrap(),
        fs::read(out.join("report.md")).unwrap(),
    ];
    task(&root, "refuse", "Bare Bearer");
    assert!(report::write(&root, &out).is_err());
    assert_eq!(
        before,
        [
            fs::read(out.join("report.json")).unwrap(),
            fs::read(out.join("report.md")).unwrap()
        ]
    );
    assert!(report::write(&root, &root.join("memory/new-output")).is_err());
    assert!(!root.join("memory/new-output").exists());
}

fn attempt(root: &Path, scratch: &Path, id: &str, executable: &str, obstruct: bool) -> PathBuf {
    let mut bundles = Vec::new();
    for role in ["input", "evaluator", "config"] {
        let source = scratch.join(role);
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("closed.json"), b"{\"closed\":true}").unwrap();
        let inventory = inv::scan(
            &source,
            role,
            if role == "input" {
                Some("closed.json".into())
            } else {
                None
            },
        )
        .unwrap();
        let inventory_path = scratch.join(format!("{role}.json"));
        fs::write(&inventory_path, serde_json::to_vec(&inventory).unwrap()).unwrap();
        bundles.push(json!({"name":role,"role":role,"source":source,"inventory":inventory_path,"sha256":inv::digest(&inventory_path).unwrap()}));
    }
    let mut p:plan::Plan=serde_json::from_value(json!({"schema_version":1,"id":id,"question":"private hypothesis","data_use":"development","bundles":bundles,"external_files":[{"path":executable,"role":"executable","sha256":plan::executable_digest(Path::new(executable)).unwrap()}],"commands":[{"argv":[executable],"cwd":"{work}"}],"env":{"PRIVATE":"secret-environment"},"limits":{"wall_time_ms":2000,"cpu_time_seconds":2,"stdout_bytes":4096,"stderr_bytes":4096,"artifact_bytes":65536,"artifact_files":20},"required_artifacts":[]})).unwrap();
    if obstruct {
        let python = "/usr/bin/python3";
        p.external_files.push(serde_json::from_value(json!({"path":python,"role":"executable","sha256":plan::executable_digest(Path::new(python)).unwrap()})).unwrap());
        p.commands.insert(0,serde_json::from_value(json!({"argv":[python,"-c","from pathlib import Path; Path('../evidence/step-001').write_text('retained preparation failure')"],"cwd":"{work}"})).unwrap());
    }
    let path = scratch.join(format!("{id}.plan.json"));
    inv::json(&path, &p).unwrap();
    plan::create(&path, root).unwrap();
    let directory = root.join("experiments").join(id);
    runner::run(&directory, None, None).unwrap();
    directory
}

#[test]
fn only_current_bound_reviews_are_public_and_failed_evidence_is_not_promoted() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    paperhands::desk::initialize(&root).unwrap();
    let reviewed = attempt(
        &root,
        temp.path(),
        "private-reviewed",
        "/usr/bin/false",
        false,
    );
    attempt(
        &root,
        temp.path(),
        "private-unreviewed",
        "/usr/bin/true",
        false,
    );
    let runner_failed = attempt(
        &root,
        temp.path(),
        "private-runner-failed",
        "/usr/bin/true",
        true,
    );
    assert_eq!(
        runner::inspect(&runner_failed).unwrap()["process_status"],
        "runner_failed"
    );
    let note = temp.path().join("review-note");
    fs::write(&note, "private correspondence sk-never-publish").unwrap();
    comparison::review(&reviewed, "retire", &note).unwrap();
    comparison::review(&runner_failed, "inconclusive", &note).unwrap();
    let out = temp.path().join("public");
    report::write(&root, &out).unwrap();
    let value = snapshot(&out);
    assert_catalog_pagination_scope(&value);
    let rows = value["experiments"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    let failed = rows
        .iter()
        .find(|row| row["process_status"] == "failed")
        .unwrap();
    assert_eq!(failed["state"], "retired");
    assert_eq!(failed["promotion"], "none");
    assert_eq!(failed.as_object().unwrap().len(), 13);
    assert!(
        rows.iter()
            .any(|row| row["process_status"] == "runner_failed"
                && row["evidence_status"] == "invalid")
    );
    for private in [
        "private-reviewed",
        "private-unreviewed",
        "secret-environment",
        "private correspondence",
        "sk-never-publish",
        "/bin/",
    ] {
        assert!(!value.to_string().contains(private));
    }
    let before = fs::read(out.join("report.json")).unwrap();
    let mut review: Value = inv::read_sealed(&reviewed.join("review.json")).unwrap();
    review["result_sha256"] = json!("f".repeat(64));
    fs::write(
        reviewed.join("review.json"),
        serde_json::to_vec_pretty(&review).unwrap(),
    )
    .unwrap();
    fs::write(
        reviewed.join("review.sha256"),
        inv::digest(&reviewed.join("review.json")).unwrap(),
    )
    .unwrap();
    assert!(report::write(&root, &out).is_err());
    assert_eq!(fs::read(out.join("report.json")).unwrap(), before);
}

#[test]
fn full_task_counts_precede_newest_first_cap_and_mcp_runs_the_same_report() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    paperhands::desk::initialize(&root).unwrap();
    let note = paperhands::memory::command()
        .try_get_matches_from([
            "memory",
            "add",
            "--root",
            root.to_str().unwrap(),
            "--kind",
            "source",
            "--title",
            "Unreviewed source",
            "--text",
            "A source assertion awaiting substantive review.",
        ])
        .unwrap();
    paperhands::memory::execute(&note).unwrap();
    for i in 0..104 {
        task(&root, &format!("task-{i:03}"), &format!("Research {i}"));
    }
    std::thread::sleep(std::time::Duration::from_millis(2));
    task(&root, "last", "Newest research");
    let out = temp.path().join("public");
    report::write(&root, &out).unwrap();
    let value = snapshot(&out);
    // Empty execution catalogs paginate fully even when research notes await review.
    assert_eq!(value["frontier"]["notes_total"], 1);
    assert_eq!(value["frontier"]["note_kinds"]["source"], 1);
    assert!(value["experiments"].as_array().unwrap().is_empty());
    assert!(
        !value
            .to_string()
            .contains("A source assertion awaiting substantive review.")
    );
    assert_catalog_pagination_scope(&value);
    assert_eq!(value["frontier"]["tasks_total"], 105);
    assert_eq!(value["frontier"]["tasks_ready"], 105);
    assert_eq!(value["tasks"].as_array().unwrap().len(), 100);
    assert_eq!(value["tasks"][0]["title"], "Newest research");
    let mcp_out = temp.path().join("mcp-public");
    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"report-fixture","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"paperhands","arguments":{"argv":["report","--root",root,"--out",mcp_out]}}}),
    ];
    let mut child = Command::new(env!("CARGO_BIN_EXE_paperhands"))
        .arg("mcp")
        .env("PAPERHANDS_SKILLS_DIR", temp.path().join("skill"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    for request in requests {
        writeln!(input, "{request}").unwrap();
    }
    drop(input);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let responses = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[1]["result"]["isError"], false);
    let mut actual = snapshot(&mcp_out);
    assert_eq!(actual["frontier"]["notes_total"], 1);
    assert!(actual["experiments"].as_array().unwrap().is_empty());
    assert_catalog_pagination_scope(&actual);
    let mut expected = value;
    actual.as_object_mut().unwrap().remove("generated_at_ms");
    expected.as_object_mut().unwrap().remove("generated_at_ms");
    actual["zoom"]
        .as_object_mut()
        .unwrap()
        .remove("generated_at_ms");
    expected["zoom"]
        .as_object_mut()
        .unwrap()
        .remove("generated_at_ms");
    assert_eq!(actual, expected);
}
