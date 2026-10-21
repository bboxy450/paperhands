//! Project discovery is stable across working subfolders and moving a checkout.
use crate::{Result, research::inventory as inv};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: u32,
    project: Identity,
    storage: Storage,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    id: String,
    name: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Storage {
    path: String,
}
pub struct Project {
    directory: PathBuf,
    manifest: PathBuf,
    root: PathBuf,
    config: Config,
}
impl Project {
    pub fn info(&self) -> Result<Value> {
        Ok(
            json!({"id":self.config.project.id,"name":self.config.project.name,"directory":self.directory,"manifest":self.manifest,"manifest_sha256":inv::digest(&self.manifest)?,"root":self.root,"scope":"project identity; nearest ancestor paperhands.toml"}),
        )
    }
    fn check_binding(&self) -> Result<()> {
        let meta: Value = inv::read_sealed(&self.root.join("project.json"))?;
        if meta["schema_version"] != 1 || meta["project_id"] != self.config.project.id {
            return Err("project/store identity mismatch; refusing to mix project data".into());
        }
        Ok(())
    }
}
pub fn discover(from: &Path) -> Result<Option<Project>> {
    let mut directory = inv::root(from)?;
    loop {
        let manifest = directory.join("paperhands.toml");
        match fs::symlink_metadata(&manifest) {
            Ok(meta) => {
                if meta.len() > 65536 {
                    return Err("paperhands.toml exceeds 64 KiB".into());
                }
                inv::digest(&manifest)?;
                let config: Config =
                    toml::from_str(&String::from_utf8(inv::read_bytes(&manifest)?)?)?;
                inv::name(&config.project.id)?;
                if config.schema_version != 1
                    || config.project.name.trim().is_empty()
                    || config.project.name.len() > 200
                    || config.project.name.chars().any(char::is_control)
                {
                    return Err("invalid project schema/name".into());
                }
                let relative = inv::relative(&config.storage.path)?;
                let mut root = directory.clone();
                for part in relative.components() {
                    root.push(part);
                    match fs::symlink_metadata(&root) {
                        Ok(meta) if meta.file_type().is_symlink() => {
                            return Err("project store path must not traverse symlinks".into());
                        }
                        Ok(_) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                        Err(e) => return Err(e.into()),
                    }
                }
                return Ok(Some(Project {
                    directory,
                    manifest,
                    root,
                    config,
                }));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        if !directory.pop() {
            return Ok(None);
        }
    }
}
pub fn resolve(explicit: Option<&str>) -> Result<PathBuf> {
    resolve_with_missing_store(explicit, false)
}
pub(crate) fn resolve_companion(explicit: Option<&str>) -> Result<PathBuf> {
    // Describe an absent store without treating an existing unbound store as empty.
    resolve_with_missing_store(explicit, true)
}
fn resolve_with_missing_store(explicit: Option<&str>, allow_missing: bool) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(PathBuf::from(path));
    }
    if let Some(root) = std::env::var_os("PAPERHANDS_WORKSPACE") {
        if root.is_empty() {
            return Err("PAPERHANDS_WORKSPACE must not be empty".into());
        }
        return Ok(PathBuf::from(root));
    }
    let project = discover(&std::env::current_dir()?)?
        .ok_or("no paperhands.toml found; use paperhands init or an explicit --root")?;
    match fs::symlink_metadata(&project.root) {
        Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
        Ok(_) => project.check_binding()?,
    }
    Ok(project.root)
}
pub fn inspect(from: &Path) -> Result<Value> {
    let project =
        discover(from)?.ok_or("no paperhands.toml found in this folder or its ancestors")?;
    project.check_binding()?;
    project.info()
}
pub fn initialize(directory: &Path, name: Option<&str>) -> Result<Value> {
    fs::create_dir_all(directory)?;
    let directory = inv::root(directory)?;
    let manifest = directory.join("paperhands.toml");
    let manifest_missing = match fs::symlink_metadata(&manifest) {
        Ok(_) => false,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(e) => return Err(e.into()),
    };
    if manifest_missing {
        let name = name.map(str::to_owned).unwrap_or_else(|| {
            directory
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        });
        if name.trim().is_empty() || name.len() > 200 || name.chars().any(char::is_control) {
            return Err("project name must contain 1..200 UTF-8 bytes without controls".into());
        }
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let id =
            inv::hash(format!("{}:{nonce}:{}", directory.display(), std::process::id()).as_bytes());
        let config = Config {
            schema_version: 1,
            project: Identity { id, name },
            storage: Storage {
                path: ".paperhands".into(),
            },
        };
        inv::exclusive(&manifest, toml::to_string_pretty(&config)?.as_bytes())?;
    }
    let project = discover(&directory)?.ok_or("project manifest creation failed")?;
    if project.directory != directory {
        return Err("init must create or resume a project in the selected folder".into());
    }
    fs::create_dir_all(&project.root)?;
    inv::root(&project.root)?;
    let metadata = project.root.join("project.json");
    match fs::symlink_metadata(&metadata) {
        Ok(_) => project.check_binding()?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            inv::sealed_json(
                &metadata,
                &json!({"schema_version":1,"project_id":project.config.project.id,"created_at_ms":inv::now(),"time_basis":"local project initialization; not market/event time"}),
            )?;
        }
        Err(e) => return Err(e.into()),
    }
    crate::desk::initialize(&project.root)?;
    if !project.root.join(".gitignore").exists() {
        inv::exclusive(&project.root.join(".gitignore"), b"*\n!.gitignore\n")?;
    }
    let mut info = project.info()?;
    info["initialized"] = json!(true);
    info["execution_started"] = json!(false);
    Ok(info)
}

