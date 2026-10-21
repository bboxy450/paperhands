//! Research work is an append-only sequence of explicit decisions, separate from run state.
use crate::{Result, research::inventory as inv};
use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    schema_version: u32,
    id: String,
    title: String,
    kind: String,
    depends_on: Vec<String>,
    sequence: usize,
    previous: Option<String>,
    state: String,
    note: String,
    recorded_at_ms: u64,
}
pub fn command() -> Command {
    let root = || Arg::new("root").long("root");
    let id = || Arg::new("id").long("id").required(true);
    let page = |cmd: Command| {
        cmd.arg(
            Arg::new("offset")
                .long("offset")
                .default_value("0")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("limit")
                .long("limit")
                .default_value("20")
                .value_parser(clap::value_parser!(u32).range(1..=100)),
        )
    };
    Command::new("tasks").about("Track research questions, experiments and reviews with durable dependencies and decision history")
        .subcommand(Command::new("add").about("Create a fresh research task with fixed prerequisites; run nothing")
            .arg(root()).arg(id()).arg(Arg::new("title").long("title").required(true))
            .arg(Arg::new("kind").long("kind").required(true).value_parser(["plan","research","experiment","review","operate"]))
            .arg(Arg::new("depends-on").long("depends-on").action(ArgAction::Append))
            .arg(Arg::new("note").long("note").help("Optional UTF-8 brief/evidence file, at most 16 KiB"))
            .arg(Arg::new("note-text").long("note-text").conflicts_with("note").help("Exact inline brief/evidence text, at most 16 KiB")))
        .subcommand(Command::new("update").about("Append a deliberate task state and evidence/revisit note; never alter an experiment")
            .arg(root()).arg(id()).arg(Arg::new("state").long("state").required(true).value_parser(["open","active","blocked","done","retired"]))
            .arg(Arg::new("note").long("note"))
            .arg(Arg::new("note-text").long("note-text"))
            .group(ArgGroup::new("decision").args(["note","note-text"]).required(true)))
        .subcommand(page(Command::new("list").about("Inspect verified tasks and ready prerequisites").arg(root())))
        .subcommand(page(Command::new("graph").about("Project the verified plan/research/task DAG with prerequisite edges and exact event citations").arg(root())))
        .subcommand(page(Command::new("read").about("Read verified decision history for one task").arg(root()).arg(id())))
}
fn store(root: &Path, write: bool) -> Result<(PathBuf, fs::File)> {
    if write {
        fs::create_dir_all(root)?;
    }
    let root = inv::root(root)?;
    let dir = if write {
        inv::subdirectory(&root, "tasks")?
    } else {
        inv::checked(&root, "tasks")?
    };
    inv::root(&dir)?;
    let p = dir.join("tasks.lock");
    match fs::symlink_metadata(&p) {
        Ok(_) => {
            inv::digest(&p)?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    let file = fs::OpenOptions::new()
        .create(write)
        .read(true)
        .write(write)
        .truncate(false)
        .open(p)?;
    if write {
        fs2::FileExt::lock_exclusive(&file)?;
    } else {
        fs2::FileExt::lock_shared(&file)?;
    }
    Ok((dir, file))
}
fn history(dir: &Path, id: &str) -> Result<Vec<(Event, String)>> {
    inv::name(id)?;
    let dir = inv::checked(dir, id)?;
    inv::root(&dir)?;
    let mut files = fs::read_dir(&dir)?
        .map(|e| e.map(|v| v.file_name()))
        .collect::<std::io::Result<Vec<_>>>()?;
    files.sort();
    if files.is_empty() || files.len() % 2 != 0 || files.len() > 8192 {
        return Err("incomplete or oversized task event history".into());
    }
    let mut out: Vec<(Event, String)> = Vec::new();
    for sequence in 0..files.len() / 2 {
        let base = format!("event-{sequence:06}");
        if files[sequence * 2] != format!("{base}.json").as_str()
            || files[sequence * 2 + 1] != format!("{base}.sha256").as_str()
        {
            return Err("task event member set/sequence changed".into());
        }
        let path = dir.join(format!("{base}.json"));
        let event: Event = inv::read_sealed(&path)?;
        if event.schema_version != 1
            || event.id != id
            || event.sequence != sequence
            || event.previous != out.last().map(|(_, sha)| sha.clone())
        {
            return Err("task event chain/identity mismatch".into());
        }
        if let Some((first, _)) = out.first()
            && (event.title != first.title
                || event.kind != first.kind
                || event.depends_on != first.depends_on)
        {
            return Err("fixed task brief or dependencies changed".into());
        }
        out.push((event, inv::digest(&path)?));
    }
    Ok(out)
}
fn all(dir: &Path) -> Result<Vec<(Event, String)>> {
    let mut ids = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "invalid task name")?;
        if name == "tasks.lock" {
            continue;
        }
        inv::name(&name)?;
        ids.push(name);
        if ids.len() > 4096 {
            return Err("task scan exceeds 4096 members".into());
        }
    }
    ids.sort();
    ids.into_iter()
        .map(|id| {
            history(dir, &id)?
                .pop()
                .ok_or_else(|| "task has no event".into())
        })
        .collect()
}
fn meta(e: &Event, sha: &str, all: &[(Event, String)]) -> Value {
    let ready = e.depends_on.iter().all(|id| {
        all.iter()
            .any(|(dep, _)| &dep.id == id && dep.state == "done")
    });
    json!({"id":e.id,"title":e.title,"kind":e.kind,"state":e.state,"depends_on":e.depends_on,"prerequisites_done":ready,"ready":ready&&e.state=="open","sequence":e.sequence,"event_sha256":sha,"recorded_at_ms":e.recorded_at_ms})
}
pub fn catalog(root: &Path) -> Result<Vec<Value>> {
    let (dir, _lock) = store(root, false)?;
    let all = all(&dir)?;
    Ok(all.iter().map(|(e, sha)| meta(e, sha, &all)).collect())
}
/// Immutable task revisions and prerequisite/history edges, using an Ontology-style
/// entity/relation projection. A state board is only a view of these objects.
pub fn graph_objects(root: &Path) -> Result<(Vec<Value>, Vec<Value>)> {
    let (dir, _lock) = store(root, false)?;
    let current = all(&dir)?;
    let mut histories = std::collections::BTreeMap::new();
    let mut count = 0;
    for (event, _) in &current {
        let revisions = history(&dir, &event.id)?;
        count += revisions.len();
        if count > 4096 {
            return Err("research graph exceeds 4096 task revisions".into());
        }
        histories.insert(event.id.clone(), revisions);
    }
    let mut objects = Vec::new();
    let mut edges = Vec::new();
    for (id, revisions) in &histories {
        for (i, (e, sha)) in revisions.iter().enumerate() {
            let key = format!("task:{id}@{sha}");
            objects.push(json!({"id":key,"logical_id":id,"node_type":"entity","kind":e.kind,"label":e.title,"sha256":sha,"evidence":"observed sealed task event; decisions are declared","text":e.note,"recorded_at_ms":e.recorded_at_ms,"attributes":{"state":e.state,"current":i+1==revisions.len(),"sequence":i,"read_argv":["paperhands","tasks","read","--root",root,"--id",id]}}));
            if i > 0 {
                edges.push(json!({"source":format!("task:{id}@{}",revisions[i-1].1),"target":key,"predicate":"revised_into","evidence":sha,"weights":{"similarity":null,"priority":null},"origin":"verified task event chain"}));
            }
            if i == 0 {
                for dep in &e.depends_on {
                    let first = histories
                        .get(dep)
                        .and_then(|h| h.first())
                        .ok_or("task prerequisite missing from graph")?;
                    edges.push(json!({"source":format!("task:{dep}@{}",first.1),"target":key,"predicate":"prerequisite_of","evidence":sha,"weights":{"similarity":null,"priority":null},"origin":"fixed initial task prerequisite"}));
                }
            }
        }
    }
    Ok((objects, edges))
}
pub fn initialize(root: &Path) -> Result<()> {
    store(root, true)?;
    Ok(())
}
fn note(path: Option<&String>) -> Result<String> {
    match path {
        None => Ok(String::new()),
        Some(path) => {
            let path = Path::new(path);
            if fs::symlink_metadata(path)?.len() > 16 * 1024 {
                return Err("task note exceeds 16 KiB".into());
            }
            let text = String::from_utf8(inv::read_bytes(path)?)?;
            if text.trim().is_empty() || text.len() > 16 * 1024 {
                return Err("task note must contain 1..16,384 UTF-8 bytes".into());
            }
            Ok(text)
        }
    }
}
pub fn execute(args: &ArgMatches) -> Result<Value> {
    let (action, args) = args.subcommand().ok_or("choose a tasks operation")?;
    let get = |key: &str| args.get_one::<String>(key).expect("required argument");
    let root = crate::project::resolve(args.get_one::<String>("root").map(String::as_str))?;
    let (dir, _lock) = store(&root, matches!(action, "add" | "update"))?;
    let result: Result<Value> = match action {
        "add" | "update" => {
            let id = get("id");
            inv::name(id)?;
            let all = all(&dir)?;
            let (mut event, previous) = if action == "add" {
                let title = get("title");
                if title.trim().is_empty()
                    || title.len() > 200
                    || title.chars().any(char::is_control)
                {
                    return Err(
                        "task title must contain 1..200 UTF-8 bytes without controls".into(),
                    );
                }
                let deps = args
                    .get_many::<String>("depends-on")
                    .map(|v| v.cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                let mut unique = std::collections::BTreeSet::new();
                for dep in &deps {
                    inv::name(dep)?;
                    if dep == id || !unique.insert(dep) || !all.iter().any(|(e, _)| &e.id == dep) {
                        return Err("prerequisites must be unique existing tasks; self/future references are refused".into());
                    }
                }
                (
                    Event {
                        schema_version: 1,
                        id: id.clone(),
                        title: title.clone(),
                        kind: get("kind").clone(),
                        depends_on: deps,
                        sequence: 0,
                        previous: None,
                        state: "open".into(),
                        note: String::new(),
                        recorded_at_ms: inv::now(),
                    },
                    None,
                )
            } else {
                let (mut e, sha) = history(&dir, id)?.pop().ok_or("missing task history")?;
                e.sequence += 1;
                e.state = get("state").clone();
                (e, Some(sha))
            };
            if event.sequence >= 4096 {
                return Err("task event limit reached".into());
            }
            event.previous = previous;
            event.note = if let Some(text) = args.get_one::<String>("note-text") {
                if text.trim().is_empty() || text.len() > 16 * 1024 {
                    return Err("task note must contain 1..16,384 UTF-8 bytes".into());
                }
                text.clone()
            } else {
                note(args.get_one::<String>("note"))?
            };
            event.recorded_at_ms = inv::now();
            if matches!(event.state.as_str(), "active" | "done")
                && !event
                    .depends_on
                    .iter()
                    .all(|id| all.iter().any(|(e, _)| &e.id == id && e.state == "done"))
            {
                return Err("task prerequisites are not done; record a blocked/open/retired disposition instead".into());
            }
            let task = dir.join(id);
            if action == "add" {
                fs::create_dir(&task)?;
            }
            let sha = inv::sealed_json(
                &task.join(format!("event-{:06}.json", event.sequence)),
                &event,
            )?;
            Ok(
                json!({"task":meta(&event,&sha,&all),"append_only":true,"experiment_mutations":false,"execution_started":false}),
            )
        }
        "list" | "graph" => {
            let all = all(&dir)?;
            let offset = *args.get_one::<usize>("offset").unwrap();
            let limit = *args.get_one::<u32>("limit").unwrap() as usize;
            let items = all
                .iter()
                .skip(offset)
                .take(limit)
                .map(|(e, sha)| meta(e, sha, &all))
                .collect::<Vec<_>>();
            let mut value = json!({"items":items,"total":all.len(),"offset":offset,"limit":limit,"integrity_status":"verified"});
            if action == "graph" {
                let edges=all.iter().skip(offset).take(limit).flat_map(|(e,sha)|e.depends_on.iter().map(move|dep|json!({"source":dep,"target":e.id,"predicate":"prerequisite_of","evidence":{"task":e.id,"event_sha256":sha},"weights":{"similarity":null,"priority":null},"authority":"explicit task dependency; not financial or causal evidence"}))).collect::<Vec<_>>();
                value["schema"] = json!("paperhands.task-dag/v1");
                value["edges"] = json!(edges);
                value["coverage"] = json!({"all_task_histories_verified":true,"nodes_paged":true,"edge_sources_may_be_outside_page":true});
                value["acyclic"] = json!(true);
            }
            Ok(value)
        }
        "read" => {
            let history = history(&dir, get("id"))?;
            let offset = *args.get_one::<usize>("offset").unwrap();
            let limit = *args.get_one::<u32>("limit").unwrap() as usize;
            Ok(
                json!({"events":history.iter().skip(offset).take(limit).map(|(e,sha)|json!({"event":e,"sha256":sha})).collect::<Vec<_>>(),"total_events":history.len(),"offset":offset,"limit":limit,"integrity_status":"verified"}),
            )
        }
        _ => Err("unknown task operation".into()),
    };
    let mut data = result?;
    data["root"] = json!(inv::root(&root)?);
    Ok(data)
}
