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
    project.check_binding()?;
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
