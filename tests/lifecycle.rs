#![cfg(unix)]
use paperhands::research::{
    batch, comparison, inventory as inv,
    plan::{self, Plan},
    runner,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

struct Fixture {
    home: tempfile::TempDir,
    base: Plan,
}
impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let root = home.path();
        let mut bundles = Vec::new();
        for (name, role) in [
            ("input", "input"),
            ("evaluator", "evaluator"),
            ("config", "config"),
        ] {
            let source = root.join(name);
            fs::create_dir(&source).unwrap();
            if name == "evaluator" {
                fs::write(
                    source.join("evaluate.py"),
                    include_str!("../examples/fixture_evaluator.py"),
                )
                .unwrap();
            } else {
                fs::write(source.join("report.json"), b"{\"closed\":true}").unwrap();
            }
            let inventory = inv::scan(
                &source,
                name,
                if name == "input" {
                    Some("report.json".into())
                } else {
                    None
                },
            )
            .unwrap();
            let path = root.join(format!("{name}.inventory.json"));
            inv::json(&path, &inventory).unwrap();
            bundles.push(json!({"name":name,"role":role,"source":source,"inventory":path,"sha256":inv::digest(&path).unwrap()}));
        }
        let program = "/usr/bin/python3";
        let base=serde_json::from_value(json!({"schema_version":1,"id":"initial","question":"synthetic test","data_use":"development","bundles":bundles,"external_files":[{"path":program,"role":"executable","sha256":plan::executable_digest(Path::new(program)).unwrap()}],"commands":[{"argv":[program,"{snapshot:evaluator}/evaluate.py","--out","{output}","--mode","ok"],"cwd":"{work}"}],"env":{},"limits":{"wall_time_ms":2000,"cpu_time_seconds":2,"stdout_bytes":4096,"stderr_bytes":4096,"artifact_bytes":1048576,"artifact_files":20},"required_artifacts":["cases.json"],"cases":"cases.json"})).unwrap();
        Self { home, base }
    }
    fn root(&self) -> &Path {
        self.home.path()
    }
    fn store(&self) -> PathBuf {
        self.root().join("store")
    }
    fn plan(&self, id: &str, mode: &str, value: f64) -> Plan {
        let mut p = self.base.clone();
        p.id = id.into();
        p.commands[0].argv[5] = mode.into();
        p.commands[0]
            .argv
            .extend(["--value".into(), value.to_string()]);
        p
    }
    fn create_plan(&self, p: &Plan) -> PathBuf {
        let file = self.root().join(format!("{}.plan.json", p.id));
        inv::json(&file, p).unwrap();
        plan::create(&file, &self.store()).unwrap();
        self.store().join("experiments").join(&p.id)
    }
    fn create(&self, id: &str, mode: &str, value: f64) -> PathBuf {
        self.create_plan(&self.plan(id, mode, value))
    }
}

#[test]
fn exact_argv_frozen_data_review_and_allowlisted_export() {
    let f = Fixture::new();
    let mut p = f.plan("honest", "ok", 1.0);
    let literal = "$HOME $(touch NEVER) ; not-a-shell {literal-json}";
    p.commands[0]
        .argv
        .extend(["--literal".into(), literal.into()]);
    let file = f.root().join("validate.json");
    inv::json(&file, &p).unwrap();
    assert_eq!(plan::validate_file(&file).unwrap()["executed"], false);
    assert!(!f.store().exists());
    let dir = f.create_plan(&p);
    let result = runner::run(&dir, None, None).unwrap();
    assert_eq!(result["process_status"], "command_succeeded");
    assert_eq!(result["evidence_status"], "unreviewed");
    assert_eq!(result["promotion"], "none");
    let cases: Value = inv::read(&dir.join("output/cases.json")).unwrap();
    assert_eq!(cases["cases"][0]["context"]["literal"], literal);
    assert!(!dir.join("work/NEVER").exists());
    assert!(result["steps"][0]["pid"].as_u64().unwrap() > 0);
    assert_eq!(result["steps"][0]["exit_code"], 0);
    assert!(
        result["steps"][0]["reported_peak_rss_bytes"]
            .as_u64()
            .unwrap()
            > 0
    );
    let command: Value = inv::read(&dir.join("evidence/step-000/command.json")).unwrap();
    assert_eq!(command["environment"]["PYTHONDONTWRITEBYTECODE"], "1");
    let note = f.root().join("note");
    fs::write(&note,"/Users/private/session SECRETKEY icq-message raw transcript; revisit after independent data").unwrap();
    assert!(comparison::export(&dir, &f.root().join("early-export")).is_err());
    comparison::review(&dir, "retire", &note).unwrap();
    assert_eq!(runner::inspect(&dir).unwrap()["state"], "retired");
    let export = f.root().join("public");
    comparison::export(&dir, &export).unwrap();
    for name in ["summary.json", "summary.md"] {
        let text = fs::read_to_string(export.join(name)).unwrap();
        for secret in [
            "/Users/",
            "SECRETKEY",
            "session",
            "icq-message",
            "transcript",
            "honest",
        ] {
            assert!(!text.contains(secret), "{text}");
        }
    }
    assert!(comparison::review(&dir, "continue", &note).is_err());
    assert!(runner::run(&dir, None, None).is_err());
}

