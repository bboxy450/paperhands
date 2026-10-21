//! Managed discovery and native MCP configuration, separate from research data.
use crate::{Result, research::inventory as inv};
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn command() -> Command {
    Command::new("setup").about("Install native Codex/Claude MCP and shared skill discovery; preserve unrelated settings")
        .arg(Arg::new("dry-run").long("dry-run").action(ArgAction::SetTrue))
        .arg(Arg::new("home").long("home").help("Explicit user home for isolated setup; default HOME"))
        .arg(Arg::new("project-dir").long("project-dir").help("Also register project-bound MCP for this initialized project"))
}

pub(crate) fn optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Ok(meta)
            if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 8 * 1024 * 1024 =>
        {
            Err("configuration must be a regular file up to 8 MiB".into())
        }
        Ok(_) => Ok(Some(inv::read_bytes(path)?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Compare before replacing; retain original private bytes outside the project.
pub(crate) fn replace(
    path: &Path,
    expected: Option<&[u8]>,
    bytes: &[u8],
    backup: Option<&Path>,
) -> Result<()> {
    if optional(path)?.as_deref() != expected {
        return Err("configuration changed during setup; retry from a fresh read".into());
    }
    fs::create_dir_all(path.parent().ok_or("missing configuration parent")?)?;
    if let (Some(old), Some(directory)) = (expected, backup) {
        fs::create_dir_all(directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
        }
        let target = directory.join(format!("{}.backup", inv::hash(old)));
        if optional(&target)?.is_none() {
            private_create(&target, old)?;
        }
    }
    let temporary = path.with_file_name(format!(
        ".paperhands-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    private_create(&temporary, bytes)?;
    let result = (|| -> Result<()> {
        if optional(path)?.as_deref() != expected {
            return Err("configuration changed before replacement".into());
        }
        fs::rename(&temporary, path)?;
        fs::File::open(path.parent().unwrap())?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

pub(crate) fn private_create(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub fn aliases(home: &Path, canonical: &Path) -> Result<Value> {
    let canonical = fs::canonicalize(canonical)?;
    let mut items = Vec::new();
    for harness in [".codex", ".claude"] {
        let parent = home.join(harness).join("skills");
        fs::create_dir_all(&parent)?;
        let alias = parent.join("paperhands");
        let status = match fs::symlink_metadata(&alias) {
            Ok(_) if alias.canonicalize().ok().as_ref() == Some(&canonical) => "shared",
            Ok(_) => "conflict: existing unrelated skill retained",
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                #[cfg(unix)]
                std::os::unix::fs::symlink(&canonical, &alias)?;
                #[cfg(not(unix))]
                return Err("skill directory links require Unix on this release".into());
                "linked"
            }
            Err(e) => return Err(e.into()),
        };
        items.push(json!({"path":alias,"status":status,"source":canonical}));
    }
    Ok(json!(items))
}

fn toml_config(old: &[u8], binary: &Path, args: &[String]) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(old).map_err(|_| "configuration is not UTF-8")?;
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|_| "invalid TOML configuration; existing bytes preserved")?;
    if doc.get("mcp_servers").is_some_and(|s| !s.is_table_like()) {
        return Err("mcp_servers must be a TOML table".into());
    }
    if doc
        .get("mcp_servers")
        .and_then(|s| s.get("paperhands"))
        .is_some_and(|s| !s.is_table_like() || s.get("url").is_some())
    {
        return Err("conflicting Paperhands MCP transport; existing entry preserved".into());
    }
    doc["mcp_servers"]["paperhands"]["command"] =
        toml_edit::value(binary.to_str().ok_or("non-UTF-8 executable")?);
    let mut array = toml_edit::Array::new();
    for arg in args {
        array.push(arg.as_str());
    }
    doc["mcp_servers"]["paperhands"]["args"] = toml_edit::value(array);
    Ok(doc.to_string().into_bytes())
}
fn json_config(old: &[u8], binary: &Path, args: &[String]) -> Result<Vec<u8>> {
    let mut value: Value = if old.is_empty() {
        json!({})
    } else {
        serde_json::from_slice(old)
            .map_err(|_| "invalid JSON configuration; existing bytes preserved")?
    };
    let object = value
        .as_object_mut()
        .ok_or("configuration must be an object")?;
    let servers = object
        .entry("mcpServers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("mcpServers must be an object")?;
    let server = servers
        .entry("paperhands")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("Paperhands MCP entry must be an object")?;
    if server.contains_key("url") {
        return Err("conflicting Paperhands MCP transport; existing entry preserved".into());
    }
    server.insert("type".into(), json!("stdio"));
    server.insert("command".into(), json!(binary));
    server.insert("args".into(), json!(args));
    let mut bytes = serde_json::to_vec_pretty(&value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn install(home: &Path, binary: &Path, project: Option<&Path>, dry_run: bool) -> Result<Value> {
    if !home.is_absolute() || !binary.is_absolute() {
        return Err("setup requires absolute home and executable paths".into());
    }
    let backup = home.join(".config/paperhands/setup-backups");
    let mut targets = vec![
        (
            home.join(".codex/config.toml"),
            false,
            vec!["mcp".to_owned()],
        ),
        (home.join(".claude.json"), true, vec!["mcp".to_owned()]),
    ];
    if let Some(project) = project {
        let info = crate::project::inspect(project)?;
        let path = PathBuf::from(
            info["directory"]
                .as_str()
                .ok_or("invalid project directory")?,
        );
        if fs::symlink_metadata(path.join(".codex")).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("project .codex directory must not be a symlink".into());
        }
        let args = vec![
            "mcp".to_owned(),
            "--project-dir".into(),
            path.to_string_lossy().into_owned(),
        ];
        targets.extend([
            (path.join(".codex/config.toml"), false, args.clone()),
            (path.join(".mcp.json"), true, args),
        ]);
    }
    // Prepare all edits before changing any configuration. Never print private contents.
    let mut edits = Vec::new();
    for (path, is_json, args) in targets {
        let old = optional(&path)?;
        let new = if is_json {
            json_config(old.as_deref().unwrap_or_default(), binary, &args)?
        } else {
            toml_config(old.as_deref().unwrap_or_default(), binary, &args)?
        };
        edits.push((path, old, new));
    }
    let skill_directory = home.join(".agents/skills/paperhands");
    let mut discovery = json!({"status":"planned"});
    let mut items = Vec::new();
    if !dry_run {
        fs::create_dir_all(&backup)?;
    }
    let lock_path = home.join(".config/paperhands/setup.lock");
    let _lock = if !dry_run {
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)?;
        fs2::FileExt::lock_exclusive(&file)?;
        Some(file)
    } else {
        None
    };
    if !dry_run {
        fs::create_dir_all(&skill_directory)?;
        let skill_path = skill_directory.join("SKILL.md");
        let old = optional(&skill_path)?;
        if old.as_deref() != Some(crate::SKILL.as_bytes()) {
            replace(&skill_path, old.as_deref(), crate::SKILL.as_bytes(), None)?;
        }
        discovery = aliases(home, &skill_directory)?;
    }
    for (path, old, new) in edits {
        let changed = old.as_deref() != Some(new.as_slice());
        if changed && !dry_run {
            replace(&path, old.as_deref(), &new, Some(&backup))?;
        }
        items.push(
            json!({"path":path,"changed":changed,"status":if dry_run{"planned"}else{"configured"}}),
        );
    }
    Ok(
        json!({"dry_run":dry_run,"configurations":items,"skill_discovery":discovery,"backup_directory":backup,"scope":"Paperhands server entries only; approval, model and inbound messaging policies preserved","client_connection":"configuration written is not live tool discovery; reconnect or restart the client to load it"}),
    )
}

pub fn execute(args: &ArgMatches) -> Result<Value> {
    let home = PathBuf::from(
        args.get_one::<String>("home")
            .cloned()
            .or_else(|| std::env::var("HOME").ok())
            .ok_or("HOME is unset")?,
    );
    install(
        &home,
        &std::env::current_exe()?,
        args.get_one::<String>("project-dir").map(Path::new),
        args.get_flag("dry-run"),
    )
}
