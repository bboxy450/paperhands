use super::inventory as inv;
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub name: String,
    pub role: String,
    pub source: PathBuf,
    pub inventory: PathBuf,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub path: PathBuf,
    pub sha256: String,
    pub role: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub argv: Vec<String>,
    pub cwd: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub wall_time_ms: u64,
    pub cpu_time_seconds: u64,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub artifact_bytes: u64,
    pub artifact_files: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema_version: u32,
    pub id: String,
    pub question: String,
    pub data_use: String,
    pub bundles: Vec<Bundle>,
    pub external_files: Vec<Pin>,
    pub commands: Vec<Step>,
    pub env: BTreeMap<String, String>,
    pub limits: Limits,
    pub required_artifacts: Vec<String>,
    pub cases: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frozen {
    pub plan_sha256: String,
    pub snapshots: BTreeMap<String, String>,
    pub executable_targets: BTreeMap<String, String>,
    pub created_at_ms: u64,
}

pub fn executable_digest(path: &Path) -> Result<String> {
    inv::digest(&fs::canonicalize(path)?)
}
pub fn validate(plan: &Plan) -> Result<()> {
    inv::name(&plan.id)?;
    if plan.schema_version != 1
        || plan.question.trim().is_empty()
        || plan.question.len() > 4096
        || !["development", "validation", "prospective-paper", "unknown"]
            .contains(&plan.data_use.as_str())
    {
        return Err("invalid plan schema, question or data_use".into());
    }
    let l = &plan.limits;
    if !(1..=86_400_000).contains(&l.wall_time_ms)
        || !(1..=86_400).contains(&l.cpu_time_seconds)
        || l.stdout_bytes == 0
        || l.stderr_bytes == 0
        || l.stdout_bytes > 64 * 1024 * 1024
        || l.stderr_bytes > 64 * 1024 * 1024
        || l.artifact_bytes == 0
        || l.artifact_bytes > 64 * 1024 * 1024 * 1024
        || !(1..=100_000).contains(&l.artifact_files)
    {
        return Err("invalid explicit resource limits".into());
    }
    if plan.bundles.is_empty()
        || plan.bundles.len() > 64
        || plan.commands.is_empty()
        || plan.commands.len() > 64
    {
        return Err("plan needs 1..64 bundles and commands".into());
    }
    let mut names = BTreeSet::new();
    let mut roles = BTreeSet::new();
    for b in &plan.bundles {
        inv::name(&b.name)?;
        if !names.insert(&b.name)
            || !["input", "evaluator", "accounting", "config", "candidate"]
                .contains(&b.role.as_str())
        {
            return Err("duplicate bundle or unsupported role".into());
        }
        roles.insert(b.role.as_str());
        if !b.source.is_absolute() || !b.inventory.is_absolute() {
            return Err("bundle sources/inventories must be absolute".into());
        }
        let inventory = inv::load(&b.inventory, &b.sha256)?;
        if b.role == "input" && inventory.closed_by.is_none() {
            return Err("input lacks declared closing evidence".into());
        }
        inv::verify(&b.source, &inventory)?;
    }
    if !roles.contains("input") || !roles.contains("evaluator") || !roles.contains("config") {
        return Err("input, evaluator and config bundles are mandatory".into());
    }
    let mut pins = BTreeSet::new();
    for p in &plan.external_files {
        if !p.path.is_absolute() || !pins.insert(&p.path) || !inv::valid_hash(&p.sha256) {
            return Err("invalid external pin".into());
        }
        let digest = if p.role == "executable" {
            executable_digest(&p.path)?
        } else {
            inv::digest(&p.path)?
        };
        if digest != p.sha256 {
            return Err("external dependency hash changed".into());
        }
    }
    for s in &plan.commands {
        if s.argv.is_empty()
            || s.argv.len() > 256
            || s.argv.iter().any(|v| v.len() > 16384 || v.contains('\0'))
        {
            return Err("invalid argv array".into());
        }
        let program = Path::new(&s.argv[0]);
        if !program.is_absolute()
            || !plan
                .external_files
                .iter()
                .any(|p| p.role == "executable" && p.path == program)
        {
            return Err("argv[0] must be an absolute pinned executable".into());
        }
        if s.cwd != "{work}"
            && !plan
                .bundles
                .iter()
                .any(|b| s.cwd == format!("{{snapshot:{}}}", b.name))
        {
            return Err("cwd must be {work} or an exact snapshot reference".into());
        }
        for argument in &s.argv {
            check_tokens(argument, plan)?;
        }
    }
    if plan.env.len() > 64
        || plan.env.iter().any(|(k, v)| {
            k.is_empty() || k.contains(['=', '\0']) || v.contains('\0') || v.len() > 16384
        })
    {
        return Err("invalid explicit environment".into());
    }
    for value in plan.env.values() {
        check_tokens(value, plan)?;
    }
    for path in &plan.required_artifacts {
        inv::relative(path)?;
    }
    if let Some(cases) = &plan.cases {
        inv::relative(cases)?;
        if !plan.required_artifacts.contains(cases) {
            return Err("cases must be a required artifact".into());
        }
    }
    Ok(())
}
fn check_tokens(text: &str, plan: &Plan) -> Result<()> {
    let mut rest = text;
    while let Some(start) = rest.find("{snapshot:") {
        let end = rest[start..].find('}').ok_or("unclosed argv template")? + start;
        let token = &rest[start..=end];
        if !plan
            .bundles
            .iter()
            .any(|b| token == format!("{{snapshot:{}}}", b.name))
        {
            return Err(
                "unknown argv template; use literal argv values or declared snapshot references"
                    .into(),
            );
        }
        rest = &rest[end + 1..];
    }
    Ok(())
}
pub fn validate_file(path: &Path) -> Result<Value> {
    let bytes = inv::read_bytes(path)?;
    let p: Plan = serde_json::from_slice(&bytes)?;
    validate(&p)?;
    Ok(
        json!({"valid":true,"id":p.id,"executed":false,"experiment_writes":false,"skill_refresh":"normal CLI startup applies","plan_sha256":inv::hash(&bytes)}),
    )
}
pub fn create(path: &Path, store: &Path) -> Result<Value> {
    let bytes = inv::read_bytes(path)?;
    let plan: Plan = serde_json::from_slice(&bytes)?;
    validate(&plan)?;
    fs::create_dir_all(store)?;
    let store = inv::root(store)?;
    inv::subdirectory(&store, "experiments")?;
    inv::subdirectory(&store, "snapshots")?;
    let dir = store.join("experiments").join(&plan.id);
    fs::create_dir(&dir)?;
    // A reservation is spent even when copying fails. Partial evidence is retained.
    inv::exclusive(&dir.join("plan.json"), &bytes)?;
    let mut snapshots = BTreeMap::new();
    for bundle in &plan.bundles {
        let inventory = inv::load(&bundle.inventory, &bundle.sha256)?;
        let destination = store.join("snapshots").join(&bundle.sha256);
        if !destination.exists() {
            let staging = store.join("snapshots").join(format!(
                "_{}-{}-{}",
                bundle.sha256,
                std::process::id(),
                inv::now()
            ));
            fs::create_dir(&staging)?;
            inv::copy(&bundle.source, &inventory, &staging.join("data"))?;
            inv::exclusive(
                &staging.join("inventory.json"),
                &fs::read(&bundle.inventory)?,
            )?;
            if let Err(error) = fs::rename(&staging, &destination)
                && !destination.exists()
            {
                return Err(error.into());
            }
        }
        if inv::digest(&destination.join("inventory.json"))? != bundle.sha256 {
            return Err("snapshot inventory integrity mismatch".into());
        }
        inv::verify(&destination.join("data"), &inventory)?;
        snapshots.insert(bundle.name.clone(), bundle.sha256.clone());
    }
    let mut executable_targets = BTreeMap::new();
    for p in &plan.external_files {
        if p.role == "executable" {
            executable_targets.insert(
                p.path.to_string_lossy().into(),
                fs::canonicalize(&p.path)?.to_string_lossy().into(),
            );
        }
    }
    let frozen = Frozen {
        plan_sha256: inv::digest(&dir.join("plan.json"))?,
        snapshots,
        executable_targets,
        created_at_ms: inv::now(),
    };
    inv::sealed_json(&dir.join("frozen.json"), &frozen)?;
    inv::exclusive(&dir.join("run.lock"), b"")?;
    Ok(
        json!({"id":plan.id,"directory":dir,"process_status":"planned","evidence_status":"unreviewed","plan_sha256":frozen.plan_sha256,"snapshots":frozen.snapshots}),
    )
}
pub fn load(dir: &Path) -> Result<(PathBuf, Plan, Frozen)> {
    let dir = inv::root(dir)?;
    let plan: Plan = inv::read(&dir.join("plan.json"))?;
    let frozen: Frozen = inv::read_sealed(&dir.join("frozen.json"))?;
    inv::name(&plan.id)?;
    if dir.file_name().and_then(|v| v.to_str()) != Some(plan.id.as_str())
        || inv::digest(&dir.join("plan.json"))? != frozen.plan_sha256
    {
        return Err("frozen plan hash/identity mismatch".into());
    }
    let store = dir
        .parent()
        .and_then(Path::parent)
        .ok_or("invalid experiment layout")?
        .to_path_buf();
    for b in &plan.bundles {
        if frozen.snapshots.get(&b.name) != Some(&b.sha256) {
            return Err("snapshot mapping changed".into());
        }
        let snapshot = store.join("snapshots").join(&b.sha256);
        let inventory = inv::load(&snapshot.join("inventory.json"), &b.sha256)?;
        inv::verify(&snapshot.join("data"), &inventory)?;
    }
    for p in &plan.external_files {
        let digest = if p.role == "executable" {
            let target = fs::canonicalize(&p.path)?.to_string_lossy().into_owned();
            if frozen
                .executable_targets
                .get(&p.path.to_string_lossy().into_owned())
                != Some(&target)
            {
                return Err("executable target changed".into());
            }
            executable_digest(&p.path)?
        } else {
            inv::digest(&p.path)?
        };
        if digest != p.sha256 {
            return Err("external dependency changed".into());
        }
    }
    Ok((dir, plan, frozen))
}
pub fn expand(value: &str, dir: &Path, frozen: &Frozen) -> Result<String> {
    let store = dir.parent().and_then(Path::parent).ok_or("invalid store")?;
    let mut result = value
        .replace("{work}", &dir.join("work").to_string_lossy())
        .replace("{output}", &dir.join("output").to_string_lossy());
    for (name, sha) in &frozen.snapshots {
        result = result.replace(
            &format!("{{snapshot:{name}}}"),
            &store
                .join("snapshots")
                .join(sha)
                .join("data")
                .to_string_lossy(),
        );
    }
    Ok(result)
}
