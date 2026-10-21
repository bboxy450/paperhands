//! A public projection, deliberately separate from the private research desk.
use crate::{
    Result,
    research::{comparison, inventory as inv},
};
use clap::{Arg, ArgMatches, Command};
use regex::Regex;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

fn exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn output_path(path: &Path) -> Result<PathBuf> {
    let mut absolute = PathBuf::new();
    for component in std::path::absolute(path)?.components() {
        match component {
            Component::ParentDir => {
                absolute.pop();
            }
            Component::CurDir => {}
            other => absolute.push(other.as_os_str()),
        }
    }
    let mut parent = absolute.clone();
    let mut missing = Vec::new();
    while !exists(&parent)? {
        missing.push(
            parent
                .file_name()
                .ok_or("invalid report output")?
                .to_owned(),
        );
        parent.pop();
    }
    let mut resolved = fs::canonicalize(parent)?;
    for name in missing.into_iter().rev() {
        resolved.push(name);
    }
    if exists(&absolute)? {
        inv::root(&absolute)?;
    }
    Ok(resolved)
}

pub fn command() -> Command {
    Command::new("report")
        .about("Atomically refresh public-allowlist report.json and report.md; never upload or start research")
        .arg(Arg::new("root").long("root"))
        .arg(Arg::new("out").long("out").required(true).help("Public output directory; replaces report.json and report.md, never source records"))
}

pub fn execute(args: &ArgMatches) -> Result<Value> {
    write(
        &crate::project::resolve(args.get_one::<String>("root").map(String::as_str))?,
        Path::new(args.get_one::<String>("out").unwrap()),
    )
}

/// Titles are the only caller-supplied prose admitted to the public projection.
pub fn public_text(text: &str) -> String {
    let patterns = [
        r"(?i)(?:https?://|www\.)\S+",
        r"(?i)\bbearer(?:\s+|[=:])\S+|\bsk-[a-z0-9_-]+",
        r"(?i)[a-f0-9]{20,}",
        r"(?i)(?:[a-z]:[\\/]|(?:~|\.)?/|\.kb[\\/])\S*",
    ];
    let mut clean = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>();
    for pattern in patterns {
        clean = Regex::new(pattern)
            .expect("fixed public sanitizer regex")
            .replace_all(&clean, " ")
            .into_owned();
    }
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    let clean = clean.chars().take(120).collect::<String>();
    if clean.is_empty() {
        "Redacted task title".into()
    } else {
        clean
    }
}

fn enum_counts(value: &Value, allowed: &[&str]) -> Result<Value> {
    let object = value
        .as_object()
        .ok_or("public report requires verified collection counts")?;
    let mut counts = BTreeMap::new();
    for (key, number) in object {
        if !allowed.contains(&key.as_str()) || number.as_u64().is_none() {
            return Err("public report refuses unknown count category".into());
        }
        counts.insert(key, number.clone());
    }
    Ok(json!(counts))
}

fn memory_coverage(value: &Value) -> Result<Value> {
    // The desk's private coverage contains volume identities, directories and argv.
    let mut public = json!({"complete":value["complete"]});
    if value["complete"] != true {
        return Err("public report requires complete retained memory coverage".into());
    }
    for key in [
        "notes_total",
        "active_notes",
        "planned_volumes",
        "max_volumes",
        "max_notes_per_volume",
        "indexed_passages",
        "indexed_text_bytes",
        "max_total_passages",
        "max_total_text_bytes",
    ] {
        if let Some(number) = value.get(key) {
            if number.as_u64().is_none() {
                return Err("invalid memory coverage count".into());
            }
            public[key] = number.clone();
        }
    }
    for (source, target) in [
        ("note_set_sha256", "note_set_sha256"),
        ("catalog_head", "catalog_head_sha256"),
    ] {
        if let Some(digest) = value.get(source) {
            if !digest.is_null() && !digest.as_str().is_some_and(inv::valid_hash) {
                return Err("invalid memory coverage digest".into());
            }
            public[target] = digest.clone();
        }
    }
    if let Some(volumes) = value["verified_volumes"].as_array() {
        public["verified_volumes"] = Value::Array(
            volumes
                .iter()
                .map(|volume| {
                    let mut row = json!({"closed":volume["closed"]});
                    if !row["closed"].is_boolean() {
                        return Err("invalid volume closure flag".into());
                    }
                    for key in ["notes", "indexed_passages", "indexed_text_bytes"] {
                        if volume[key].as_u64().is_none() {
                            return Err("invalid volume count".into());
                        }
                        row[key] = volume[key].clone();
                    }
                    Ok(row)
                })
                .collect::<Result<Vec<_>>>()?,
        );
    }
    Ok(public)
}

