//! Owned native-format fixtures only: no evaluator, transport or provider is run.
use paperhands::{
    research::{inventory as inv, plan},
    zoom::{self, Thresholds},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const HOUR: u64 = 3_600_000;

fn root(home: &Path) -> PathBuf {
    let root = home.join("store");
    paperhands::desk::initialize(&root).unwrap();
    root
}
fn note(root: &Path, at: u64) -> String {
    // Produce one genuine native note locally, then preserve its exact projection
    // and vectors while selecting explicit receipt clocks for these owned fixtures.
    static NOTE: OnceLock<Value> = OnceLock::new();
    let mut value = NOTE
        .get_or_init(|| {
            let home = tempfile::tempdir().unwrap();
            let seed = home.path().join("seed");
            paperhands::memory::initialize(&seed).unwrap();
            let args = paperhands::memory::command()
                .try_get_matches_from([
                    "memory",
                    "add",
                    "--root",
                    seed.to_str().unwrap(),
                    "--kind",
                    "source",
                    "--title",
                    "Synthetic zoom receipt",
                    "--source",
                    "fixture:zoom-native",
                    "--text",
                    "A retained synthetic source observation for bounded loop diagnostics.",
                ])
                .unwrap();
            let added = paperhands::memory::execute(&args).unwrap();
            let id = added["record"]["id"].as_str().unwrap();
            inv::read(&seed.join("memory").join(format!("{id}.json"))).unwrap()
        })
        .clone();
    value["recorded_at_ms"] = at.into();
    let bytes = serde_json::to_vec_pretty(&value).unwrap();
    let id = inv::hash(&bytes);
    let directory = if root.join("memory/volumes-v1.json").exists() {
        let args = paperhands::memory::command()
            .try_get_matches_from(["memory", "volumes", "--root", root.to_str().unwrap()])
            .unwrap();
        let volumes = paperhands::memory::execute(&args).unwrap();
        root.join("memory-volumes")
            .join(volumes["active_volume"].as_str().unwrap())
    } else {
        root.join("memory")
    };
    fs::write(directory.join(format!("{id}.json")), bytes).unwrap();
    id
}
fn native_task(root: &Path, id: &str, events: &[(u64, &str, &str)]) -> String {
    let dir = root.join("tasks").join(id);
    fs::create_dir(&dir).unwrap();
    let mut previous = None;
    for (sequence, (at, state, note)) in events.iter().enumerate() {
        let value = json!({"schema_version":1,"id":id,"title":"Synthetic task","kind":"research","depends_on":[],"sequence":sequence,"previous":previous,"state":state,"note":note,"recorded_at_ms":at});
        previous =
            Some(inv::sealed_json(&dir.join(format!("event-{sequence:06}.json")), &value).unwrap());
    }
    format!("task:{id}@{}", previous.unwrap())
}
fn history(times: &[u64]) -> Value {
    json!({"complete":true,"wakes":times.iter().enumerate().map(|(slot,at)|json!({"id":format!("synthetic/{slot}"),"schedule_sha256":"a".repeat(64),"slot":slot,"claimed_at_ms":at,"result":null})).collect::<Vec<_>>(),"gaps":[]})
}
fn inspect(root: &Path, since: Option<u64>, thresholds: &Thresholds, wakes: &Value) -> Value {
    zoom::inspect_with_automation(root, since, thresholds, wakes).unwrap()
}
fn signal<'a>(zoom: &'a Value, kind: &str) -> &'a Value {
    zoom["signals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["kind"] == kind)
        .unwrap()
}
fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut found = BTreeMap::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            for (name, bytes) in files(&path) {
                found.insert(path.file_name().unwrap().into(), vec![]);
                found.insert(PathBuf::from(path.file_name().unwrap()).join(name), bytes);
            }
        } else {
            found.insert(path.file_name().unwrap().into(), fs::read(path).unwrap());
        }
    }
    found
}
fn reseal(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    fs::write(path.with_extension("sha256"), inv::digest(path).unwrap()).unwrap();
}
fn completed_fixture(
    root: &Path,
    scratch: &Path,
    id: &str,
    created: u64,
    finished: u64,
) -> PathBuf {
    let mut bundles = Vec::new();
    for role in ["input", "evaluator", "config"] {
        let source = scratch.join(role);
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("closed.json"), b"{\"synthetic\":true}").unwrap();
        let inventory = inv::scan(
            &source,
            role,
            (role == "input").then(|| "closed.json".into()),
        )
        .unwrap();
        let path = scratch.join(format!("{role}.json"));
        inv::json(&path, &inventory).unwrap();
        bundles.push(json!({"name":role,"role":role,"source":source,"inventory":path,"sha256":inv::digest(&path).unwrap()}));
    }
    let executable = "/usr/bin/true";
    let value = json!({"schema_version":1,"id":id,"question":"Synthetic retained process record; never executed","data_use":"development","bundles":bundles,"external_files":[{"path":executable,"role":"executable","sha256":plan::executable_digest(Path::new(executable)).unwrap()}],"commands":[{"argv":[executable],"cwd":"{work}"}],"env":{},"limits":{"wall_time_ms":1000,"cpu_time_seconds":1,"stdout_bytes":1024,"stderr_bytes":1024,"artifact_bytes":1024,"artifact_files":4},"required_artifacts":[]});
    let source = scratch.join("fixture-plan.json");
    inv::json(&source, &value).unwrap();
    plan::create(&source, root).unwrap();
    let dir = root.join("experiments").join(id);
    let mut frozen: Value = inv::read_sealed(&dir.join("frozen.json")).unwrap();
    frozen["created_at_ms"] = created.into();
    reseal(&dir.join("frozen.json"), &frozen);
    // plan::create reserves the identity; runner::run normally creates these.
    // Build only the synthetic retained layout, without running that evaluator.
    for directory in ["work", "output", "evidence"] {
        fs::create_dir(dir.join(directory)).unwrap();
    }
    let result = json!({"id":id,"process_status":"failed","evidence_status":"invalid","finished_at_ms":finished,"wall_time_ms":0,"steps":[],"not_started_commands":1,"gaps":["Explicit synthetic fixture; no actual process execution"],"output_inventory":inv::scan(&dir.join("output"),"output",None).unwrap(),"process_inventory":inv::scan(&dir.join("evidence"),"process",None).unwrap(),"plan_sha256":frozen["plan_sha256"],"exit_code":3,"financial_validation":"synthetic lifecycle fixture only; no execution or financial evidence","promotion":"none"});
    inv::sealed_json(&dir.join("result.json"), &result).unwrap();
    dir
}

