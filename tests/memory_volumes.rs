use paperhands::research::inventory as inv;
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn command(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_paperhands"));
    cmd.env("PAPERHANDS_SKILLS_DIR", home.join("skill"))
        .env_remove("PAPERHANDS_WORKSPACE");
    cmd
}
fn cli(home: &Path, args: &[&str]) -> (i32, Value) {
    let out = command(home).args(args).output().unwrap();
    let bytes = if out.stdout.is_empty() {
        &out.stderr
    } else {
        &out.stdout
    };
    (
        out.status.code().unwrap(),
        serde_json::from_slice(bytes)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(bytes))),
    )
}
fn seed(home: &Path, count: usize) -> (PathBuf, String, Value) {
    let root = home.join("store");
    let r = root.to_str().unwrap();
    assert_eq!(cli(home, &["init", "--root", r]).0, 0);
    let (code, note) = cli(
        home,
        &[
            "memory",
            "add",
            "--root",
            r,
            "--kind",
            "source",
            "--title",
            "Fixture",
            "--text",
            "Retain unknown holdings",
        ],
    );
    assert_eq!(code, 0, "{note}");
    let id = note["data"]["record"]["id"].as_str().unwrap().to_owned();
    let record: Value =
        serde_json::from_slice(&fs::read(root.join("memory").join(format!("{id}.json"))).unwrap())
            .unwrap();
    fill(&root.join("memory"), &record, count - 1, 1);
    (root, id, record)
}
fn fill(dir: &Path, record: &Value, count: usize, start: u64) {
    for i in 0..count {
        let mut v = record.clone();
        v["recorded_at_ms"] = json!(start + i as u64);
        let bytes = serde_json::to_vec_pretty(&v).unwrap();
        inv::exclusive(&dir.join(format!("{}.json", inv::hash(&bytes))), &bytes).unwrap();
    }
}
fn files(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    let mut result = std::collections::BTreeMap::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            for (name, bytes) in files(&path) {
                result.insert(PathBuf::from(path.file_name().unwrap()).join(name), bytes);
            }
        } else {
            result.insert(path.file_name().unwrap().into(), fs::read(path).unwrap());
        }
    }
    result
}
fn rollover(home: &Path, root: &Path, expected: &str) -> Value {
    let (status, volumes) = cli(
        home,
        &["memory", "volumes", "--root", root.to_str().unwrap()],
    );
    assert_eq!(status, 0, "{volumes}");
    let notes = volumes["data"]["note_set_sha256"].as_str().unwrap();
    let (code, value) = cli(
        home,
        &[
            "memory",
            "rollover",
            "--root",
            root.to_str().unwrap(),
            "--expected-head",
            expected,
            "--expected-note-set",
            notes,
        ],
    );
    assert_eq!(code, 0, "{value}");
    value["data"].clone()
}
fn reseal(path: &Path, value: &Value) {
    let bytes = serde_json::to_vec_pretty(value).unwrap();
    fs::write(path, &bytes).unwrap();
    fs::write(path.with_extension("sha256"), inv::hash(&bytes)).unwrap();
}

