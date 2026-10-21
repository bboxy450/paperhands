//! Shared command execution for the terminal and MCP.
use crate::{Result, VERSION, command, envelope};
use clap::ArgMatches;
use serde_json::{Value, json};

pub fn execute(matches: &ArgMatches, skill: Value) -> Result<(&'static str, Value)> {
    let mut app = command();
    let mut leaf = matches;
    while let Some((name, child)) = leaf.subcommand() {
        app = app
            .find_subcommand_mut(name)
            .expect("parsed command")
            .clone();
        leaf = child;
    }
    if matches.subcommand_name().is_none() {
        return Ok(("help", crate::help(&mut app)));
    }
    match matches.subcommand() {
        Some(("status", _)) => Ok((
            "status",
            json!({
                "stage":"local-experiments", "trading_enabled":false,
                "capabilities":["semantic-help", "bundled-docs", "automatic-agent-skill", "retained-inventory", "bounded-external-evaluators", "bounded-batches", "explicit-comparison-and-review", "stdio-mcp", "project-identity", "grounded-companion", "research-memory", "research-dag", "weighted-provenance", "semantic-neighbors-and-clusters", "research-tasks-and-frontier", "embedded-examples", "evidence-views", "embedded-pele", "bounded-decision-judges", "offline-threshold-and-calibration-evals", "native-harness-setup", "bounded-research-wakes", "owned-cron"],
                "planned_venues":["polymarket", "robinhood", "other-platforms"],
                "specification":"local-research-datapack", "decision_evals":{"provider":"typesafe","calls":"only explicit judge run; bounded and pinned","analysis":"saved scores, grouped folds, constrained thresholds and measured calibration; offline"}, "skills":skill
            }),
        )),
        Some(("bundle", _)) => Ok(("bundle", crate::bundle::manifest())),
        Some(("mcp-config", _)) => Ok(("mcp-config", crate::mcp::config()?)),
        Some(("mcp", _)) => Err(
            "mcp is a transport entry point; launch paperhands mcp as a separate process".into(),
        ),
        Some(("memory", args)) => Ok(("memory", crate::memory::execute(args)?)),
        Some(("tasks", args)) => Ok(("tasks", crate::tasks::execute(args)?)),
        Some(("examples", args)) => Ok(("examples", crate::examples::execute(args)?)),
        Some(("graph", args)) => Ok(("graph", crate::graph::execute(args)?)),
        Some(("ask", args)) => Ok(("ask", crate::companion::execute(args)?)),
        Some(("judge", args)) => {
            if args.subcommand_name().is_none() {
                Ok(("help", crate::help(&mut app)))
            } else {
                Ok(("judge", crate::judge::execute(args)?))
            }
        }
        Some(("setup", args)) => Ok(("setup", crate::setup::execute(args)?)),
        Some(("automation", args)) => {
            if args.subcommand_name().is_some() {
                Ok(("automation", crate::automation::execute(args)?))
            } else {
                Ok(("help", crate::help(&mut app)))
            }
        }
        Some(("init", args)) => Ok((
            "init",
            if let Some(root) = args.get_one::<String>("root") {
                crate::desk::initialize(std::path::Path::new(root))?
            } else {
                crate::project::bootstrap(
                    std::path::Path::new(
                        args.get_one::<String>("project-dir")
                            .map(String::as_str)
                            .unwrap_or("."),
                    ),
                    args.get_one::<String>("name").map(String::as_str),
                    args.get_one::<String>("research").map(String::as_str),
                    !args.get_flag("no-setup")
                        && std::env::var_os("PAPERHANDS_SKILLS_DIR").is_none(),
                )?
            },
        )),
        Some(("project", args)) => Ok((
            "project",
            crate::project::inspect(std::path::Path::new(args.get_one::<String>("dir").unwrap()))?,
        )),
        Some(("desk", args)) => Ok((
            "desk",
            crate::view::desk(
                &crate::project::resolve(args.get_one::<String>("root").map(String::as_str))?,
                *args.get_one::<usize>("offset").unwrap(),
                *args.get_one::<u32>("limit").unwrap() as usize,
            )?,
        )),
        Some(("view", args)) if args.subcommand_name().is_some() => {
            Ok(("view", crate::view::execute(args)?))
        }
        Some(("how-to", args)) => Ok((
            "how-to",
            crate::search(args.get_one::<String>("query").unwrap(), "all")?,
        )),
        Some((family @ ("inventory" | "experiment" | "batch"), args)) => {
            let name = match family {
                "inventory" => "inventory",
                "experiment" => "experiment",
                _ => "batch",
            };
            if args.subcommand_name().is_none() {
                Ok(("help", crate::help(&mut app)))
            } else {
                Ok((name, crate::research::cli::execute(family, args)?))
            }
        }
        Some(("cli", args)) | Some(("docs", args)) => {
            let scope = if matches.subcommand_name() == Some("cli") {
                "commands"
            } else {
                "docs"
            };
            match args.subcommand() {
                Some(("search", query)) => Ok((
                    if scope == "commands" {
                        "cli search"
                    } else {
                        "docs search"
                    },
                    crate::search(query.get_one::<String>("query").unwrap(), scope)?,
                )),
                Some(("list", _)) => Ok((
                    "docs list",
                    json!({"guides": crate::GUIDES.iter().map(|(id,path,text)|
                    json!({"id":id,"source":path,"title":text.lines().next().unwrap_or(id).trim_start_matches('#').trim()})
                ).collect::<Vec<_>>()}),
                )),
                Some(("read", args)) => {
                    let id = args.get_one::<String>("id").unwrap();
                    let (_, path, text) = crate::GUIDES
                        .iter()
                        .find(|(guide, _, _)| guide == id)
                        .ok_or("unknown guide ID; use paperhands docs list")?;
                    let start = *args.get_one::<u32>("start-line").unwrap() as usize;
                    let lines = text.lines().collect::<Vec<_>>();
                    if start > lines.len().max(1) {
                        return Err("start-line exceeds the guide's source lines".into());
                    }
                    let end = start
                        .saturating_sub(1)
                        .saturating_add(
                            args.get_one::<u32>("lines")
                                .map(|n| *n as usize)
                                .unwrap_or(lines.len()),
                        )
                        .min(lines.len());
                    let page = if start == 1 && args.get_one::<u32>("lines").is_none() {
                        (*text).to_owned()
                    } else {
                        lines[start - 1..end].join("\n")
                    };
                    Ok((
                        "docs read",
                        json!({"id":id,"source":path,"text":page,"start_line":start,"end_line":end,"total_lines":lines.len(),"next_line":if end<lines.len(){Some(end+1)}else{None}}),
                    ))
                }
                _ => Ok(("help", crate::help(&mut app))),
            }
        }
        Some(("skills", _)) => Ok(("skills", json!({"installation":skill,"text":crate::SKILL}))),
        Some(("help", args)) => {
            let mut app = command();
            app.build();
            if let Some(path) = args.get_many::<String>("path") {
                for name in path {
                    app = app
                        .find_subcommand(name)
                        .ok_or("unknown command help path")?
                        .clone();
                }
            }
            Ok(("help", crate::help(&mut app)))
        }
        _ => Ok(("help", crate::help(&mut app))),
    }
}

