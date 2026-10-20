use super::{inventory as inv, plan, runner};
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub group: String,
    pub metrics: BTreeMap<String, Option<f64>>,
    pub context: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cases {
    pub schema_version: u32,
    pub accounting: String,
    pub units: BTreeMap<String, String>,
    pub cases: Vec<Case>,
    pub diagnostics: Value,
    pub limitations: Vec<String>,
}

fn cases(dir: &Path) -> Result<Cases> {
    let (_, p, _) = plan::load(dir)?;
    let path = p
        .cases
        .ok_or("plan has no declared normalized case artifact; comparison unsupported")?;
    let result: Cases = inv::read(&inv::checked(&dir.join("output"), &path)?)?;
    if result.schema_version != 1
        || result.accounting.is_empty()
        || result.units.is_empty()
        || result.cases.len() > 100_000
    {
        return Err("unsupported/empty case schema or accounting identity".into());
    }
    let mut ids = BTreeSet::new();
    for c in &result.cases {
        if c.id.is_empty()
            || c.id.len() > 1024
            || c.group.is_empty()
            || !ids.insert(&c.id)
            || c.metrics.keys().ne(result.units.keys())
            || c.metrics.values().flatten().any(|v| !v.is_finite())
        {
            return Err("duplicate/missing/nonfinite/mismatched case metrics".into());
        }
    }
    Ok(result)
}
fn fixed(p: &plan::Plan) -> BTreeMap<String, String> {
    let mut result = p
        .bundles
        .iter()
        .filter(|b| b.role != "candidate")
        .map(|b| (format!("{}:{}", b.role, b.name), b.sha256.clone()))
        .collect::<BTreeMap<_, _>>();
    for pin in &p.external_files {
        result.insert(
            format!("external:{}:{}", pin.role, pin.path.display()),
            pin.sha256.clone(),
        );
    }
    result
}
fn calculate(baseline: &Path, candidate: &Path) -> Result<Value> {
    let (_, bp, bf) = plan::load(baseline)?;
    let (_, cp, cf) = plan::load(candidate)?;
    let br = runner::verified_result(baseline)?;
    let cr = runner::verified_result(candidate)?;
    if br["process_status"] != "command_succeeded"
        || cr["process_status"] != "command_succeeded"
        || br["evidence_status"] == "invalid"
        || cr["evidence_status"] == "invalid"
    {
        return Err("both attempts need successful commands and intact required artifacts".into());
    }
    if fixed(&bp) != fixed(&cp) || bp.data_use != cp.data_use || bp.env != cp.env {
        return Err(
            "fixed inputs/evaluator/config/external dependencies, environment or data-use labels differ".into(),
        );
    }
    let b = cases(baseline)?;
    let c = cases(candidate)?;
    if b.accounting != c.accounting || b.units != c.units {
        return Err("accounting identity or metric units differ; comparison unsupported".into());
    }
    let bm = b
        .cases
        .iter()
        .map(|v| (&v.id, v))
        .collect::<BTreeMap<_, _>>();
    let cm = c
        .cases
        .iter()
        .map(|v| (&v.id, v))
        .collect::<BTreeMap<_, _>>();
    let mut common = Vec::new();
    let mut baseline_only = Vec::new();
    let mut candidate_only = Vec::new();
    for (id, left) in &bm {
        if let Some(right) = cm.get(id) {
            if left.group != right.group {
                return Err("common case partition/group differs".into());
            }
            let mut delta = BTreeMap::new();
            let mut unknown = Vec::new();
            for (metric, lv) in &left.metrics {
                match (lv, right.metrics[metric]) {
                    (Some(lv), Some(rv)) => {
                        let d = rv - lv;
                        if !d.is_finite() {
                            return Err("nonfinite metric delta".into());
                        }
                        delta.insert(metric, Some(d));
                    }
                    _ => {
                        delta.insert(metric, None);
                        unknown.push(metric.clone());
                    }
                }
            }
            common.push(json!({"id":id,"group":left.group,"baseline":left,"candidate":right,"delta":delta,"unknown_metrics":unknown,"context_equal":left.context==right.context}));
        } else {
            baseline_only.push(json!(left));
        }
    }
    for (id, right) in &cm {
        if !bm.contains_key(id) {
            candidate_only.push(json!(right));
        }
    }
    Ok(
        json!({"schema_version":1,"evidence_status":"diagnostic_comparison","baseline_plan_sha256":bf.plan_sha256,"candidate_plan_sha256":cf.plan_sha256,"baseline_result_sha256":inv::digest(&baseline.join("result.json"))?,"candidate_result_sha256":inv::digest(&candidate.join("result.json"))?,"fixed_snapshots":fixed(&bp),"data_use":bp.data_use,"accounting":b.accounting,"units":b.units,"common":common,"baseline_only":baseline_only,"candidate_only":candidate_only,"raw_diagnostics":{"baseline":b.diagnostics,"candidate":c.diagnostics},"limitations":{"baseline":b.limitations,"candidate":c.limitations,"runner":"Case IDs/accounting and financial diagnostics are supplied by trusted adapters. Anchor deltas are not whole-policy causal effects, independent samples, funded returns, fill proof or chain truth. Unknowns are retained, not replaced by zero. No promotion decision is made."},"promotion":"none"}),
    )
}
pub fn compare(baseline: &Path, candidate: &Path, out: &Path) -> Result<Value> {
    if out.exists() || out.with_extension("sha256").exists() {
        return Err("comparison identity/path already spent".into());
    }
    let result = match calculate(baseline, candidate) {
        Ok(v) => v,
        Err(e) => {
            json!({"schema_version":1,"evidence_status":"unsupported_or_invalid","error":e.to_string(),"promotion":"none"})
        }
    };
    let sha = inv::sealed_json(out, &result)?;
    if result["evidence_status"] == "unsupported_or_invalid" {
        return Err(format!(
            "comparison refused; retained record {} ({sha}): {}",
            out.display(),
            result["error"]
        )
        .into());
    }
    Ok(
        json!({"comparison":out,"sha256":sha,"common_cases":result["common"].as_array().map_or(0,Vec::len),"baseline_only":result["baseline_only"].as_array().map_or(0,Vec::len),"candidate_only":result["candidate_only"].as_array().map_or(0,Vec::len),"evidence_status":"diagnostic_comparison","promotion":"none"}),
    )
}
pub fn review(dir: &Path, disposition: &str, note: &Path) -> Result<Value> {
    let status = runner::inspect(dir)?;
    if ["planned", "queued", "running"]
        .contains(&status["process_status"].as_str().unwrap_or("invalid"))
    {
        return Err("cannot review an unstarted or running attempt".into());
    }
    if !["continue", "revise", "nominate", "inconclusive", "retire"].contains(&disposition) {
        return Err("unsupported disposition".into());
    }
    if fs::metadata(note)?.len() > 16384 {
        return Err("private review note exceeds 16 KiB".into());
    }
    let text = fs::read_to_string(note)?;
    if text.trim().is_empty() {
        return Err("review requires a private evidence/revisit note".into());
    }
    let record = json!({"schema_version":1,"disposition":disposition,"note":text,"reviewed_at_ms":inv::now(),"process_status":status["process_status"],"plan_sha256":status["plan_sha256"],"result_sha256":if dir.join("result.json").exists(){Some(inv::digest(&dir.join("result.json"))?)}else{None},"automatic_promotion":false});
    inv::sealed_json(&dir.join("review.json"), &record)?;
    Ok(
        json!({"state":if disposition=="retire"{"retired"}else{"reviewed"},"disposition":disposition,"promotion":"none"}),
    )
}
pub fn export(dir: &Path, out: &Path) -> Result<Value> {
    let status = runner::inspect(dir)?;
    let review = status["review"]
        .as_object()
        .ok_or("public export requires a durable review")?;
    let result = status.get("result").filter(|v| !v.is_null());
    if let Some(r) = result
        && (review.get("result_sha256").and_then(Value::as_str)
            != Some(inv::digest(&dir.join("result.json"))?.as_str())
            || r["plan_sha256"] != status["plan_sha256"])
    {
        return Err("review does not bind current result".into());
    }
    // Explicit allowlist: no plan/argv/environment/IDs/notes/diagnostics/path strings enter public output.
    let report = json!({"schema":"paperhands.public/v1","generated_at_ms":inv::now(),"state":status["state"],"process_status":status["process_status"],"evidence_status":status["evidence_status"],"disposition":review["disposition"],"plan_sha256":status["plan_sha256"],"result_sha256":review["result_sha256"],"wall_time_ms":result.map(|r|r["wall_time_ms"].clone()),"commands_recorded":result.and_then(|r|r["steps"].as_array()).map(Vec::len),"promotion":"none","scope":"Lifecycle summary only. Financial diagnostics and private evidence are not included."});
    fs::create_dir(out)?;
    inv::json(&out.join("summary.json"), &report)?;
    let md = format!(
        "# Reviewed experiment\n\nState: {}\n\nProcess: {}\n\nEvidence: {}\n\nDisposition: {}\n\nPlan SHA-256: {}\n\nLifecycle evidence only; no financial validation or automatic promotion.\n",
        report["state"].as_str().unwrap_or("unknown"),
        report["process_status"].as_str().unwrap_or("unknown"),
        report["evidence_status"].as_str().unwrap_or("unknown"),
        report["disposition"].as_str().unwrap_or("unknown"),
        report["plan_sha256"].as_str().unwrap_or("unknown")
    );
    inv::exclusive(&out.join("summary.md"), md.as_bytes())?;
    Ok(
        json!({"export":out,"sanitization":"fixed allowlist; free-form notes, labels, metrics, paths, source files and raw output excluded"}),
    )
}