#[test]
fn repeated_blocked_events_are_same_task_exact_reason_and_not_imported_prose() {
    let home = tempfile::tempdir().unwrap();
    let root = root(home.path());
    let now = inv::now();
    note(&root, now - 1000);
    native_task(
        &root,
        "one",
        &[
            (now - 10_000, "open", ""),
            (now - 9000, "blocked", "Same admission condition"),
            (now - 8000, "blocked", "Same admission condition"),
            (now - 7000, "blocked", "Same admission condition"),
        ],
    );
    native_task(
        &root,
        "two",
        &[
            (now - 10_000, "open", ""),
            (now - 9000, "blocked", "Same admission condition"),
            (now - 8000, "blocked", "Same admission condition"),
        ],
    );
    let before = files(&root);
    let t = Thresholds::default();
    let value = inspect(&root, None, &t, &history(&[]));
    assert_eq!(signal(&value, "repeated_refusal")["count"], 3);
    assert_eq!(
        signal(&value, "repeated_refusal")["same_reason"],
        "Same admission condition"
    );
    assert_eq!(signal(&value, "repeated_refusal")["task"], "one");
    assert_eq!(signal(&value, "repeated_refusal")["lifecycle_errors"], 0);
    assert_eq!(value["verdict"], "stalled");
    assert_eq!(value["verdict_rule"], "paperhands.zoom.verdict/v1");
    assert_eq!(value["signals_fired"], json!(["repeated_refusal"]));
    assert!(!value["questions"].as_array().unwrap().is_empty());
    assert_eq!(files(&root), before);
    let higher = Thresholds {
        refusal_count: 4,
        ..t
    };
    assert_eq!(
        inspect(&root, None, &higher, &history(&[]))["verdict"],
        "moving"
    );
    let newer = inspect(&root, Some(now - 6000), &higher, &history(&[]));
    assert_eq!(signal(&newer, "repeated_refusal")["count"], 0);
    assert_eq!(newer["thresholds"]["refusal_count"], 4);
    assert_eq!(newer["since_ms"], now - 6000);
}