#[test]
fn rollover_crosses_original_cap_without_rewriting_notes_and_keeps_global_citations() {
    let temp = tempfile::tempdir().unwrap();
    let h = temp.path();
    let (root, id, _) = seed(h, 1024);
    let r = root.to_str().unwrap();
    let original = files(&root);
    let (_, catalog) = cli(h, &["memory", "volumes", "--root", r]);
    let initial_notes = catalog["data"]["note_set_sha256"].as_str().unwrap();
    let (code, refusal) = cli(
        h,
        &[
            "memory",
            "add",
            "--root",
            r,
            "--kind",
            "source",
            "--title",
            "Overflow",
            "--text",
            "No publication",
        ],
    );
    assert_eq!(code, 2, "{refusal}");
    assert_eq!(files(&root), original);
    let (code, dry) = cli(
        h,
        &[
            "memory",
            "rollover",
            "--root",
            r,
            "--expected-head",
            "none",
            "--expected-note-set",
            initial_notes,
            "--dry-run",
        ],
    );
    assert_eq!(code, 0, "{dry}");
    assert_eq!(files(&root), original);
    let event = rollover(h, &root, "none");
    let head = event["catalog_head"].as_str().unwrap();
    for (name, bytes) in &original {
        assert_eq!(&fs::read(root.join(name)).unwrap(), bytes);
    }
    assert_eq!(
        cli(
            h,
            &["memory", "rollover", "--root", r, "--expected-head", "none"]
        )
        .0,
        2
    );
    let (code, added) = cli(
        h,
        &[
            "memory",
            "add",
            "--root",
            r,
            "--kind",
            "direction",
            "--title",
            "Fixture",
            "--text",
            "Retain unknown holdings",
        ],
    );
    assert_eq!(code, 0, "{added}");
    assert_eq!(added["data"]["volume"], "volume-000001");
    let new_id = added["data"]["record"]["id"].as_str().unwrap();
    for key in [&id, new_id] {
        assert_eq!(cli(h, &["memory", "read", "--root", r, key]).0, 0);
    }
    let (code, found) = cli(
        h,
        &[
            "memory",
            "search",
            "--root",
            r,
            "unknown holdings",
            "--limit",
            "20",
        ],
    );
    assert_eq!(code, 0, "{found}");
    let found = &found["data"];
    assert_eq!(found["notes_verified"], 1025);
    assert_eq!(found["memory_coverage"]["catalog_head"], head);
    assert_eq!(found["memory_coverage"]["planned_volumes"], 2);
    assert_eq!(found["coverage"]["ranking"], "global-semantic-cosine");
    assert_eq!(found["coverage"]["query_embeddings"], 1);
    let mut expected = Vec::new();
    for dir in [
        root.join("memory"),
        root.join("memory-volumes/volume-000001"),
    ] {
        for e in fs::read_dir(dir).unwrap() {
            let name = e.unwrap().file_name().into_string().unwrap();
            if let Some(id) = name.strip_suffix(".json").filter(|id| inv::valid_hash(id)) {
                expected.push(id.to_owned());
            }
        }
    }
    expected.sort();
    let actual = found["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["source"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(actual, expected[..20]);
    for item in found["items"].as_array().unwrap() {
        let argv = item["read_argv"]
            .as_array()
            .unwrap()
            .iter()
            .skip(1)
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(cli(h, &argv).0, 0);
    }
    let (code, desk) = cli(h, &["desk", "--root", r]);
    assert_eq!(code, 0, "{desk}");
    assert_eq!(desk["data"]["desk"]["notes_total"], 1025);
    assert_eq!(desk["data"]["desk"]["memory_coverage"]["complete"], true);
    assert_eq!(desk["data"]["desk"]["gaps"], json!([]));
    let (code, ask) = cli(h, &["ask", "--root", r, "What is our focus?"]);
    assert_eq!(code, 0, "{ask}");
    assert_eq!(ask["data"]["focus"]["record"]["id"], new_id);
    let before = files(&root);
    assert_eq!(cli(h, &["init", "--root", r]).0, 0);
    assert_eq!(files(&root), before);
}

#[test]
fn bad_or_interrupted_catalogs_and_changed_members_refuse_complete_coverage() {
    for fault in [
        "changed",
        "extra",
        "missing",
        "link",
        "missing-lock",
        "parent",
        "path",
        "marker",
        "partial",
        "duplicate",
        "model",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let h = temp.path();
        let (root, id, record) = seed(h, 2);
        let r = root.to_str().unwrap();
        rollover(h, &root, "none");
        let path = root.join("memory").join(format!("{id}.json"));
        let history = root.join("memory-volumes/rollover-000001.json");
        match fault {
            "changed" => fs::write(&path, b"{}").unwrap(),
            "extra" => fill(&root.join("memory"), &record, 1, 999),
            "missing" => {
                fs::remove_file(&path).unwrap();
            }
            "link" => {
                fs::remove_file(&path).unwrap();
                std::os::unix::fs::symlink(h.join("outside"), &path).unwrap();
            }
            "missing-lock" => {
                fs::remove_file(root.join("memory-volumes/volume-000001/memory.lock")).unwrap();
            }
            "parent" | "path" => {
                let mut v: Value = serde_json::from_slice(&fs::read(&history).unwrap()).unwrap();
                if fault == "parent" {
                    v["previous"] = json!("1".repeat(64));
                } else {
                    v["next_volume"] = json!("../escape");
                }
                reseal(&history, &v);
            }
            "marker" => {
                fs::remove_file(root.join("memory/volumes-v1.json")).unwrap();
            }
            "partial" => {
                fs::remove_file(history.with_extension("sha256")).unwrap();
            }
            "duplicate" => {
                fs::copy(
                    &path,
                    root.join("memory-volumes/volume-000001")
                        .join(format!("{id}.json")),
                )
                .unwrap();
            }
            "model" => {
                let mut v = record.clone();
                v["search"]["model"] = json!({});
                let bytes = serde_json::to_vec_pretty(&v).unwrap();
                fs::write(
                    root.join("memory-volumes/volume-000001")
                        .join(format!("{}.json", inv::hash(&bytes))),
                    bytes,
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        let before = files_without_links(&root);
        let (code, error) = cli(h, &["memory", "search", "--root", r, "unknown inventory"]);
        assert_eq!(code, 2, "{fault}: {error}");
        let (code, desk) = cli(h, &["desk", "--root", r]);
        assert_eq!(code, 0);
        assert!(
            desk["data"]["desk"]["notes_total"].is_null(),
            "{fault}: {desk}"
        );
        let (code, ask) = cli(h, &["ask", "--root", r, "unknown inventory"]);
        assert_eq!(code, 0);
        assert!(
            !ask["data"]["context"]["gaps"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{fault}: {ask}"
        );
        assert_eq!(files_without_links(&root), before);
    }
}
fn files_without_links(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    let mut result = std::collections::BTreeMap::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        let name = PathBuf::from(path.file_name().unwrap());
        if path.is_symlink() {
            result.insert(name, b"symlink".to_vec());
        } else if path.is_dir() {
            for (tail, bytes) in files_without_links(&path) {
                result.insert(name.join(tail), bytes);
            }
        } else {
            result.insert(name, fs::read(path).unwrap());
        }
    }
    result
}

#[test]
fn concurrent_append_and_rollover_keep_capacity_and_expected_heads_exclusive() {
    let temp = tempfile::tempdir().unwrap();
    let h = temp.path();
    let (root, _, record) = seed(h, 1);
    let r = root.to_str().unwrap();
    let (_, catalog) = cli(h, &["memory", "volumes", "--root", r]);
    let note_set = catalog["data"]["note_set_sha256"].as_str().unwrap();
    let args = [
        "memory",
        "rollover",
        "--root",
        r,
        "--expected-head",
        "none",
        "--expected-note-set",
        note_set,
    ];
    let first = command(h)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let second = command(h)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut codes = [
        first.wait_with_output().unwrap().status.code().unwrap(),
        second.wait_with_output().unwrap().status.code().unwrap(),
    ];
    codes.sort();
    assert_eq!(codes, [0, 2]);
    fill(&root.join("memory-volumes/volume-000001"), &record, 1023, 1);
    let args = [
        "memory",
        "add",
        "--root",
        r,
        "--kind",
        "source",
        "--title",
        "Last slot",
        "--text",
        "Owned concurrent fixture",
    ];
    let first = command(h)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let second = command(h)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut codes = [
        first.wait_with_output().unwrap().status.code().unwrap(),
        second.wait_with_output().unwrap().status.code().unwrap(),
    ];
    codes.sort();
    assert_eq!(codes, [0, 2]);
    let (code, catalog) = cli(h, &["memory", "volumes", "--root", r]);
    assert_eq!(code, 0, "{catalog}");
    assert_eq!(catalog["data"]["active_notes"], 1024);
    assert_eq!(catalog["data"]["notes_total"], 1025);
}

#[test]
fn desk_attention_scales_past_full_graph_bound_and_mcp_keeps_complete_memory() {
    let temp = tempfile::tempdir().unwrap();
    let h = temp.path();
    let (root, id, record) = seed(h, 1);
    let r = root.to_str().unwrap();
    assert_eq!(
        cli(
            h,
            &[
                "tasks",
                "add",
                "--root",
                r,
                "--id",
                "question",
                "--title",
                "Inspect evidence",
                "--kind",
                "research"
            ]
        )
        .0,
        0
    );
    let (_, tasks) = cli(h, &["tasks", "list", "--root", r]);
    let task = tasks["data"]["items"][0]["event_sha256"].as_str().unwrap();
    let from = format!("memory:{id}");
    let to = format!("task:question@{task}");
    let (code, link) = cli(
        h,
        &[
            "graph",
            "link",
            "--root",
            r,
            "--from",
            &from,
            "--to",
            &to,
            "--relation",
            "informs",
            "--priority",
            "0.7",
            "--evidence-text",
            "Declared fixture attention",
        ],
    );
    assert_eq!(code, 0, "{link}");
    fill(&root.join("memory"), &record, 1023, 1);
    let mut head = "none".to_owned();
    for i in 1..=4 {
        head = rollover(h, &root, &head)["catalog_head"]
            .as_str()
            .unwrap()
            .to_owned();
        fill(
            &root.join(format!("memory-volumes/volume-{i:06}")),
            &record,
            1024,
            1 + i as u64 * 2000,
        );
    }
    let (code, desk) = cli(h, &["desk", "--root", r]);
    assert_eq!(code, 0, "{desk}");
    assert_eq!(desk["data"]["desk"]["notes_total"], 5120);
    assert_eq!(desk["data"]["desk"]["gaps"], json!([]));
    assert_eq!(
        desk["data"]["desk"]["frontier"][0]["attention"]["priority"],
        0.7
    );
    assert_eq!(cli(h, &["graph", "inspect", "--root", r]).0, 2);
    let before = files(&root);
    #[cfg(not(target_os = "macos"))]
    let mut cmd = command(h);
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut cmd = Command::new("/usr/bin/sandbox-exec");
        cmd.args([
            "-p",
            &format!(
                "(version 1)(allow default)(deny network*)(deny file-write* (subpath {}))",
                serde_json::to_string(root.canonicalize().unwrap().to_str().unwrap()).unwrap()
            ),
            env!("CARGO_BIN_EXE_paperhands"),
        ]);
        cmd.env("PAPERHANDS_SKILLS_DIR", h.join("skill"))
            .env_remove("PAPERHANDS_WORKSPACE");
        cmd
    };
    let mut child = cmd
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for v in [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","clientInfo":{"name":"fixture","version":"1"},"capabilities":{}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"paperhands_ask","arguments":{"question":"What should I do next?","root":r}}}),
    ] {
        writeln!(stdin, "{v}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).lines().last().unwrap()).unwrap();
    let data = &v["result"]["structuredContent"]["data"];
    assert_eq!(v["result"]["isError"], false, "{v}");
    assert_eq!(data["notes"]["notes_verified"], 5120, "{v}");
    assert_eq!(data["notes"]["memory_coverage"]["planned_volumes"], 5);
    assert_eq!(data["context"]["gaps"], json!([]));
    assert_eq!(files(&root), before);
}

#[test]
fn bounded_volume_count_and_changed_active_note_set_refuse_without_writes() {
    let temp = tempfile::tempdir().unwrap();
    let h = temp.path();
    let (root, _, record) = seed(h, 1);
    let r = root.to_str().unwrap();
    let (_, before) = cli(h, &["memory", "volumes", "--root", r]);
    let notes = before["data"]["note_set_sha256"].as_str().unwrap();
    fill(&root.join("memory"), &record, 1, 40);
    let snapshot = files(&root);
    let (code, refusal) = cli(
        h,
        &[
            "memory",
            "rollover",
            "--root",
            r,
            "--expected-head",
            "none",
            "--expected-note-set",
            notes,
        ],
    );
    assert_eq!(code, 2, "{refusal}");
    assert_eq!(files(&root), snapshot);
    let mut head = "none".to_owned();
    for i in 1..=15 {
        head = rollover(h, &root, &head)["catalog_head"]
            .as_str()
            .unwrap()
            .to_owned();
        fill(
            &root.join(format!("memory-volumes/volume-{i:06}")),
            &record,
            1,
            100 + i,
        );
    }
    let (_, catalog) = cli(h, &["memory", "volumes", "--root", r]);
    assert_eq!(catalog["data"]["planned_volumes"], 16);
    let notes = catalog["data"]["note_set_sha256"].as_str().unwrap();
    let before = files(&root);
    assert_eq!(
        cli(
            h,
            &[
                "memory",
                "rollover",
                "--root",
                r,
                "--expected-head",
                &head,
                "--expected-note-set",
                notes
            ]
        )
        .0,
        2
    );
    assert_eq!(files(&root), before);
}

#[test]
fn combined_indexed_text_budget_refuses_append_before_publication() {
    let temp = tempfile::tempdir().unwrap();
    let h = temp.path();
    let (root, _, short) = seed(h, 1);
    let r = root.to_str().unwrap();
    // Long retained section titles repeat in each source passage. This compact
    // valid source reaches the indexed-text budget in two bounded volumes.
    let text = format!("# {}\n{}", "h".repeat(470), "x ".repeat(7800));
    let source = h.join("long.txt");
    fs::write(&source, &text).unwrap();
    let args = [
        "memory",
        "add",
        "--root",
        r,
        "--kind",
        "source",
        "--title",
        "Long",
        "--file",
        source.to_str().unwrap(),
    ];
    let (code, added) = cli(h, &args);
    assert_eq!(code, 0, "{added}");
    let id = added["data"]["record"]["id"].as_str().unwrap();
    let record: Value =
        serde_json::from_slice(&fs::read(root.join("memory").join(format!("{id}.json"))).unwrap())
            .unwrap();
    let weight = |v: &Value| {
        v["search"]["documents"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["title"].as_str().unwrap().len() * 2 + d["text"].as_str().unwrap().len() + 2)
            .sum::<usize>()
    };
    let count = (32 * 1024 * 1024 - weight(&short)) / weight(&record);
    assert!(
        count > 2 && count < 2000,
        "unexpected fixture size: {count}"
    );
    let first = count / 2;
    fill(&root.join("memory"), &record, first - 1, 100);
    rollover(h, &root, "none");
    fill(
        &root.join("memory-volumes/volume-000001"),
        &record,
        count - first,
        20000,
    );
    let (code, catalog) = cli(h, &["memory", "volumes", "--root", r]);
    assert_eq!(code, 0, "{catalog}");
    let bytes = catalog["data"]["indexed_text_bytes"].as_u64().unwrap() as usize;
    assert!(bytes <= 32 * 1024 * 1024 && bytes + weight(&record) > 32 * 1024 * 1024);
    let before = files(&root);
    let (code, refusal) = cli(h, &args);
    assert_eq!(code, 2, "{refusal}");
    assert!(
        refusal["error"]["message"]
            .as_str()
            .unwrap()
            .contains("combined memory")
    );
    assert_eq!(files(&root), before);
    let (code, search) = cli(h, &["memory", "search", "--root", r, "retained source"]);
    assert_eq!(code, 0, "{search}");
    assert_eq!(search["data"]["notes_verified"], count + 1);
    assert_eq!(search["data"]["memory_coverage"]["complete"], true);
}
