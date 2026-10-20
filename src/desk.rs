//! The research frontier comes from retained tasks and evidence, never scalar promotion.
use crate::{
    Result, memory,
    research::{batch, inventory as inv, runner},
    tasks,
};
use serde_json::{Value, json};
use std::{fs, path::Path};

pub fn initialize(root: &Path) -> Result<Value> {
    memory::initialize(root)?;
    tasks::initialize(root)?;
    Ok(
        json!({"root":inv::root(root)?,"initialized":true,"execution_started":false,"trading_enabled":false}),
    )
}
fn names(root: &Path, collection: &str) -> Result<Vec<String>> {
    let path = root.join(collection);
    match fs::symlink_metadata(&path) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.into()),
    }
    inv::root(&path)?;
    let mut ids = Vec::new();
    for entry in fs::read_dir(path)? {
        let name = entry?
            .file_name()
            .into_string()
            .map_err(|_| "unsafe research-store identity")?;
        inv::name(&name)?;
        ids.push(name);
        if ids.len() > 4096 {
            return Err("desk collection exceeds 4096 members".into());
        }
    }
    ids.sort();
    Ok(ids)
}
fn exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}
pub fn inspect(root: &Path, offset: usize, limit: usize) -> Result<Value> {
    let root = inv::root(root)?;
    let mut gaps = Vec::new();
    let notes = if exists(&root.join("memory"))? {
        match memory::catalog(&root) {
            Ok(v) => v,
            Err(e) => {
                gaps.push(json!({"collection":"memory","error":e.to_string()}));
                vec![]
            }
        }
    } else {
        vec![]
    };
    let tasks = if exists(&root.join("tasks"))? {
        match tasks::catalog(&root) {
            Ok(v) => v,
            Err(e) => {
                gaps.push(json!({"collection":"tasks","error":e.to_string()}));
                vec![]
            }
        }
    } else {
        vec![]
    };
    let mut rows = Vec::new();
    let attention = match crate::graph::attention(&root) {
        Ok(v) => v,
        Err(e) => {
            gaps.push(json!({"collection":"graph","error":e.to_string()}));
            Default::default()
        }
    };
    let mut frontier=tasks.iter().filter(|t|t["ready"]==true).map(|t|json!({"kind":"task","id":t["id"],"title":t["title"],"reason":"open task; fixed prerequisites are done","attention":attention.get(t["id"].as_str().unwrap_or("")),"argv":["paperhands","tasks","read","--root",root,"--id",t["id"]]})).collect::<Vec<_>>();
    frontier.sort_by(|a, b| {
        b["attention"]["priority"]
            .as_f64()
            .partial_cmp(&a["attention"]["priority"].as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a["id"].as_str().cmp(&b["id"].as_str()))
    });
    let experiments = names(&root, "experiments")?;
    let batches = names(&root, "batches")?;
    let judges = names(&root, "judges")?;
    let total = experiments.len() + batches.len() + judges.len();
    for (collection, ids, count_before) in [
        ("experiments", &experiments, 0),
        ("batches", &batches, experiments.len()),
        ("judges", &judges, experiments.len() + batches.len()),
    ] {
        for (index, id) in ids.iter().enumerate() {
            let position = count_before + index;
            if position < offset || position >= offset.saturating_add(limit) {
                continue;
            }
            let dir = inv::checked(&root, &format!("{collection}/{id}"))?;
            let result = if collection == "experiments" {
                runner::inspect(&dir)
            } else if collection == "batches" {
                batch::inspect(&dir)
            } else {
                crate::judge::inspect(&dir)
            };
            match result {
                Ok(record) => {
                    let run = if collection != "batches" {
                        &record["result"]
                    } else {
                        &record
                    };
                    rows.push(json!({"id":id,"kind":collection,"process_status":record["process_status"],"evidence_status":record["evidence_status"],"state":record["state"],"wall_time_ms":run["wall_time_ms"],"exit_code":run["exit_code"],"integrity_status":if run.is_null()||record["integrity_status"]=="not_checked"{"not_sealed"}else{"verified"},"plan_sha256":record["plan_sha256"]}));
                    if collection == "experiments"
                        && !record["result"].is_null()
                        && record["review"].is_null()
                    {
                        frontier.push(json!({"kind":"review","id":id,"reason":"retained completion has no researcher review; process success is separate","argv":["paperhands","experiment","inspect","--dir",dir]}));
                    }
                }
                Err(error) => {
                    let gap = json!({"collection":collection,"id":id,"error":error.to_string()});
                    gaps.push(gap);
                    rows.push(json!({"id":id,"kind":collection,"process_status":"unknown","evidence_status":"integrity_gap","integrity_status":"failed","wall_time_ms":null,"exit_code":null}));
                }
            }
        }
    }
    let mut note_kinds = std::collections::BTreeMap::<String, usize>::new();
    for note in &notes {
        *note_kinds
            .entry(note["kind"].as_str().unwrap_or("unknown").into())
            .or_default() += 1;
    }
    let mut task_states = std::collections::BTreeMap::<String, usize>::new();
    for task in &tasks {
        *task_states
            .entry(task["state"].as_str().unwrap_or("unknown").into())
            .or_default() += 1;
    }
    let notes_verified = !gaps.iter().any(|g| g["collection"] == "memory");
    let tasks_verified = !gaps.iter().any(|g| g["collection"] == "tasks");
    let notes_total = notes_verified.then_some(notes.len());
    let tasks_total = tasks_verified.then_some(tasks.len());
    Ok(
        json!({"schema":"paperhands.desk/v1","generated_at_ms":inv::now(),"root":root,"notes_total":notes_total,"note_kinds":notes_verified.then_some(note_kinds),"tasks_total":tasks_total,"task_states":tasks_verified.then_some(task_states),
        "tasks":tasks.into_iter().take(100).collect::<Vec<_>>(),"questions":notes.iter().filter(|n|n["kind"]=="question").take(20).collect::<Vec<_>>(),
        "rows":rows,"frontier":frontier.into_iter().take(100).collect::<Vec<_>>(),"gaps":gaps,
        "coverage":{"offset":offset,"limit":limit,"total_attempts_and_batches":total,"all_evidence_inspected":offset==0&&total<=limit,"tasks_return_limit":100,"questions_return_limit":20,"frontier_return_limit":100},
        "researcher":"external agent through MCP; the binary provides grounded companion navigation, memory and bounded instruments; no strategy-generating model or optimizer",
        "selection_policy":"ready tasks use declared graph attention weights, then IDs; unreviewed selected attempts follow. No financial/semantic score ranking, automatic retirement or promotion","trading_enabled":false}),
    )
}