fn automation(value: &Value) -> Result<Value> {
    let mut schedules = Vec::new();
    let mut missed_slots = 0u64;
    for row in value["items"]
        .as_array()
        .ok_or("invalid automation status")?
    {
        let state = row["status"].as_str().ok_or("missing schedule state")?;
        if !["waiting", "due", "cancelled", "expired", "budget_exhausted"].contains(&state) {
            return Err("unknown public schedule state".into());
        }
        let mut public = json!({"status":state});
        for key in [
            "interval_ms",
            "attempted_wakes",
            "max_wakes",
            "remaining_attempts",
        ] {
            if row[key].as_u64().is_none() {
                return Err("invalid schedule count".into());
            }
            public[key] = row[key].clone();
        }
        for key in ["next_due_at_ms", "expires_at_ms"] {
            if !row[key].is_null() && row[key].as_u64().is_none() {
                return Err("invalid schedule time".into());
            }
            public[key] = row[key].clone();
        }
        missed_slots = missed_slots
            .checked_add(
                row["missed_slots"]
                    .as_u64()
                    .ok_or("missing measured missed-slot count")?,
            )
            .ok_or("missed-slot count overflow")?;
        schedules.push(public);
    }
    Ok(
        json!({"schedules":schedules,"missed_slots":missed_slots,"model_or_evaluator_started":false}),
    )
}

/// Declared hash fields and fixed schema names are the only regex exceptions.
fn public_gate(report: &Value) -> Result<()> {
    fn mask(value: &Value, key: &str) -> Result<Value> {
        if key.ends_with("_sha256") {
            return if value.is_null() {
                Ok(Value::Null)
            } else if value.as_str().is_some_and(inv::valid_hash) {
                Ok(json!("verified digest"))
            } else {
                Err("invalid public digest".into())
            };
        }
        match value {
            Value::Object(object) => object
                .iter()
                .map(|(key, value)| Ok((key.clone(), mask(value, key)?)))
                .collect::<Result<serde_json::Map<_, _>>>()
                .map(Value::Object),
            Value::Array(items) => items
                .iter()
                .map(|item| mask(item, ""))
                .collect::<Result<Vec<_>>>()
                .map(Value::Array),
            Value::String(text)
                if key == "schema"
                    && ["paperhands.report/v1", "paperhands.public/v1"]
                        .contains(&text.as_str()) =>
            {
                Ok(json!("verified schema"))
            }
            _ => Ok(value.clone()),
        }
    }
    let serialized = serde_json::to_string(&mask(report, "")?)?;
    let forbidden =
        Regex::new(r"(?i)(?:https?://|www\.|/|\\\\|\.kb\b|[a-f0-9]{20,}|\bbearer\b|\bsk-)")?;
    if forbidden.is_match(&serialized) {
        return Err("public report refused private-looking data; previous output retained".into());
    }
    Ok(())
}

