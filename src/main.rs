use paperhands::{command, runtime};

fn main() {
    let args = match std::env::args_os()
        .skip(1)
        .map(|a| a.into_string())
        .collect::<std::result::Result<Vec<_>, _>>()
    {
        Ok(args) => args,
        Err(_) => finish(runtime::failure("arguments must be valid UTF-8", 2), false),
    };
    let human = args
        .iter()
        .take_while(|arg| arg.as_str() != "--")
        .any(|arg| arg == "--human");
    let skill = match paperhands::refresh_skill() {
        Ok(skill) => skill,
        Err(error) => finish(
            runtime::failure(&format!("skill installation: {error}"), 1),
            human,
        ),
    };
    if let Ok(matches) = command()
        .try_get_matches_from(std::iter::once("paperhands").chain(args.iter().map(String::as_str)))
        && matches.subcommand_name() == Some("mcp")
    {
        if human {
            finish(
                runtime::failure("mcp uses JSON-RPC; omit --human", 2),
                human,
            );
        }
        if let Some(directory) = matches
            .subcommand()
            .and_then(|(_, args)| args.get_one::<String>("project-dir"))
        {
            let select = (|| -> paperhands::Result<()> {
                let project = paperhands::project::inspect(std::path::Path::new(directory))?;
                std::env::set_current_dir(
                    project["directory"]
                        .as_str()
                        .ok_or("invalid project directory")?,
                )?;
                Ok(())
            })();
            if let Err(error) = select {
                finish(runtime::failure(&format!("MCP project: {error}"), 2), false);
            }
        }
        if let Err(error) =
            paperhands::mcp::serve(std::io::stdin().lock(), std::io::stdout().lock(), &skill)
        {
            finish(
                runtime::failure(&format!("MCP transport: {error}"), 1),
                false,
            );
        }
        return;
    }
    finish(runtime::invoke(&args, &skill), human)
}

fn finish(outcome: runtime::Outcome, human: bool) -> ! {
    let data = outcome.value.get("data");
    let text = if human {
        paperhands::human(data.unwrap_or(&outcome.value["error"]["message"]))
    } else {
        outcome.value.to_string()
    };
    if data.is_some() {
        println!("{text}");
    } else {
        eprintln!("{text}");
    }
    std::process::exit(outcome.exit_code)
}