pub struct Outcome {
    pub value: Value,
    pub exit_code: i32,
}

pub fn failure(message: &str, exit_code: i32) -> Outcome {
    Outcome {
        value: json!({"schema":"paperhands/v1","version":VERSION,"ok":false,"error":{"message":message,"exit_code":exit_code}}),
        exit_code,
    }
}

/// Parse exact arguments in-process. No subprocess, shell, network or new skill installation.
pub fn invoke(argv: &[String], skill: &Value) -> Outcome {
    let args = std::iter::once("paperhands").chain(argv.iter().map(String::as_str));
    let matches = match command().try_get_matches_from(args) {
        Ok(matches) => matches,
        Err(error) => {
            let (name, data) = match error.kind() {
                clap::error::ErrorKind::DisplayHelp => ("help", json!({"help":error.to_string()})),
                clap::error::ErrorKind::DisplayVersion => {
                    ("version", json!({"name":"paperhands","version":VERSION}))
                }
                _ => return failure(&error.to_string(), 2),
            };
            return Outcome {
                value: envelope(name, data),
                exit_code: 0,
            };
        }
    };
    match execute(&matches, skill.clone()) {
        Ok((name, data)) => {
            let exit_code = data.get("exit_code").and_then(Value::as_i64).unwrap_or(0) as i32;
            let mut value = envelope(name, data);
            value["ok"] = json!(exit_code == 0);
            Outcome { value, exit_code }
        }
        Err(error) => {
            let code = if error
                .downcast_ref::<clap_how_to::Error>()
                .is_some_and(|e| matches!(e, clap_how_to::Error::Encoder(_)))
            {
                1
            } else {
                2
            };
            failure(&error.to_string(), code)
        }
    }
}

/// Navigation hints are exact argv arrays, never shell strings or automatic actions.
pub fn next(name: &str, data: &Value) -> Vec<Value> {
    let hint = |argv: Vec<String>, purpose: &str| json!({"argv":argv,"purpose":purpose});
    let args = |a: &[&str]| a.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    match name {
        "ask" => data["suggested_actions"]
            .as_array()
            .cloned()
            .unwrap_or_default(),
        "init" => vec![hint(
            vec![
                "paperhands".into(),
                "desk".into(),
                "--root".into(),
                data["root"].as_str().unwrap_or(".").into(),
            ],
            "Inspect the initialized research desk",
        )],
        "desk" => data["desk"]["frontier"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|item| {
                item.get("argv")
                    .map(|argv| json!({"argv":argv,"purpose":item["reason"]}))
            })
            .collect(),
        "memory" if data["record"]["id"].is_string() => vec![hint(
            vec![
                "paperhands".into(),
                "memory".into(),
                "read".into(),
                "--root".into(),
                data["root"].as_str().unwrap_or(".").into(),
                data["record"]["id"].as_str().unwrap().into(),
            ],
            "Read the retained research note",
        )],
        "tasks" if data["task"]["id"].is_string() => vec![hint(
            vec![
                "paperhands".into(),
                "tasks".into(),
                "read".into(),
                "--root".into(),
                data["root"].as_str().unwrap_or(".").into(),
                "--id".into(),
                data["task"]["id"].as_str().unwrap().into(),
            ],
            "Read the durable task decision history",
        )],
        "docs list" => data["guides"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|guide| guide["id"].as_str())
            .map(|id| {
                hint(
                    args(&["paperhands", "docs", "read", id]),
                    "Read a bundled guide",
                )
            })
            .collect(),
        "docs read" if data["next_line"].is_u64() => vec![hint(
            args(&[
                "paperhands",
                "docs",
                "read",
                data["id"].as_str().unwrap_or("cli"),
                "--start-line",
                &data["next_line"].to_string(),
                "--lines",
                "60",
            ]),
            "Read the next source page",
        )],
        "status" | "help" | "bundle" => vec![
            hint(
                args(&["paperhands", "docs", "read", "agents"]),
                "Read the CLI/MCP and living research memory contract",
            ),
            hint(
                args(&["paperhands", "mcp-config"]),
                "Inspect the MCP launch configuration",
            ),
            hint(
                args(&["paperhands", "help", "experiment"]),
                "Inspect lifecycle commands",
            ),
        ],
        _ => vec![],
    }
}