#[test]
fn terminal_heads_resolve_refusals_and_reopening_counts_retained_events_again() {
    for terminal in ["retired", "done"] {
        let home = tempfile::tempdir().unwrap();
        let root = root(home.path());
        let now = inv::now();
        note(&root, now - 1000);
        native_task(
            &root,
            "gate",
            &[
                (now - 10_000, "open", "Synthetic gate"),
                (now - 9000, "blocked", "Same synthetic admission refusal"),
                (now - 8000, "blocked", "Same synthetic admission refusal"),
                (now - 7000, "blocked", "Same synthetic admission refusal"),
            ],
        );
        let original = files(&root);
        let thresholds = Thresholds::default();
        let blocked = inspect(&root, None, &thresholds, &history(&[]));
        let refusal = signal(&blocked, "repeated_refusal");
        assert_eq!(refusal["count"], 3);
        assert_eq!(refusal["fired"], true);
        assert_eq!(blocked["verdict"], "stalled");
        assert_eq!(files(&root), original);
        let blocked_evidence = refusal["evidence"].clone();

        for state in [terminal, "open", "active", "blocked"] {
            let args = paperhands::tasks::command()
                .try_get_matches_from([
                    "tasks",
                    "update",
                    "--root",
                    root.to_str().unwrap(),
                    "--id",
                    "gate",
                    "--state",
                    state,
                    "--note-text",
                    "Explicit synthetic disposition; preserve original refusal records.",
                ])
                .unwrap();
            paperhands::tasks::execute(&args).unwrap();
            let before = files(&root);
            for (path, bytes) in &original {
                assert_eq!(before.get(path), Some(bytes), "original {path:?}");
            }
            let value = inspect(&root, None, &thresholds, &history(&[]));
            let refusal = signal(&value, "repeated_refusal");
            let eligible = matches!(state, "open" | "blocked");
            assert_eq!(refusal["count"], if eligible { 3 } else { 0 }, "{state}");
            assert_eq!(refusal["fired"], eligible, "{state}");
            assert_eq!(value["coverage_complete"], true);
            assert_eq!(
                value["verdict"],
                if eligible { "stalled" } else { "moving" }
            );
            assert_eq!(value["verdict_rule"], "paperhands.zoom.verdict/v1");
            if eligible {
                assert_eq!(refusal["evidence"], blocked_evidence);
            } else {
                assert_eq!(refusal["evidence"], json!([]));
            }
            let resolved = &refusal["resolved_refusals"];
            assert_eq!(resolved["count"], if state == terminal { 3 } else { 0 });
            if state == terminal {
                let mut expected = blocked_evidence
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|id| id.as_str().unwrap().to_owned())
                    .collect::<Vec<_>>();
                expected.sort();
                assert_eq!(resolved["evidence"], json!(expected));
            } else {
                assert_eq!(resolved["evidence"], json!([]));
            }
            assert_eq!(files(&root), before);
        }
    }
}

#[test]
fn resolved_refusals_preserve_all_eligible_evidence_without_masking_active_groups() {
    let home = tempfile::tempdir().unwrap();
    let root = root(home.path());
    let now = inv::now();
    note(&root, now - 1000);
    native_task(
        &root,
        "resolved-a",
        &[
            (now - 51 * HOUR, "open", "Synthetic gate"),
            (now - 50 * HOUR, "blocked", "Outside the lookback"),
            (now - 9000, "blocked", "Repeated resolved refusal"),
            (now - 8000, "blocked", "Repeated resolved refusal"),
            (now - 7000, "blocked", "Repeated resolved refusal"),
            (now - 6000, "retired", "Premise deliberately closed"),
        ],
    );
    native_task(
        &root,
        "resolved-b",
        &[
            (now - 9000, "open", "Synthetic gate"),
            (now - 8000, "blocked", "Another resolved refusal"),
            (now - 7000, "blocked", ""),
            (now - 6000, "blocked", "   "),
            (now - 5000, "done", "Gate deliberately completed"),
        ],
    );
    native_task(
        &root,
        "eligible-open",
        &[
            (now - 6000, "open", "Synthetic gate"),
            (now - 5000, "blocked", "Eligible refusal"),
            (now - 4000, "blocked", "Eligible refusal"),
            (now - 3000, "open", "Reopened without erasing events"),
        ],
    );
    let mut resolved_ids = [
        ("resolved-a", 2),
        ("resolved-a", 3),
        ("resolved-a", 4),
        ("resolved-b", 1),
    ]
    .into_iter()
    .map(|(id, sequence)| {
        let path = root
            .join("tasks")
            .join(id)
            .join(format!("event-{sequence:06}.json"));
        format!("task:{id}@{}", inv::digest(&path).unwrap())
    })
    .collect::<Vec<_>>();
    resolved_ids.sort();
    let original = files(&root);
    let thresholds = Thresholds::default();
    let value = inspect(&root, None, &thresholds, &history(&[]));
    let refusal = signal(&value, "repeated_refusal");
    assert_eq!(refusal["count"], 2);
    assert_eq!(refusal["task"], "eligible-open");
    assert_eq!(refusal["fired"], false);
    assert_eq!(
        refusal["resolved_refusals"],
        json!({"count":4,"evidence":resolved_ids})
    );
    assert_eq!(value["verdict"], "moving");
    assert_eq!(files(&root), original);

    native_task(
        &root,
        "eligible-blocked",
        &[
            (now - 6000, "open", "Synthetic gate"),
            (now - 5000, "blocked", "Currently blocked refusal"),
            (now - 4000, "blocked", "Currently blocked refusal"),
            (now - 3000, "blocked", "Currently blocked refusal"),
        ],
    );
    let before = files(&root);
    for (path, bytes) in &original {
        assert_eq!(before.get(path), Some(bytes));
    }
    let blocked = inspect(&root, None, &thresholds, &history(&[]));
    let refusal = signal(&blocked, "repeated_refusal");
    assert_eq!(refusal["count"], 3);
    assert_eq!(refusal["task"], "eligible-blocked");
    assert_eq!(refusal["fired"], true);
    assert_eq!(
        refusal["resolved_refusals"],
        json!({"count":4,"evidence":resolved_ids})
    );
    assert_eq!(blocked["verdict"], "stalled");
    let recent = inspect(&root, Some(now - 1000), &thresholds, &history(&[]));
    let refusal = signal(&recent, "repeated_refusal");
    assert_eq!(refusal["count"], 0);
    assert_eq!(
        refusal["resolved_refusals"],
        json!({"count":0,"evidence":[]})
    );
    assert_eq!(refusal["fired"], false);
    assert_eq!(files(&root), before);
}