fn experiments(root: &Path) -> Result<Vec<Value>> {
    let directory = root.join("experiments");
    if !exists(&directory)? {
        return Ok(vec![]);
    }
    inv::root(&directory)?;
    let mut entries = fs::read_dir(&directory)?.collect::<std::io::Result<Vec<_>>>()?;
    if entries.len() > 4096 {
        return Err("public report experiment scan exceeds 4096 members".into());
    }
    entries.sort_by_key(|entry| entry.file_name());
    let mut rows = Vec::new();
    for entry in entries {
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "invalid experiment name")?;
        inv::name(&name)?;
        let dir = inv::checked(root, &format!("experiments/{name}"))?;
        let review = dir.join("review.json");
        if !exists(&review)? {
            continue;
        }
        let digest = inv::digest(&review)?;
        let reviewed: Value = inv::read_sealed(&review)?;
        let mut public = comparison::public_summary(&dir)?;
        for (key, allowed) in [
            ("state", &["reviewed", "retired"][..]),
            (
                "process_status",
                &[
                    "command_succeeded",
                    "failed",
                    "runner_failed",
                    "timed_out",
                    "resource_limited",
                    "interrupted",
                    "spawn_failed",
                    "not_started_budget",
                ][..],
            ),
            ("evidence_status", &["unreviewed", "invalid"][..]),
            (
                "disposition",
                &["continue", "revise", "nominate", "inconclusive", "retire"][..],
            ),
        ] {
            if !public[key]
                .as_str()
                .is_some_and(|value| allowed.contains(&value))
            {
                return Err("unknown public lifecycle category".into());
            }
        }
        for key in ["wall_time_ms", "commands_recorded"] {
            if !public[key].is_null() && public[key].as_u64().is_none() {
                return Err("invalid public lifecycle count".into());
            }
        }
        if inv::digest(&review)? != digest {
            return Err("review changed during public inspection".into());
        }
        public["reviewed_at_ms"] = reviewed["reviewed_at_ms"].clone();
        if !public["reviewed_at_ms"].is_null() && public["reviewed_at_ms"].as_u64().is_none() {
            return Err("invalid review timestamp".into());
        }
        rows.push(public);
    }
    rows.sort_by(|a, b| {
        b["reviewed_at_ms"]
            .as_u64()
            .cmp(&a["reviewed_at_ms"].as_u64())
    });
    rows.truncate(200);
    Ok(rows)
}

fn zoom_summary(value: &Value) -> Result<Value> {
    if !value["verdict"].is_null()
        && !matches!(
            value["verdict"].as_str(),
            Some("stalled" | "drifting" | "moving")
        )
    {
        return Err("unsupported public zoom verdict".into());
    }
    let generated = value["generated_at_ms"]
        .as_u64()
        .ok_or("missing zoom generation clock")?;
    let kinds = [
        "repeated_refusal",
        "unreviewed_output",
        "monoculture",
        "no_new_evidence",
        "spent_without_change",
    ];
    let fired = value["signals_fired"]
        .as_array()
        .ok_or("missing zoom signal summary")?;
    let mut seen = std::collections::BTreeSet::new();
    for kind in fired {
        let kind = kind.as_str().ok_or("invalid public zoom signal")?;
        if !kinds.contains(&kind) || !seen.insert(kind) {
            return Err("unsupported or duplicate public zoom signal".into());
        }
    }
    Ok(json!({"verdict":value["verdict"],"signals_fired":fired,"generated_at_ms":generated}))
}