#[test]
fn inventory_preserves_invalid_bytes_refuses_mutation_links_and_member_changes() {
    let f = Fixture::new();
    let source = f.root().join("retained");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("failed-response.bin"), [0, 255, 13, 0]).unwrap();
    fs::write(
        source.join("report.json"),
        b"invalid JSON retained, not repaired",
    )
    .unwrap();
    fs::create_dir(source.join("empty")).unwrap();
    let inventory = inv::scan(&source, "capture", Some("report.json".into())).unwrap();
    let relocated = f.root().join("relocated");
    inv::copy(&source, &inventory, &relocated).unwrap();
    inv::verify(&relocated, &inventory).unwrap();
    assert_eq!(
        fs::read(relocated.join("failed-response.bin")).unwrap(),
        [0, 255, 13, 0]
    );
    fs::write(relocated.join("extra"), b"new").unwrap();
    assert!(inv::verify(&relocated, &inventory).is_err());
    fs::write(source.join("failed-response.bin"), b"changed").unwrap();
    assert!(inv::verify(&source, &inventory).is_err());
    let linked = f.root().join("linked");
    fs::create_dir(&linked).unwrap();
    std::os::unix::fs::symlink("/etc/hosts", linked.join("link")).unwrap();
    assert!(inv::scan(&linked, "data", None).is_err());
    for bad in ["../escape", "/absolute", "a/../../x", "a\\b", "./x"] {
        assert!(inv::relative(bad).is_err(), "{bad}");
    }
}

#[test]
fn tampered_plan_inventory_source_and_unsafe_paths_are_refused() {
    let f = Fixture::new();
    let dir = f.create("tamper-plan", "ok", 0.0);
    fs::write(dir.join("plan.json"), b"{}").unwrap();
    assert!(runner::run(&dir, None, None).is_err());
    assert!(!dir.join("start.json").exists());
    let mut p = f.plan("escape", "ok", 0.0);
    p.required_artifacts = vec!["../escape.json".into()];
    assert!(plan::validate(&p).is_err());
    p = f.plan("bad-cwd", "ok", 0.0);
    p.commands[0].cwd = "../outside".into();
    assert!(plan::validate(&p).is_err());
    let dir = f.create("tamper-source", "ok", 0.0);
    let sha = &f.base.bundles[1].sha256;
    fs::write(
        f.store()
            .join("snapshots")
            .join(sha)
            .join("data/evaluate.py"),
        "tampered",
    )
    .unwrap();
    assert!(runner::run(&dir, None, None).is_err());
    let mut inventory: inv::Inventory = inv::read(&f.base.bundles[0].inventory).unwrap();
    inventory.files[0].path = "../escape".into();
    assert!(inv::verify(&f.base.bundles[0].source, &inventory).is_err());
    let f = Fixture::new();
    let mut p = f.plan("bad-pin", "ok", 0.0);
    p.external_files[0].sha256 = "0".repeat(64);
    assert!(plan::validate(&p).is_err());
    let store = f.store();
    fs::create_dir(&store).unwrap();
    std::os::unix::fs::symlink(f.root().join("input"), store.join("experiments")).unwrap();
    let file = f.root().join("escape-store.json");
    inv::json(&file, &f.plan("escape-store", "ok", 0.0)).unwrap();
    assert!(plan::create(&file, &store).is_err());
    assert!(!f.root().join("input/escape-store").exists());
}