#[test]
fn current_unreviewed_backlog_and_recording_age_are_not_lost_at_window_boundary() {
    let home = tempfile::tempdir().unwrap();
    let root = root(home.path());
    let now = inv::now();
    note(&root, now - 80 * HOUR);
    let dir = completed_fixture(
        &root,
        home.path(),
        "old-output",
        now - 80 * HOUR,
        now - 72 * HOUR,
    );
    let t = Thresholds::default();
    let value = inspect(&root, None, &t, &history(&[]));
    assert_eq!(value["window_ms"], 48 * HOUR);
    assert_eq!(signal(&value, "unreviewed_output")["count"], 1);
    assert!(
        signal(&value, "unreviewed_output")["oldest_age_ms"]
            .as_u64()
            .unwrap()
            >= 72 * HOUR
    );
    assert_eq!(signal(&value, "unreviewed_output")["fired"], true);
    assert_eq!(signal(&value, "no_new_evidence")["fired"], true);
    assert_eq!(value["verdict"], "stalled");
    // A native lifecycle failure remains separate from the blocked-task reason count.
    assert_eq!(signal(&value, "repeated_refusal")["count"], 0);
    assert_eq!(signal(&value, "repeated_refusal")["lifecycle_errors"], 0); // completion predates --since
    let relaxed = Thresholds {
        unreviewed_age_ms: 96 * HOUR,
        ..t
    };
    assert_eq!(
        signal(
            &inspect(&root, None, &relaxed, &history(&[])),
            "unreviewed_output"
        )["fired"],
        false
    );
    let frozen: Value = inv::read_sealed(&dir.join("frozen.json")).unwrap();
    inv::sealed_json(&dir.join("review.json"),&json!({"schema_version":1,"disposition":"inconclusive","note":"Explicit synthetic review","reviewed_at_ms":now-1000,"plan_sha256":frozen["plan_sha256"],"result_sha256":inv::digest(&dir.join("result.json")).unwrap(),"automatic_promotion":false})).unwrap();
    note(&root, now - 500);
    let reviewed = inspect(&root, None, &Thresholds::default(), &history(&[]));
    assert_eq!(signal(&reviewed, "unreviewed_output")["count"], 0);
    assert_eq!(signal(&reviewed, "no_new_evidence")["fired"], false);
    assert_eq!(reviewed["verdict"], "moving");
}