fn build(root: &Path, schedules: &Value, zoom: &Value) -> Result<Value> {
    let desk = crate::desk::inspect(root, 0, usize::MAX)?;
    if desk["notes_total"].as_u64().is_none() || desk["tasks_total"].as_u64().is_none() {
        return Err("public report cannot publish unknown memory or task totals".into());
    }
    let kinds = enum_counts(
        &desk["note_kinds"],
        &[
            "observation",
            "source",
            "hypothesis",
            "lesson",
            "question",
            "direction",
        ],
    )?;
    let states = enum_counts(
        &desk["task_states"],
        &["open", "active", "blocked", "done", "retired"],
    )?;
    let mut tasks = if exists(&root.join("tasks"))? {
        crate::tasks::catalog(root)?
    } else {
        vec![]
    };
    if tasks.len() as u64 != desk["tasks_total"].as_u64().unwrap() {
        return Err("tasks changed during report inspection; retry".into());
    }
    let ready = tasks.iter().filter(|task| task["ready"] == true).count();
    tasks.sort_by(|a, b| {
        b["recorded_at_ms"]
            .as_u64()
            .cmp(&a["recorded_at_ms"].as_u64())
            .then(a["id"].as_str().cmp(&b["id"].as_str()))
    });
    let tasks = tasks.into_iter().take(100).map(|task| {
        if !task["ready"].is_boolean() || task["recorded_at_ms"].as_u64().is_none() { return Err("invalid task metadata".into()); }
        Ok(json!({"title":public_text(task["title"].as_str().ok_or("missing task title")?),"state":task["state"],"ready":task["ready"],"depends_count":task["depends_on"].as_array().ok_or("missing prerequisites")?.len(),"updated_at_ms":task["recorded_at_ms"]}))
    }).collect::<Result<Vec<_>>>()?;
    let report = json!({
        "schema":"paperhands.report/v1","generated_at_ms":inv::now(),"binary_version":crate::VERSION,"visibility":"public-allowlist",
        "frontier":{"notes_total":desk["notes_total"],"note_kinds":kinds,"tasks_total":desk["tasks_total"],"task_states":states,"tasks_ready":ready,"questions_open":kinds["question"].as_u64().unwrap_or(0),"gaps_count":desk["gaps"].as_array().ok_or("missing gap inventory")?.len(),"catalog_fully_paginated":desk["coverage"]["all_evidence_inspected"],"memory_coverage":memory_coverage(&desk["memory_coverage"])?},
        "tasks":tasks,"experiments":experiments(root)?,"automation":automation(schedules)?,"zoom":zoom_summary(zoom)?,
        "limitations":["Public aggregate projection; private text, identities and navigation are excluded.","Task titles are redacted and limited to 120 characters. At most 100 newest tasks and 200 newest reviewed experiments are shown; frontier counts cover the verified catalogs.","catalog_fully_paginated means native execution-catalog pagination coverage, not substantive source review or successful execution.","Zoom is a descriptive retained-state signal summary; null is unknown. It creates no review, task, retirement or research decision, and excludes private evidence IDs and premise text.","Question count means retained question notes; no resolution is inferred.","Memory coverage exposes verified counts, bounds and named digests, with private volume identities and paths omitted.","Collections are inspected under their own locks; this is not an atomic cross-collection snapshot. Integrity gaps are counted without publishing private errors.","Reviewed lifecycle evidence is not financial validation, independent market evidence or automatic promotion.","Wake scheduling and missed nominal slots do not prove model delivery or completed work.","Files are atomically replaced individually; readers must check generation time. JSON and Markdown are not a transactional pair."]
    });
    public_gate(&report)?;
    Ok(report)
}

pub fn write(root: &Path, out: &Path) -> Result<Value> {
    let root = inv::root(root)?;
    let schedules = if exists(&root.join("automation"))? {
        let args = crate::automation::command().try_get_matches_from([
            "automation",
            "status",
            "--root",
            root.to_str().ok_or("non-UTF-8 root")?,
        ])?;
        crate::automation::execute(&args)?
    } else {
        json!({"items":[]})
    };
    let zoom = crate::zoom::inspect(&root, None, &crate::zoom::Thresholds::default())?;
    write_with_automation(&root, out, &schedules, &zoom)
}