#[test]
fn crashes_timeouts_partial_output_and_output_limits_are_durable() {
    let f = Fixture::new();
    let mut unrelated = Command::new("/bin/sleep").arg("10").spawn().unwrap();
    for (mode, state) in [
        ("crash", "failed"),
        ("timeout", "timed_out"),
        ("descendant", "timed_out"),
        ("overflow", "resource_limited"),
        ("pressure", "resource_limited"),
        ("symlink", "resource_limited"),
    ] {
        let mut p = f.plan(mode, mode, 0.0);
        p.limits.wall_time_ms = if ["timeout", "descendant"].contains(&mode) {
            500
        } else {
            1500
        };
        let dir = f.create_plan(&p);
        let began = Instant::now();
        let r = runner::run(&dir, None, None).unwrap();
        assert_eq!(r["process_status"], state, "{r}");
        assert!(began.elapsed() < Duration::from_secs(4));
        assert_eq!(r["exit_code"], 3);
        assert_eq!(
            fs::read_to_string(dir.join("output/partial.txt")).unwrap(),
            "retained before completion\n"
        );
        assert!(
            fs::metadata(dir.join("evidence/step-000/stdout"))
                .unwrap()
                .len()
                > 0
        );
        if mode == "symlink" {
            assert_eq!(r["evidence_status"], "invalid");
            assert!(runner::verified_result(&dir).is_err());
        } else {
            runner::verified_result(&dir).unwrap();
        }
        if mode == "descendant" {
            let record: Value = inv::read(&dir.join("output/child.json")).unwrap();
            let pid = record["pid"].to_string();
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                let output = Command::new("ps")
                    .args(["-p", &pid, "-o", "stat="])
                    .output()
                    .unwrap();
                let status = String::from_utf8_lossy(&output.stdout);
                if status.trim().is_empty() || status.trim_start().starts_with('Z') {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "owned descendant survived timeout: {pid} {status}"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
    assert!(unrelated.try_wait().unwrap().is_none());
    unrelated.kill().unwrap();
    unrelated.wait().unwrap();
}

#[test]
fn concurrent_reused_and_interrupted_identities_never_resume() {
    let f = Fixture::new();
    let mut p = f.plan("concurrent", "timeout", 0.0);
    p.limits.wall_time_ms = 500;
    let dir = f.create_plan(&p);
    let mut child = Command::new(env!("CARGO_BIN_EXE_paperhands"))
        .args(["experiment", "run", "--dir"])
        .arg(&dir)
        .env("PAPERHANDS_SKILLS_DIR", f.root().join("managed"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !dir.join("evidence/step-000/start.json").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(dir.join("start.json").exists());
    assert!(runner::run(&dir, None, None).is_err());
    assert_eq!(child.wait().unwrap().code(), Some(3));
    assert!(runner::run(&dir, None, None).is_err());
    let file = f.root().join("reused-plan.json");
    inv::json(&file, &p).unwrap();
    assert!(plan::create(&file, &f.store()).is_err());
    let abandoned = f.create("abandoned", "ok", 0.0);
    inv::json(
        &abandoned.join("start.json"),
        &json!({"runner_pid":0,"started_at_ms":0}),
    )
    .unwrap();
    assert_eq!(
        runner::inspect(&abandoned).unwrap()["process_status"],
        "interrupted"
    );
    assert!(runner::run(&abandoned, None, None).is_err());
}

#[test]
fn comparisons_retain_unknowns_unmatched_and_refuse_invalid_or_changed_outputs() {
    let f = Fixture::new();
    let b = f.create("baseline", "ok", 0.0);
    let c = f.create("candidate", "ok", 1.0);
    runner::run(&b, None, None).unwrap();
    runner::run(&c, None, None).unwrap();
    let out = f.root().join("comparison.json");
    let r = comparison::compare(&b, &c, &out).unwrap();
    assert_eq!(r["common_cases"], 2);
    assert_eq!(r["candidate_only"], 1);
    let data: Value = inv::read_sealed(&out).unwrap();
    assert!(
        data["common"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == "unknown" && c["delta"]["net"].is_null())
    );
    assert_eq!(data["promotion"], "none");
    for mode in ["nonfinite", "missing_metric", "units", "duplicate"] {
        let dir = f.create(mode, mode, 0.0);
        runner::run(&dir, None, None).unwrap();
        let out = f.root().join(format!("bad-{mode}.json"));
        assert!(comparison::compare(&b, &dir, &out).is_err());
        assert_eq!(
            inv::read_sealed::<Value>(&out).unwrap()["evidence_status"],
            "unsupported_or_invalid"
        );
    }
    let missing = f.create("missing", "missing", 0.0);
    let r = runner::run(&missing, None, None).unwrap();
    assert_eq!(r["process_status"], "command_succeeded");
    assert_eq!(r["evidence_status"], "invalid");
    assert_eq!(r["exit_code"], 3);
    fs::write(c.join("output/partial.txt"), "mutated").unwrap();
    assert!(runner::verified_result(&c).is_err());
    assert!(comparison::compare(&b, &c, &f.root().join("mutated.json")).is_err());
}

#[test]
fn two_hundred_members_share_snapshots_and_account_for_every_failure() {
    let f = Fixture::new();
    let mut ids = Vec::new();
    for i in 0..200 {
        let id = format!("policy-{i:03}");
        let mode = match i {
            31 => "crash",
            47 => "timeout",
            193 => "missing",
            _ => "ok",
        };
        f.create(&id, mode, 0.0);
        ids.push(id);
    }
    assert_eq!(
        fs::read_dir(f.store().join("snapshots")).unwrap().count(),
        3
    );
    let plan = f.root().join("batch.json");
    inv::json(&plan,&json!({"schema_version":1,"id":"two-hundred","attempts":ids,"max_parallel":4,"wall_time_ms":120000,"total_output_bytes":10485760,"total_artifact_bytes":10485760})).unwrap();
    let r = batch::run(&f.store(), &plan).unwrap();
    assert_eq!(r["planned_members"], 200);
    assert_eq!(r["members"].as_array().unwrap().len(), 200);
    assert_eq!(r["states"]["failed"], 1);
    assert_eq!(r["states"]["timed_out"], 1);
    assert_eq!(r["peak_workers"], 4);
    assert_eq!(
        r["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["exit_code"] == 0)
            .count(),
        197
    );
    assert!(r["cpu_time_ms"].as_u64().unwrap() > 0);
    assert_eq!(r["exit_code"], 3);
    assert!(batch::run(&f.store(), &plan).is_err());
    assert_eq!(
        batch::inspect(&f.store().join("batches/two-hundred")).unwrap()["planned_members"],
        200
    );
}

#[test]
fn batch_budget_exhaustion_keeps_all_queued_members() {
    let f = Fixture::new();
    let mut ids = Vec::new();
    for i in 0..20 {
        let id = format!("budget-{i}");
        f.create(&id, "timeout", 0.0);
        ids.push(id);
    }
    let file = f.root().join("budget.json");
    inv::json(&file,&json!({"schema_version":1,"id":"budget","attempts":ids,"max_parallel":2,"wall_time_ms":100,"total_output_bytes":1024,"total_artifact_bytes":1024})).unwrap();
    let r = batch::run(&f.store(), &file).unwrap();
    assert_eq!(r["members"].as_array().unwrap().len(), 20);
    assert!(r["states"]["not_started_budget"].as_u64().unwrap() > 0);
    assert!(r["peak_workers"].as_u64().unwrap() <= 2);
    let queued = r["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["process_status"] == "not_started_budget")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let dir = f.store().join("experiments").join(queued);
    assert_eq!(
        runner::inspect(&dir).unwrap()["process_status"],
        "not_started_budget"
    );
    assert!(runner::run(&dir, None, None).is_err());
}