#[test]
fn malformed_native_result_status_is_unknown_instead_of_zero_lifecycle_errors() {
    let home = tempfile::tempdir().unwrap();
    let root = root(home.path());
    let now = inv::now();
    note(&root, now - 1000);
    let directory = completed_fixture(&root, home.path(), "status-fixture", now - 5000, now - 2000);
    let result_path = directory.join("result.json");
    let original: Value = inv::read_sealed(&result_path).unwrap();
    let thresholds = Thresholds::default();
    let valid = inspect(&root, None, &thresholds, &history(&[]));
    assert_eq!(signal(&valid, "repeated_refusal")["lifecycle_errors"], 1);
    assert_eq!(valid["verdict"], "moving");

    for status in [None, Some(json!(7)), Some(json!("queued"))] {
        let mut malformed = original.clone();
        if let Some(status) = status {
            malformed["process_status"] = status;
        } else {
            malformed.as_object_mut().unwrap().remove("process_status");
        }
        // Both the bytes and their seal agree: status validation must still fail.
        reseal(&result_path, &malformed);
        let before = files(&root);
        let value = inspect(&root, None, &thresholds, &history(&[]));
        assert!(signal(&value, "repeated_refusal")["lifecycle_errors"].is_null());
        assert!(signal(&value, "unreviewed_output")["count"].is_null());
        assert!(value["verdict"].is_null());
        assert_eq!(value["coverage_complete"], false);
        assert!(value["gaps"].to_string().contains("process_status"));
        assert_eq!(files(&root), before);
    }
}

#[test]
fn spent_claims_include_ambiguous_outcomes_but_only_real_state_changes_count() {
    let home = tempfile::tempdir().unwrap();
    let root = root(home.path());
    let now = inv::now();
    note(&root, now - 72 * HOUR);
    native_task(
        &root,
        "same-state",
        &[
            (now - 2000, "open", "first"),
            (now - 1000, "open", "different note, same state"),
        ],
    );
    let t = Thresholds::default();
    let claims = history(&[now - 3000, now - 2000]);
    let value = inspect(&root, None, &t, &claims);
    assert_eq!(signal(&value, "spent_without_change")["wakes"], 2);
    assert_eq!(
        signal(&value, "spent_without_change")["task_state_changes"],
        0
    );
    assert_eq!(signal(&value, "spent_without_change")["fired"], true);
    assert_eq!(value["verdict"], "stalled");
    let outside = inspect(&root, Some(now - 1500), &t, &claims);
    assert_eq!(signal(&outside, "spent_without_change")["wakes"], 0);
    assert_eq!(outside["verdict"], "drifting");
    note(&root, now - 500);
    assert_eq!(inspect(&root, None, &t, &claims)["verdict"], "drifting");
    native_task(
        &root,
        "changed",
        &[
            (now - 2000, "open", "first"),
            (now - 1000, "active", "a deliberate transition"),
        ],
    );
    let changed = inspect(&root, None, &t, &claims);
    assert_eq!(
        signal(&changed, "spent_without_change")["task_state_changes"],
        1
    );
    assert_eq!(signal(&changed, "spent_without_change")["fired"], false);
    assert_eq!(changed["verdict"], "moving");
    let unknown = inspect(
        &root,
        None,
        &t,
        &json!({"complete":false,"error":"fixture cannot verify claims"}),
    );
    assert!(signal(&unknown, "spent_without_change")["wakes"].is_null());
    assert!(unknown["verdict"].is_null());
}

#[test]
fn monoculture_requires_explicit_retirement_links_and_preserves_missing_mapping_unknown() {
    let home = tempfile::tempdir().unwrap();
    let root = root(home.path());
    let now = inv::now();
    let note = note(&root, now - 1000);
    let t = Thresholds::default();
    let mut retired = Vec::new();
    for i in 0..5 {
        retired.push(native_task(
            &root,
            &format!("retired-{i}"),
            &[
                (now - 5000, "open", ""),
                (
                    now - 1000,
                    "retired",
                    "Synthetic retirement; no economic conclusion",
                ),
            ],
        ));
    }
    let missing = inspect(&root, None, &t, &history(&[]));
    assert!(signal(&missing, "monoculture")["fired"].is_null());
    assert!(missing["verdict"].is_null());
    assert_eq!(missing["coverage_complete"], false);
    for from in retired {
        let args = paperhands::graph::command()
            .try_get_matches_from([
                "graph",
                "link",
                "--root",
                root.to_str().unwrap(),
                "--from",
                &from,
                "--to",
                &format!("memory:{note}"),
                "--relation",
                "relates",
                "--evidence-text",
                "Explicit synthetic retirement-to-note association",
            ])
            .unwrap();
        paperhands::graph::execute(&args).unwrap();
    }
    let before = files(&root);
    let linked = inspect(&root, None, &t, &history(&[]));
    assert_eq!(signal(&linked, "monoculture")["retired"], 5);
    assert_eq!(signal(&linked, "monoculture")["clusters"], 1);
    assert_eq!(signal(&linked, "monoculture")["dominant_share"], 1.0);
    assert_eq!(signal(&linked, "monoculture")["fired"], true);
    assert_eq!(linked["verdict"], "drifting");
    assert_eq!(files(&root), before);
    let larger = Thresholds {
        monoculture_retirements: 6,
        ..t
    };
    let insufficient = inspect(&root, None, &larger, &history(&[]));
    assert_eq!(signal(&insufficient, "monoculture")["fired"], false);
    assert_eq!(insufficient["verdict"], "moving");
}

