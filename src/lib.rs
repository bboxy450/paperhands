use clap::{Arg, ArgAction, Command};
use clap_how_to::{Document, HowToExt, Index, Mode, PreparedIndex, Source};
use serde_json::{Value, json};
use std::{error::Error, fs, path::PathBuf};

pub mod research;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SKILL: &str = include_str!("../skills/paperhands/SKILL.md");
pub const GUIDES: &[(&str, &str, &str)] = &[
    ("overview", "README.md", include_str!("../README.md")),
    ("cli", "docs/cli.md", include_str!("../docs/cli.md")),
    (
        "experiments",
        "docs/experiments.md",
        include_str!("../docs/experiments.md"),
    ),
    ("skill", "skills/paperhands/SKILL.md", SKILL),
];

pub fn command() -> Command {
    let search = || {
        Command::new("search")
            .about("Find relevant sources by meaning, offline")
            .arg(
                Arg::new("query")
                    .required(true)
                    .help("Quote the whole question"),
            )
    };
    Command::new("paperhands")
        .version(VERSION)
        .propagate_version(true)
        .about("Local strategy research experiments, retained evidence and offline semantic help")
        .disable_help_flag(true)
        .disable_version_flag(true)
        .disable_help_subcommand(true)
        .arg(
            Arg::new("human")
                .long("human")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Render readable text instead of JSON"),
        )
        .arg(
            Arg::new("help")
                .long("help")
                .short('h')
                .global(true)
                .action(ArgAction::Help)
                .help("Show command help"),
        )
        .arg(
            Arg::new("version")
                .long("version")
                .short('V')
                .global(true)
                .action(ArgAction::Version)
                .help("Show binary version"),
        )
        .subcommand(
            Command::new("status")
                .about("Inspect implemented capabilities and planned trading venues"),
        )
        .with_how_to()
        .subcommands(research::cli::commands())
        .subcommand(
            Command::new("cli")
                .about("Discover installed commands by intent")
                .subcommand(search()),
        )
        .subcommand(
            Command::new("docs")
                .about("Search and read bundled documentation offline")
                .subcommand(search())
                .subcommand(Command::new("list").about("List bundled guide IDs and source paths"))
                .subcommand(
                    Command::new("read")
                        .about("Read a bundled guide by ID")
                        .arg(Arg::new("id").required(true)),
                ),
        )
        .subcommand(
            Command::new("skills")
                .about("Inspect the bundled agent skill and its automatic installation"),
        )
        .subcommand(
            Command::new("help")
                .about("Read help for a command path")
                .arg(Arg::new("path").num_args(0..)),
        )
}

pub fn help_documents() -> Vec<Document> {
    let mut documents = clap_how_to::command_documents(&command());
    for (id, _, text) in GUIDES {
        documents.extend(clap_how_to::markdown_documents(id, text, None));
    }
    documents
}

pub fn search(query: &str, scope: &str) -> Result<Value> {
    clap_how_to::validate_query(query)?;
    let mut prepared: PreparedIndex = serde_json::from_str(include_str!("../assets/how-to.json"))?;
    let rows = prepared.documents.into_iter().zip(prepared.embeddings);
    (prepared.documents, prepared.embeddings) = rows
        .filter(|(doc, _)| match scope {
            "commands" => matches!(doc.source, Source::Command { .. }),
            "docs" => matches!(doc.source, Source::Documentation { .. }),
            _ => true,
        })
        .unzip();
    let index = Index::from_prepared(prepared)?;
    Ok(json!({
        "query": query, "scope": scope, "offline": true, "uses_inference": true,
        "retrieval": "neural-cosine+bm25-rrf", "model": index.prepared().model,
        "items": index.search_with_mode(query, 5, Mode::Hybrid)?,
        "message": "Ranked source matches, not generated answers or confidence. Read the sources before acting."
    }))
}

pub fn help(app: &mut Command) -> Value {
    app.build();
    let arguments = app
        .get_arguments()
        .map(|arg| {
            json!({
                "name": arg.get_id().as_str(), "long": arg.get_long(), "short": arg.get_short(),
                "required": arg.is_required_set(), "help": arg.get_help().map(ToString::to_string)
            })
        })
        .collect::<Vec<_>>();
    let subcommands = app
        .get_subcommands()
        .map(|child| {
            json!({
                "name":child.get_name(), "aliases":child.get_visible_aliases().collect::<Vec<_>>(),
                "about":child.get_about().map(ToString::to_string)
            })
        })
        .collect::<Vec<_>>();
    json!({"usage": app.render_usage().to_string(), "help": app.render_long_help().to_string(), "arguments": arguments, "subcommands": subcommands})
}

pub fn refresh_skill() -> Result<Value> {
    let directory = match std::env::var_os("PAPERHANDS_SKILLS_DIR") {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        Some(_) => return Err("PAPERHANDS_SKILLS_DIR must not be empty".into()),
        None => PathBuf::from(
            std::env::var_os("HOME").ok_or("HOME is unset; set PAPERHANDS_SKILLS_DIR")?,
        )
        .join(".agents/skills/paperhands"),
    };
    fs::create_dir_all(&directory)?;
    let destination = directory.join("SKILL.md");
    match fs::symlink_metadata(&destination) {
        Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => {
            return Err("managed SKILL.md must be a regular file".into());
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let changed = match fs::read(&destination) {
        Ok(bytes) => bytes != SKILL.as_bytes(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(error) => return Err(error.into()),
    };
    if changed {
        use std::io::Write;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let temporary = directory.join(format!(".SKILL.md.{}.{nonce}", std::process::id()));
        let result = (|| -> Result<()> {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(SKILL.as_bytes())?;
            file.sync_all()?;
            fs::rename(&temporary, &destination)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result?;
    }
    Ok(json!({"directory":directory, "path":destination, "updated":changed, "version":VERSION}))
}

pub fn envelope(command: &str, data: Value) -> Value {
    json!({"schema":"paperhands/v1", "version":VERSION, "ok":true, "command":command, "data":data})
}

pub fn human(data: &Value) -> String {
    if let Some(text) = data
        .get("help")
        .or_else(|| data.get("text"))
        .and_then(Value::as_str)
    {
        return text.to_owned();
    }
    if let Some(items) = data.get("items").and_then(Value::as_array) {
        return items
            .iter()
            .map(|item| {
                format!(
                    "{}\n{}\n{}\n",
                    item["title"].as_str().unwrap_or(""),
                    item["text"].as_str().unwrap_or(""),
                    item.get("help")
                        .or_else(|| item.get("source"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
    }
    match data {
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| format!("{key}: {}", human(value)))
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Array(items) => items.iter().map(human).collect::<Vec<_>>().join(", "),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn prepared_index_matches_shipped_commands_and_guides() {
        let prepared: clap_how_to::PreparedIndex =
            serde_json::from_str(include_str!("../assets/how-to.json")).unwrap();
        assert_eq!(prepared.documents, super::help_documents());
        clap_how_to::Index::from_prepared(prepared).unwrap();
    }
}