/// Tick supplies its already-locked status, avoiding a recursive automation lock.
pub(crate) fn write_with_automation(
    root: &Path,
    out: &Path,
    schedules: &Value,
    zoom: &Value,
) -> Result<Value> {
    let root = inv::root(root)?;
    let out = output_path(out)?;
    for collection in [
        "memory",
        "memory-volumes",
        "tasks",
        "graph",
        "experiments",
        "batches",
        "judges",
        "automation",
    ] {
        if out.starts_with(root.join(collection)) {
            return Err("public output must be outside retained collections".into());
        }
    }
    let report = build(&root, schedules, zoom)?;
    let bytes = serde_json::to_vec_pretty(&report)?;
    let markdown = format!(
        "# Paperhands research report\n\nGenerated at UTC epoch millisecond {} by Paperhands {}.\n\n{} notes · {} tasks · {} ready · {} open questions · {} integrity gaps\n\nPublic lifecycle evidence only; no financial validation or automatic promotion.\n\nThe complete public snapshot below contains the task, reviewed-experiment and automation tables.\n\n```json\n{}\n```\n",
        report["generated_at_ms"],
        crate::VERSION,
        report["frontier"]["notes_total"],
        report["frontier"]["tasks_total"],
        report["frontier"]["tasks_ready"],
        report["frontier"]["questions_open"],
        report["frontier"]["gaps_count"],
        std::str::from_utf8(&bytes)?
    );
    // Do not publish over retained evidence or through output symlinks.
    fs::create_dir_all(&out)?;
    let out = inv::root(&out)?;
    let json_path = out.join("report.json");
    let md_path = out.join("report.md");
    let old_json = crate::setup::optional(&json_path)?;
    let old_md = crate::setup::optional(&md_path)?;
    crate::setup::replace(&md_path, old_md.as_deref(), markdown.as_bytes(), None)?;
    crate::setup::replace(&json_path, old_json.as_deref(), &bytes, None)?;
    Ok(
        json!({"report":"written","generated_at_ms":report["generated_at_ms"],"report_sha256":inv::hash(&bytes),"visibility":"public-allowlist","directory":out,"model_or_evaluator_started":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_projection_excludes_private_ids_questions_and_errors() {
        let value = json!({"verdict":"stalled","signals_fired":["repeated_refusal"],"generated_at_ms":17,"evidence":["private-id"],"questions":["private premise"],"error":"/private/source"});
        assert_eq!(
            zoom_summary(&value).unwrap(),
            json!({"verdict":"stalled","signals_fired":["repeated_refusal"],"generated_at_ms":17})
        );
        assert!(
            zoom_summary(&json!({"verdict":"private","signals_fired":[],"generated_at_ms":17}))
                .is_err()
        );
        assert!(
            zoom_summary(
                &json!({"verdict":null,"signals_fired":["private-id"],"generated_at_ms":17})
            )
            .is_err()
        );
        assert!(zoom_summary(&json!({"verdict":null,"signals_fired":[]})).is_err());
    }

    #[test]
    fn coverage_projects_counts_without_private_navigation() {
        let public = memory_coverage(&json!({"complete":true,"notes_total":2,"planned_volumes":2,"catalog_head":"a".repeat(64),"note_set_sha256":"b".repeat(64),"active_volume":"private-volume","rollover_argv":["/Users/private/root"],"verified_volumes":[{"id":"private-volume","relative_directory":"memory-volumes/private","notes":2,"closed":true,"indexed_passages":4,"indexed_text_bytes":20}]})).unwrap();
        assert!(public.get("active_volume").is_none());
        assert!(public.get("rollover_argv").is_none());
        assert_eq!(public["catalog_head_sha256"], "a".repeat(64));
        assert_eq!(public["verified_volumes"][0].as_object().unwrap().len(), 4);
        assert!(!public.to_string().contains("private"));
        public_gate(&public).unwrap();
    }

    #[test]
    fn final_gate_refuses_surviving_private_text_but_accepts_declared_hashes() {
        for poison in [
            "/Users/private",
            "https://secret.invalid",
            ".kb/private",
            "sk-secret",
            "Bearer",
            "abcdef0123456789abcdef",
        ] {
            assert!(public_gate(&json!({"title":poison})).is_err(), "{poison}");
        }
        public_gate(&json!({"schema":"paperhands.report/v1","result_sha256":"a".repeat(64)}))
            .unwrap();
        assert!(public_gate(&json!({"result_sha256":"private"})).is_err());
    }
}