#[test]
fn referenced_retirement_population_survives_native_rollover_and_ignores_unlinked_notes() {
    let home = tempfile::tempdir().unwrap();
    let root = root(home.path());
    let now = inv::now();
    let primary = (0..1024)
        .map(|offset| note(&root, now - 20_000 - offset))
        .collect::<Vec<_>>();
    let args = paperhands::memory::command()
        .try_get_matches_from(["memory", "volumes", "--root", root.to_str().unwrap()])
        .unwrap();
    let volumes = paperhands::memory::execute(&args).unwrap();
    assert_eq!(volumes["notes_total"], 1024);
    let args = paperhands::memory::command()
        .try_get_matches_from([
            "memory",
            "rollover",
            "--root",
            root.to_str().unwrap(),
            "--expected-head",
            "none",
            "--expected-note-set",
            volumes["note_set_sha256"].as_str().unwrap(),
        ])
        .unwrap();
    paperhands::memory::execute(&args).unwrap();
    let active = note(&root, now - 1000);
    let selected = [format!("memory:{}", primary[0]), format!("memory:{active}")];
    let retired = (0..5)
        .map(|index| {
            native_task(
                &root,
                &format!("volume-retired-{index}"),
                &[
                    (now - 5000, "open", ""),
                    (
                        now - 1000,
                        "retired",
                        "Explicit synthetic retirement, no economic claim",
                    ),
                ],
            )
        })
        .collect::<Vec<_>>();
    let thresholds = Thresholds::default();
    let unmapped = inspect(&root, None, &thresholds, &history(&[]));
    assert!(signal(&unmapped, "monoculture")["fired"].is_null());
    assert!(unmapped["verdict"].is_null());
    for (index, from) in retired.iter().enumerate() {
        let args = paperhands::graph::command()
            .try_get_matches_from([
                "graph",
                "link",
                "--root",
                root.to_str().unwrap(),
                "--from",
                from,
                "--to",
                &selected[index % selected.len()],
                "--relation",
                "relates",
                "--evidence-text",
                "Explicit synthetic retirement-to-note association",
            ])
            .unwrap();
        paperhands::graph::execute(&args).unwrap();
    }
    let before = files(&root);
    let linked = inspect(&root, None, &thresholds, &history(&[]));
    let concentration = signal(&linked, "monoculture");
    assert_eq!(concentration["population"], "referenced-notes");
    assert_eq!(concentration["retired"], 5);
    assert_eq!(concentration["referenced_notes"], 2);
    assert_eq!(concentration["clusters"], 1);
    assert_eq!(concentration["dominant_share"], 1.0);
    assert_eq!(concentration["fired"], true);
    assert_eq!(linked["verdict"], "drifting");
    assert_eq!(linked["uses_inference"], false);
    assert_eq!(files(&root), before);
    for offset in 0..16 {
        note(&root, now - 500 - offset);
    }
    let original = files(&root);
    let expanded = inspect(&root, None, &thresholds, &history(&[]));
    for key in [
        "population",
        "retired",
        "referenced_notes",
        "referenced_note_ids",
        "snapshot_sha256",
        "clusters",
        "dominant_share",
        "fired",
    ] {
        assert_eq!(
            signal(&expanded, "monoculture")[key],
            concentration[key],
            "{key}"
        );
    }
    let args = paperhands::graph::command()
        .try_get_matches_from([
            "graph",
            "clusters",
            "--root",
            root.to_str().unwrap(),
            "--max-notes",
            "1024",
        ])
        .unwrap();
    let error = paperhands::graph::execute(&args).unwrap_err().to_string();
    assert!(error.contains("clustering cap"), "{error}");
    assert_eq!(files(&root), original);
    let args = paperhands::memory::command()
        .try_get_matches_from(["memory", "volumes", "--root", root.to_str().unwrap()])
        .unwrap();
    let volumes = paperhands::memory::execute(&args).unwrap();
    assert_eq!(volumes["notes_total"], 1041);
    assert_eq!(volumes["planned_volumes"], 2);
    let referenced_file = root
        .join("memory-volumes/volume-000001")
        .join(format!("{active}.json"));
    let retained = fs::read(&referenced_file).unwrap();
    fs::remove_file(&referenced_file).unwrap();
    let missing = inspect(&root, None, &thresholds, &history(&[]));
    assert!(signal(&missing, "monoculture")["fired"].is_null());
    assert!(missing["verdict"].is_null());
    assert!(!referenced_file.exists());
    fs::write(referenced_file, retained).unwrap();
    assert_eq!(files(&root), original);
}

