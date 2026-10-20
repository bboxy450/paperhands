use clap::ArgMatches;
use paperhands::{Result, VERSION, command, envelope};
use serde_json::{Value, json};

fn execute(matches: &ArgMatches, skill: Value) -> Result<(&'static str, Value)> {
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
        return Ok(("help", paperhands::help(&mut app)));
    }
    match matches.subcommand() {
        Some(("status", _)) => Ok((
            "status",
            json!({
                "stage":"local-experiments", "trading_enabled":false,
                "capabilities":["semantic-help", "bundled-docs", "automatic-agent-skill", "retained-inventory", "bounded-external-evaluators", "bounded-batches", "explicit-comparison-and-review"],
                "planned_venues":["polymarket", "robinhood", "other-platforms"],
                "specification":"pending", "skills":skill
            }),
        )),
        Some(("how-to", args)) => Ok((
            "how-to",
            paperhands::search(args.get_one::<String>("query").unwrap(), "all")?,
        )),
        Some((family @ ("inventory" | "experiment" | "batch"), args)) => {
            let name = match family {
                "inventory" => "inventory",
                "experiment" => "experiment",
                _ => "batch",
            };
            if args.subcommand_name().is_none() {
                Ok(("help", paperhands::help(&mut app)))
            } else {
                Ok((name, paperhands::research::cli::execute(family, args)?))
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
                    paperhands::search(query.get_one::<String>("query").unwrap(), scope)?,
                )),
                Some(("list", _)) => Ok((
                    "docs list",
                    json!({"guides": paperhands::GUIDES.iter().map(|(id,path,text)|
                    json!({"id":id,"source":path,"title":text.lines().next().unwrap_or(id).trim_start_matches('#').trim()})
                ).collect::<Vec<_>>()}),
                )),
                Some(("read", args)) => {
                    let id = args.get_one::<String>("id").unwrap();
                    let (_, path, text) = paperhands::GUIDES
                        .iter()
                        .find(|(guide, _, _)| guide == id)
                        .ok_or("unknown guide ID; use paperhands docs list")?;
                    Ok(("docs read", json!({"id":id,"source":path,"text":text})))
                }
                _ => Ok(("help", paperhands::help(&mut app))),
            }
        }
        Some(("skills", _)) => Ok((
            "skills",
            json!({"installation":skill,"text":paperhands::SKILL}),
        )),
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
            Ok(("help", paperhands::help(&mut app)))
        }
        _ => Ok(("help", paperhands::help(&mut app))),
    }
}

fn fail(message: &str, human: bool, code: i32) -> ! {
    if human {
        eprintln!("{message}");
    } else {
        eprintln!(
            "{}",
            json!({"schema":"paperhands/v1","version":VERSION,"ok":false,"error":{"message":message,"exit_code":code}})
        );
    }
    std::process::exit(code)
}

fn main() {
    let args = std::env::args_os().collect::<Vec<_>>();
    let human = args
        .iter()
        .take_while(|arg| *arg != "--")
        .any(|arg| arg == "--human");
    let skill = paperhands::refresh_skill()
        .unwrap_or_else(|error| fail(&format!("skill installation: {error}"), human, 1));
    let matches = match command().try_get_matches_from(args) {
        Ok(matches) => matches,
        Err(error) => {
            let (name, data) = match error.kind() {
                clap::error::ErrorKind::DisplayHelp => ("help", json!({"help":error.to_string()})),
                clap::error::ErrorKind::DisplayVersion => {
                    ("version", json!({"name":"paperhands", "version":VERSION}))
                }
                _ => fail(&error.to_string(), human, 2),
            };
            emit(name, data, human);
            return;
        }
    };
    let (name, data) = execute(&matches, skill).unwrap_or_else(|error| {
        let code = if error
            .downcast_ref::<clap_how_to::Error>()
            .is_some_and(|error| matches!(error, clap_how_to::Error::Encoder(_)))
        {
            1
        } else {
            2
        };
        fail(&error.to_string(), human, code)
    });
    let exit = if matches!(name, "experiment" | "batch") {
        data.get("exit_code").and_then(Value::as_i64).unwrap_or(0) as i32
    } else {
        0
    };
    emit(name, data, human);
    if exit != 0 {
        std::process::exit(exit);
    }
}

fn emit(name: &str, data: Value, human: bool) {
    if human {
        println!("{}", paperhands::human(&data));
    } else {
        let mut result = envelope(name, data);
        if result["data"]["exit_code"].as_i64().is_some_and(|v| v != 0) {
            result["ok"] = json!(false);
        }
        println!("{}", result);
    }
}
