//! A natural-language front door grounded in recorded state and retained sources.
use crate::{Result, desk, memory};
use clap::{Arg, ArgMatches, Command};
use clap_how_to::{Document, Index, Source};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

pub fn command() -> Command {
    Command::new("ask").about("Talk to the research companion: what next, current work, focus, remembered evidence, or how to use a command")
        .arg(Arg::new("question").required(true).help("A quoted natural-language question; retrieves grounded context, never executes a suggested action"))
        .arg(Arg::new("root").long("root").help("Research store; defaults to PAPERHANDS_WORKSPACE or the nearest paperhands.toml project"))
}
pub fn route_documents() -> Vec<Document> {
    [
        ("next","What should I do next? What needs doing? What research or review needs attention? Show ready tasks, unmet prerequisites and the next useful question."),
        ("state","What are we working on? What is running or queued? What happened in our experiments? Show current activity, open tasks, crashes, completed work and evidence states."),
        ("focus","What is our focus? What is our direction, mission, purpose and research mandate? Show the latest retained direction and current priorities."),
        ("how","How do I do this? How can I use a command, create or run an experiment, inspect help, configure MCP, install skills or export an adapter? Show shipped instructions and exact command help."),
        ("remember","What did we learn or find? What do we know about a subject? Search research notes, observations, hypotheses, sources, lessons and questions. Retrieve original evidence and citations.")
    ].into_iter().map(|(id,text)|Document{title:id.into(),text:text.into(),source:Source::Documentation{source:id.into(),section:id.into(),line:1,url:None}}).collect()
}
fn hint(argv: Value, purpose: &str) -> Value {
    json!({"argv":argv,"purpose":purpose})
}
fn help_hint(item: &Value) -> Option<Value> {
    if item["kind"] == "command" {
        let argv = item["help"]
            .as_str()?
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        Some(hint(json!(argv), "Read the cited command help"))
    } else {
        Some(hint(
            json!([
                "paperhands",
                "docs",
                "read",
                item["source"],
                "--start-line",
                item["line"].to_string(),
                "--lines",
                "60"
            ]),
            "Read the cited bundled source",
        ))
    }
}
fn workspace(args: &ArgMatches) -> Result<PathBuf> {
    let current = std::env::current_dir()?;
    if args.get_one::<String>("root").is_none()
        && std::env::var_os("PAPERHANDS_WORKSPACE").is_none()
        && crate::project::discover(&current)?.is_none()
    {
        return Ok(current.join(".paperhands"));
    }
    crate::project::resolve(args.get_one::<String>("root").map(String::as_str))
}
pub fn execute(args: &ArgMatches) -> Result<Value> {
    let question = args.get_one::<String>("question").unwrap();
    clap_how_to::validate_query(question)?;
    let root = workspace(args)?;
    let prepared = serde_json::from_str(include_str!("../assets/companion.json"))?;
    let routes = Index::from_prepared(prepared)?.search(question, 3)?;
    let route = routes
        .first()
        .and_then(|r| match &r.document.source {
            Source::Documentation { source, .. } => Some(source.as_str()),
            _ => None,
        })
        .unwrap_or("how");
    let help = crate::search(question, "all")?;
    let available = match fs::symlink_metadata(&root) {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(e.into()),
    };
    let mut context = if available {
        Some(desk::inspect(&root, 0, 40)?)
    } else {
        None
    };
    let focus = if available && fs::symlink_metadata(root.join("memory")).is_ok() {
        match memory::direction(&root) {
            Ok(value) => value,
            Err(error) => {
                if let Some(context) = &mut context {
                    context["gaps"]
                        .as_array_mut()
                        .ok_or("missing desk gaps")?
                        .push(json!({"collection":"direction","error":error.to_string()}));
                }
                None
            }
        }
    } else {
        None
    };
    let notes = if available && fs::symlink_metadata(root.join("memory")).is_ok() {
        let args = vec![
            "memory".into(),
            "search".into(),
            "--root".into(),
            root.to_string_lossy().into_owned(),
            question.clone(),
        ];
        let result = crate::runtime::invoke(&args, &json!({}));
        if result.exit_code == 0 {
            result.value["data"].clone()
        } else {
            json!({"items":[],"integrity_status":"failed","error":result.value["error"]})
        }
    } else {
        json!({"items":[],"scope":"no retained memory in the selected workspace"})
    };
    if notes["integrity_status"] == "failed"
        && let Some(context) = &mut context
    {
        context["gaps"].as_array_mut().ok_or("missing desk gaps")?.push(json!({"collection":"memory_search","error":notes["error"],"coverage":"unknown; query failure is not an empty archive"}));
    }
    let active = context
        .as_ref()
        .and_then(|c| c["tasks"].as_array())
        .into_iter()
        .flatten()
        .filter(|t| t["state"] == "active")
        .cloned()
        .collect::<Vec<_>>();
    let frontier = context
        .as_ref()
        .and_then(|c| c["frontier"].as_array())
        .cloned()
        .unwrap_or_default();
    let gaps = context
        .as_ref()
        .and_then(|c| c["gaps"].as_array())
        .cloned()
        .unwrap_or_default();
    let mut next = Vec::new();
    let answer = if !gaps.is_empty() {
        "There are retained retrieval or integrity gaps. Inspect those gaps before relying on this workspace's state or evidence.".to_owned()
    } else if !available && route != "how" {
        let initialize = if args.get_one::<String>("root").is_some()
            || std::env::var_os("PAPERHANDS_WORKSPACE").is_some()
        {
            json!(["paperhands", "init", "--root", root])
        } else {
            json!(["paperhands", "init"])
        };
        next.push(hint(
            initialize,
            "Initialize the selected research workspace",
        ));
        "I do not have a research store at the selected path yet. Initialize it, then add direction, questions and tasks. Shipped help is available now.".to_owned()
    } else {
        match route {
            "next" => {
                for item in frontier.iter().take(10) {
                    next.push(hint(
                        item["argv"].clone(),
                        item["reason"]
                            .as_str()
                            .unwrap_or("Inspect the recorded work"),
                    ));
                }
                if let Some(item) = frontier.first() {
                    format!(
                        "Next recorded work: {}. {}. Choose within your mandate; this frontier does not rank financial scores.",
                        item["title"]
                            .as_str()
                            .or(item["id"].as_str())
                            .unwrap_or("inspect the frontier"),
                        item["reason"].as_str().unwrap_or("Inspect the evidence")
                    )
                } else {
                    next.push(hint(
                        json!(["paperhands", "help", "tasks", "add"]),
                        "Record the next research question or task",
                    ));
                    "No ready work is recorded in the inspected scope. Add a question or task, or inspect the other evidence pages; this does not mean all research is finished.".into()
                }
            }
            "focus" => {
                if let Some(focus) = &focus {
                    next.push(hint(
                        json!([
                            "paperhands",
                            "memory",
                            "read",
                            "--root",
                            root,
                            focus["record"]["id"]
                        ]),
                        "Read the retained direction",
                    ));
                    format!(
                        "The latest retained direction says: {}",
                        focus["text"]
                            .as_str()
                            .unwrap_or("")
                            .chars()
                            .take(800)
                            .collect::<String>()
                    )
                } else {
                    next.push(hint(
                        json!(["paperhands", "help", "memory", "add"]),
                        "Record a direction note with --kind direction",
                    ));
                    "No direction note is recorded. Add a direction note to preserve your focus, scope and research budget.".into()
                }
            }
            "state" => {
                next.push(hint(
                    json!(["paperhands", "desk", "--root", root]),
                    "Inspect current tasks, evidence coverage and work frontier",
                ));
                let rows = context
                    .as_ref()
                    .and_then(|c| c["rows"].as_array())
                    .cloned()
                    .unwrap_or_default();
                let running = rows
                    .iter()
                    .filter(|r| r["process_status"] == "running")
                    .count();
                let queued = rows
                    .iter()
                    .filter(|r| r["process_status"] == "queued")
                    .count();
                let names = active
                    .iter()
                    .filter_map(|t| t["title"].as_str())
                    .collect::<Vec<_>>()
                    .join("; ");
                format!(
                    "Active tasks in the returned task preview: {}. This inspected evidence page has {running} running and {queued} queued attempts; other retained statuses and coverage are in the desk.",
                    if names.is_empty() {
                        "none recorded in this scope"
                    } else {
                        &names
                    }
                )
            }
            "remember" => {
                if let Some(item) = notes["items"].as_array().and_then(|items| items.first()) {
                    next.push(hint(
                        item["read_argv"].clone(),
                        "Read the original research note",
                    ));
                    format!(
                        "A retained note says: {}",
                        item["text"]
                            .as_str()
                            .unwrap_or("")
                            .chars()
                            .take(800)
                            .collect::<String>()
                    )
                } else {
                    "I found no retained note to cite for that question. The shipped help and current workspace context are included; the research question remains open.".into()
                }
            }
            _ => {
                if let Some(item) = help["items"].as_array().and_then(|items| items.first()) {
                    if let Some(hint) = help_hint(item) {
                        next.push(hint);
                    }
                    format!(
                        "Relevant shipped guidance: {}. {}",
                        item["title"].as_str().unwrap_or("command help"),
                        item["text"]
                            .as_str()
                            .unwrap_or("")
                            .chars()
                            .take(600)
                            .collect::<String>()
                    )
                } else {
                    "No shipped guidance matched. Keep the missing capability explicit rather than inventing a command.".into()
                }
            }
        }
    };
    Ok(
        json!({"question":question,"answer":answer,"route":route,"route_candidates":routes,"route_semantics":"offline semantic navigation, not confidence or execution authority","workspace":root,"workspace_available":available,"focus":focus,"working_on":active,"context":context,"notes":notes,"help":help,"suggested_actions":next,"offline":true,"generated_by":"recorded-state templates and cited retrieval; external agent supplies reasoning","executed_actions":false,"visibility":"private-local-research"}),
    )
}