#[test]
fn untouched_open_task_backlog_uses_final_receipt_all_history_and_explicit_components() {
    let home = tempfile::tempdir().unwrap();
    let root = root(home.path());
    let now = inv::now();
    note(&root, now - 1000);
    let old = native_task(
        &root,
        "old-open",
        &[
            (now - 80 * HOUR, "open", "First receipt"),
            (now - 60 * HOUR, "open", "Same state, latest receipt"),
        ],
    );
    for (id, state) in [
        ("active", "active"),
        ("blocked", "blocked"),
        ("done", "done"),
        ("retired", "retired"),
    ] {
        native_task(
            &root,
            id,
            &[
                (now - 80 * HOUR, "open", ""),
                (
                    now - 60 * HOUR,
                    state,
                    "Historical transition excludes the task",
                ),
            ],
        );
    }
    native_task(
        &root,
        "reopened",
        &[
            (now - 90 * HOUR, "open", ""),
            (now - 80 * HOUR, "active", ""),
            (now - 60 * HOUR, "open", "Reopening does not erase history"),
        ],
    );
    native_task(
        &root,
        "young",
        &[(now - 5 * 60_000, "open", "Young receipt")],
    );
    native_task(
        &root,
        "same-state-reset",
        &[
            (now - 80 * HOUR, "open", "Original old receipt"),
            (
                now - 2 * 60_000,
                "open",
                "Latest same-state receipt resets age",
            ),
        ],
    );
    let old_scratch = home.path().join("old-output-fixture");
    completed_fixture(
        &root,
        &old_scratch,
        "old-output",
        now - 80 * HOUR,
        now - 72 * HOUR,
    );
    let young_scratch = home.path().join("young-output-fixture");
    completed_fixture(
        &root,
        &young_scratch,
        "young-output",
        now - 5 * 60_000,
        now - 60_000,
    );
    let thresholds = Thresholds {
        unreviewed_age_ms: HOUR,
        ..Thresholds::default()
    };
    let before = files(&root);
    let value = inspect(&root, Some(now - 60_000), &thresholds, &history(&[]));
    let backlog = signal(&value, "unreviewed_output");
    assert_eq!(backlog["native_experiments"]["count"], 2);
    assert_eq!(backlog["open_untouched_tasks"]["count"], 1);
    assert_eq!(backlog["open_untouched_tasks"]["evidence"], json!([old]));
    assert_eq!(backlog["count"], 3);
    assert_eq!(backlog["evidence"].as_array().unwrap().len(), 3);
    assert!(
        backlog["native_experiments"]["oldest_age_ms"]
            .as_u64()
            .unwrap()
            >= 72 * HOUR
    );
    assert!(
        backlog["open_untouched_tasks"]["oldest_age_ms"]
            .as_u64()
            .unwrap()
            >= 60 * HOUR
    );
    assert_eq!(
        backlog["oldest_age_ms"],
        backlog["native_experiments"]["oldest_age_ms"]
    );
    assert_eq!(backlog["fired"], true);
    assert_eq!(files(&root), before);
    let all_history = inspect(&root, None, &thresholds, &history(&[]));
    assert_eq!(
        signal(&all_history, "unreviewed_output")["native_experiments"]["count"],
        2
    );
    assert_eq!(
        signal(&all_history, "unreviewed_output")["open_untouched_tasks"]["evidence"],
        backlog["open_untouched_tasks"]["evidence"]
    );
    assert_eq!(signal(&all_history, "unreviewed_output")["count"], 3);
    assert_eq!(files(&root), before);
}