pub fn bootstrap(
    directory: &Path,
    name: Option<&str>,
    research: Option<&str>,
    register: bool,
) -> Result<Value> {
    if research
        .is_some_and(|text| text.trim().is_empty() || text.len() > 8192 || text.contains('\0'))
    {
        return Err("research brief must contain 1..8192 UTF-8 bytes without NUL".into());
    }
    let mut info = initialize(directory, name)?;
    let project = PathBuf::from(
        info["directory"]
            .as_str()
            .ok_or("missing project directory")?,
    );
    let root = info["root"].as_str().ok_or("missing store")?.to_owned();
    if let Some(text) = research {
        let research_dir = inv::subdirectory(&project, "research")?;
        let brief = research_dir.join("BRIEF.md");
        if !brief.exists() {
            inv::exclusive(
                &brief,
                format!("# Research direction\n\n{text}\n").as_bytes(),
            )?;
        } else if String::from_utf8(inv::read_bytes(&brief)?)?
            != format!("# Research direction\n\n{text}\n")
        {
            return Err("research/BRIEF.md already contains a different brief; preserve it and record a new direction explicitly".into());
        }
        let program = research_dir.join("program.md");
        if !program.exists() {
            inv::exclusive(&program,b"# Research practice\n\nRead BRIEF.md and the Paperhands skill. Inspect desk, tasks and cited memory.\nGenerate competing explanations before ranking them. Choose the cheapest test\nthat distinguishes them. Pin data, evaluator and accounting; retain failures,\nunknown inventory, sources and revisit conditions. Record decisions explicitly.\n\nUse paperhands docs read automation for bounded wake setup and receipts.\nA wake does not expand authority, spending limits or control of other collectors.\n")?;
        }
        let receipt = PathBuf::from(&root).join("research-bootstrap.json");
        if !receipt.exists() {
            let run = |argv: Vec<String>| -> Result<Value> {
                let out = crate::runtime::invoke(&argv, &json!({}));
                if out.exit_code != 0 {
                    return Err("research bootstrap operation failed; retained partial records remain inspectable".into());
                }
                Ok(out.value["data"].clone())
            };
            let note_receipt = PathBuf::from(&root).join("research-bootstrap-direction.json");
            let note = if note_receipt.exists() {
                let retained: Value = inv::read_sealed(&note_receipt)?;
                if retained["brief_sha256"] != inv::digest(&brief)? {
                    return Err("partial bootstrap brief changed".into());
                }
                run(vec![
                    "memory".into(),
                    "read".into(),
                    "--root".into(),
                    root.clone(),
                    retained["id"]
                        .as_str()
                        .ok_or("missing bootstrap direction identity")?
                        .into(),
                ])?;
                json!({"record":{"id":retained["id"]}})
            } else {
                let added = run(vec![
                    "memory".into(),
                    "add".into(),
                    "--root".into(),
                    root.clone(),
                    "--kind".into(),
                    "direction".into(),
                    "--title".into(),
                    "Project research direction".into(),
                    "--text".into(),
                    text.into(),
                    "--source".into(),
                    "project:research/BRIEF.md".into(),
                ])?;
                inv::sealed_json(
                    &note_receipt,
                    &json!({"brief_sha256":inv::digest(&brief)?,"id":added["record"]["id"]}),
                )?;
                added
            };
            let mut tasks = Vec::new();
            for (id, title, kind, dependency) in [
                (
                    "frame-question",
                    "Define observations, competing explanations and controls",
                    "plan",
                    None,
                ),
                (
                    "collect-evidence",
                    "Collect or identify closed, causal evidence",
                    "research",
                    Some("frame-question"),
                ),
                (
                    "test-hypothesis",
                    "Run the cheapest discriminating bounded experiment",
                    "experiment",
                    Some("collect-evidence"),
                ),
                (
                    "review-evidence",
                    "Review evidence, unknowns and conditions for another attempt",
                    "review",
                    Some("test-hypothesis"),
                ),
            ] {
                let mut argv = vec![
                    "tasks".into(),
                    "add".into(),
                    "--root".into(),
                    root.clone(),
                    "--id".into(),
                    id.into(),
                    "--title".into(),
                    title.into(),
                    "--kind".into(),
                    kind.into(),
                ];
                if let Some(dependency) = dependency {
                    argv.extend(["--depends-on".into(), dependency.into()]);
                }
                // Existing user tasks retain their identity/decisions on resumed bootstrap.
                let path = PathBuf::from(&root).join("tasks").join(id);
                if !path.exists() {
                    run(argv)?;
                } else {
                    let existing = run(vec![
                        "tasks".into(),
                        "read".into(),
                        "--root".into(),
                        root.clone(),
                        "--id".into(),
                        id.into(),
                    ])?;
                    let first = &existing["events"][0]["event"];
                    let expected = dependency.into_iter().collect::<Vec<_>>();
                    if first["title"] != title
                        || first["kind"] != kind
                        || first["depends_on"] != json!(expected)
                    {
                        return Err("bootstrap task name collides with a different existing task; existing decisions preserved".into());
                    }
                }
                tasks.push(id);
            }
            inv::sealed_json(
                &receipt,
                &json!({"brief_sha256":inv::digest(&brief)?,"direction":note["record"]["id"],"tasks":tasks}),
            )?;
        } else {
            let old: Value = inv::read_sealed(&receipt)?;
            if old["brief_sha256"] != inv::digest(&brief)? {
                return Err("research bootstrap brief changed".into());
            }
        }
        info["research_brief"] = json!(brief);
    }
    info["integration"] = if register {
        crate::setup::install(
            &PathBuf::from(std::env::var_os("HOME").ok_or("HOME unset")?),
            &std::env::current_exe()?,
            Some(&project),
            false,
        )?
    } else {
        json!({"status":"not_requested_or_isolated","next_argv":["paperhands","setup","--project-dir",project]})
    };
    info["automation"] = json!({"scheduler_installed":false,"messages_sent":false,"guide_argv":["paperhands","docs","read","automation"]});
    Ok(info)
}
