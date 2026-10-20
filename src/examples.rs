//! Versioned inert adapters travel inside the executable and the source crate.
use crate::{Result, research::inventory as inv};
use clap::{Arg, ArgMatches, Command};
use serde_json::{Value, json};
use std::{fs, path::Path};

pub const FILES: &[(&str, &str, &str)] = &[
    (
        "decision-evals",
        "decision_evals.py",
        include_str!("../examples/decision_evals.py"),
    ),
    (
        "research-desk",
        "research_desk.py",
        include_str!("../examples/research_desk.py"),
    ),
    (
        "fixture-evaluator",
        "fixture_evaluator.py",
        include_str!("../examples/fixture_evaluator.py"),
    ),
    (
        "fixture-plans",
        "make_fixture.py",
        include_str!("../examples/make_fixture.py"),
    ),
    (
        "winnow-cases",
        "winnow_cases.py",
        include_str!("../examples/winnow_cases.py"),
    ),
    (
        "winnow-closed",
        "winnow_closed.py",
        include_str!("../examples/winnow_closed.py"),
    ),
    (
        "verify-winnow",
        "verify_winnow.py",
        include_str!("../examples/verify_winnow.py"),
    ),
];
pub fn command() -> Command {
    Command::new("examples")
        .about("Discover and extract versioned inert evaluator adapters embedded in this binary")
        .subcommand(Command::new("list").about(
            "List embedded scripts with exact content hashes and external runtime requirements",
        ))
        .subcommand(
            Command::new("read")
                .about("Read an embedded adapter; execute nothing")
                .arg(Arg::new("id").required(true)),
        )
        .subcommand(
            Command::new("export")
                .about("Write the companion script set into a fresh directory; execute nothing")
                .arg(Arg::new("out").long("out").required(true)),
        )
}
pub fn manifest() -> Vec<Value> {
    FILES.iter().map(|(id,name,text)|json!({"id":id,"file":name,"bytes":text.len(),"sha256":inv::hash(text.as_bytes()),"runtime":"external Python 3; consumer evaluators/interpreters remain supplied dependencies"})).collect()
}
pub fn execute(args: &ArgMatches) -> Result<Value> {
    let (action, args) = args
        .subcommand()
        .ok_or("choose examples list, read or export")?;
    match action {
        "list" => Ok(json!({"items":manifest(),"executed":false})),
        "read" => {
            let id = args.get_one::<String>("id").unwrap();
            let (_, file, text) = FILES
                .iter()
                .find(|(name, _, _)| *name == id)
                .ok_or("unknown embedded example ID; use examples list")?;
            Ok(
                json!({"id":id,"file":file,"text":text,"sha256":inv::hash(text.as_bytes()),"executed":false}),
            )
        }
        "export" => {
            let path = Path::new(args.get_one::<String>("out").unwrap());
            fs::create_dir(path)?;
            for (_, name, text) in FILES {
                inv::exclusive(&path.join(name), text.as_bytes())?;
            }
            Ok(
                json!({"directory":inv::root(path)?,"files":manifest(),"executed":false,"scope":"inert source examples; no runtime installation, capture, inference, network or orders"}),
            )
        }
        _ => Err("unknown examples operation".into()),
    }
}