#[test]
fn unreviewed_backlog_preserves_known_components_when_another_collection_is_unknown() {
    let home = tempfile::tempdir().unwrap();
    let root = root(home.path());
    let now = inv::now();
    note(&root, now - 1000);
    let old = native_task(
        &root,
        "old-open",
        &[(now - 72 * HOUR, "open", "Owned old open receipt")],
    );
    let directory = completed_fixture(
        &root,
        home.path(),
        "output",
        now - 80 * HOUR,
        now - 60 * HOUR,
    );
    let thresholds = Thresholds::default();
    let result_path = directory.join("result.json");
    let result_bytes = fs::read(&result_path).unwrap();
    fs::write(&result_path, b"{}").unwrap();
    let before = files(&root);
    let bad_experiment = inspect(&root, None, &thresholds, &history(&[]));
    let backlog = signal(&bad_experiment, "unreviewed_output");
    assert!(backlog["native_experiments"]["count"].is_null());
    assert_eq!(backlog["open_untouched_tasks"]["count"], 1);
    assert_eq!(backlog["open_untouched_tasks"]["evidence"], json!([old]));
    assert!(
        backlog["count"].is_null()
            && backlog["oldest_age_ms"].is_null()
            && backlog["fired"].is_null()
    );
    assert!(bad_experiment["verdict"].is_null());
    assert_eq!(files(&root), before);
    fs::write(&result_path, result_bytes).unwrap();
    let event_path = root.join("tasks/old-open/event-000000.json");
    fs::write(&event_path, b"{}").unwrap();
    let before = files(&root);
    let bad_task = inspect(&root, None, &thresholds, &history(&[]));
    let backlog = signal(&bad_task, "unreviewed_output");
    assert_eq!(backlog["native_experiments"]["count"], 1);
    assert!(
        backlog["native_experiments"]["oldest_age_ms"]
            .as_u64()
            .unwrap()
            >= 60 * HOUR
    );
    assert!(backlog["open_untouched_tasks"]["count"].is_null());
    assert!(
        backlog["count"].is_null()
            && backlog["oldest_age_ms"].is_null()
            && backlog["fired"].is_null()
    );
    assert!(bad_task["verdict"].is_null());
    assert_eq!(
        signal(&bad_task, "repeated_refusal")["resolved_refusals"],
        json!({"count":null,"evidence":[]})
    );
    assert_eq!(files(&root), before);
}

#[test]
fn empty_or_corrupt_required_history_stays_null_even_with_another_firing_signal() {
    let home = tempfile::tempdir().unwrap();
    let root = root(home.path());
    let now = inv::now();
    let t = Thresholds::default();
    fs::write(
        root.join("imported-history.json"),
        json!({"recorded_at_ms":now,"claims":"three refusals and five reviewed retirements"})
            .to_string(),
    )
    .unwrap();
    let empty = zoom::inspect(&root, None, &t).unwrap();
    assert!(signal(&empty, "no_new_evidence")["age_ms"].is_null());
    assert!(empty["verdict"].is_null());
    let id = note(&root, now - 1000);
    native_task(
        &root,
        "blocked",
        &[
            (now - 10_000, "open", ""),
            (now - 9000, "blocked", "Same reason"),
            (now - 8000, "blocked", "Same reason"),
            (now - 7000, "blocked", "Same reason"),
        ],
    );
    fs::write(root.join("memory").join(format!("{id}.json")), b"{}").unwrap();
    let before = files(&root);
    let corrupt = inspect(&root, None, &t, &history(&[]));
    assert_eq!(signal(&corrupt, "repeated_refusal")["fired"], true);
    assert!(signal(&corrupt, "no_new_evidence")["age_ms"].is_null());
    assert!(corrupt["verdict"].is_null());
    assert_eq!(corrupt["coverage_complete"], false);
    assert_eq!(files(&root), before);
}

#[test]
fn bounded_scans_and_invalid_thresholds_refuse_partial_success() {
    let home = tempfile::tempdir().unwrap();
    let root = root(home.path());
    note(&root, inv::now() - 1000);
    let t = Thresholds::default();
    let oversized =
        json!({"complete":true,"wakes":vec![json!({"id":"one","claimed_at_ms":0});4097]});
    let value = inspect(&root, None, &t, &oversized);
    assert!(signal(&value, "spent_without_change")["wakes"].is_null());
    assert!(value["verdict"].is_null());
    fs::create_dir(root.join("experiments")).unwrap();
    for i in 0..1025 {
        fs::create_dir(root.join("experiments").join(format!("attempt-{i}"))).unwrap();
    }
    let value = inspect(&root, None, &t, &history(&[]));
    assert!(signal(&value, "unreviewed_output")["count"].is_null());
    assert!(value["verdict"].is_null());
    assert!(value["gaps"].to_string().contains("exceeds 1024"));
    assert!(
        zoom::inspect_with_automation(&root, Some(inv::now() + 60_000), &t, &history(&[])).is_err()
    );
    assert!(
        Thresholds {
            cluster_threshold: f32::NAN,
            ..t.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        Thresholds {
            spent_wakes: 0,
            ..t
        }
        .validate()
        .is_err()
    );
}
