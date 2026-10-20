use crate::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    pub schema_version: u32,
    pub kind: String,
    pub sealed_at_ms: u64,
    pub closed_by: Option<String>,
    pub time_basis: String,
    pub directories: Vec<String>,
    pub files: Vec<Member>,
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn hash(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub fn name(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 80
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err("identity/name must contain 1..80 ASCII letters, digits, - or _".into());
    }
    Ok(())
}
pub fn relative(value: &str) -> Result<PathBuf> {
    let p = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || value.chars().any(char::is_control)
        || p.components().any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("unsafe relative member path".into());
    }
    Ok(p.to_path_buf())
}
pub fn root(path: &Path) -> Result<PathBuf> {
    if fs::symlink_metadata(path)?.file_type().is_symlink() || !path.is_dir() {
        return Err("source must be a real directory, not a symlink".into());
    }
    Ok(fs::canonicalize(path)?)
}
pub fn subdirectory(parent: &Path, name: &str) -> Result<PathBuf> {
    self::name(name)?;
    let path = parent.join(name);
    match fs::symlink_metadata(&path) {
        Ok(m) if !m.is_dir() || m.file_type().is_symlink() => {
            return Err("store directory must not be a link or file".into());
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => fs::create_dir(&path)?,
        Err(e) => return Err(e.into()),
    }
    Ok(path)
}
pub fn checked(root: &Path, value: &str) -> Result<PathBuf> {
    let relative = relative(value)?;
    let mut path = root.to_path_buf();
    for part in relative.components() {
        path.push(part);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err("symlink member refused".into());
        }
    }
    Ok(path)
}
pub fn digest(path: &Path) -> Result<String> {
    let before = fs::symlink_metadata(path)?;
    if !before.is_file() || before.file_type().is_symlink() {
        return Err("hash target must be a regular file, not a link".into());
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let after = fs::symlink_metadata(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.ino() != after.ino() || before.dev() != after.dev() {
            return Err("file identity changed while hashing".into());
        }
    }
    if !after.is_file() || before.len() != after.len() || before.modified()? != after.modified()? {
        return Err("file changed while hashing".into());
    }
    Ok(hex(&hasher.finalize()))
}
pub fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 64 * 1024 * 1024 {
        return Err("JSON must be a regular file of at most 64 MiB".into());
    }
    Ok(fs::read(path)?)
}
pub fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_slice(&read_bytes(path)?)?)
}
pub fn exclusive(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    #[cfg(unix)]
    fs::File::open(
        path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?
    .sync_all()?;
    Ok(())
}
pub fn json(path: &Path, value: &impl Serialize) -> Result<()> {
    exclusive(path, &serde_json::to_vec_pretty(value)?)
}
pub fn sealed_json(path: &Path, value: &impl Serialize) -> Result<String> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let sha = hash(&bytes);
    exclusive(path, &bytes)?;
    exclusive(&path.with_extension("sha256"), sha.as_bytes())?;
    Ok(sha)
}
pub fn read_sealed<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let sha_path = path.with_extension("sha256");
    digest(&sha_path)?;
    let sha = fs::read_to_string(sha_path)?;
    if !valid_hash(&sha) || digest(path)? != sha {
        return Err("sealed record integrity mismatch".into());
    }
    read(path)
}

fn paths(root: &Path) -> Result<(Vec<String>, Vec<String>)> {
    fn visit(
        root: &Path,
        dir: &Path,
        depth: usize,
        dirs: &mut Vec<String>,
        files: &mut Vec<String>,
    ) -> Result<()> {
        if depth > 64 || dirs.len() + files.len() > 100_000 {
            return Err("inventory exceeds depth/member bound".into());
        }
        let mut entries = fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let p = e.path();
            let rel = p
                .strip_prefix(root)?
                .to_str()
                .ok_or("non-UTF8 member path")?
                .to_owned();
            relative(&rel)?;
            let t = fs::symlink_metadata(&p)?.file_type();
            if t.is_symlink() {
                return Err("symlink in inventory refused".into());
            }
            if t.is_dir() {
                dirs.push(rel);
                visit(root, &p, depth + 1, dirs, files)?;
            } else if t.is_file() {
                files.push(rel);
            } else {
                return Err("special filesystem member refused".into());
            }
        }
        Ok(())
    }
    let (mut dirs, mut files) = (Vec::new(), Vec::new());
    visit(root, root, 0, &mut dirs, &mut files)?;
    dirs.sort();
    files.sort();
    Ok((dirs, files))
}
pub fn scan(source: &Path, kind: &str, closed_by: Option<String>) -> Result<Inventory> {
    let source = root(source)?;
    let (directories, paths_before) = paths(&source)?;
    let mut files = Vec::new();
    for path in &paths_before {
        let file = checked(&source, path)?;
        let sha256 = digest(&file)?;
        files.push(Member {
            path: path.clone(),
            bytes: fs::metadata(file)?.len(),
            sha256,
        });
    }
    if (directories.clone(), paths_before) != paths(&source)? {
        return Err("inventory member set changed while hashing".into());
    }
    if let Some(ref closure) = closed_by {
        relative(closure)?;
        if !files.iter().any(|f| &f.path == closure) {
            return Err("closing evidence is not a retained member".into());
        }
    }
    Ok(Inventory {
        schema_version: 1,
        kind: kind.into(),
        sealed_at_ms: now(),
        closed_by,
        time_basis:
            "seal time is local receipt, not venue/event time; original bytes remain authoritative"
                .into(),
        directories,
        files,
    })
}
pub fn verify(source: &Path, expected: &Inventory) -> Result<()> {
    if expected.schema_version != 1 {
        return Err("unsupported inventory schema".into());
    }
    for m in &expected.files {
        relative(&m.path)?;
        if !valid_hash(&m.sha256) {
            return Err("invalid member digest".into());
        }
    }
    let actual = scan(source, &expected.kind, expected.closed_by.clone())?;
    if actual.files != expected.files || actual.directories != expected.directories {
        return Err("inventory bytes or member set changed".into());
    }
    Ok(())
}
pub fn load(path: &Path, expected: &str) -> Result<Inventory> {
    if !valid_hash(expected) || digest(path)? != expected {
        return Err("inventory file hash mismatch".into());
    }
    read(path)
}
pub fn seal(
    source: &Path,
    out: &Path,
    kind: &str,
    closed_by: Option<String>,
) -> Result<serde_json::Value> {
    let source = root(source)?;
    let parent = fs::canonicalize(
        out.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
    if parent.starts_with(&source) {
        return Err("inventory output must be outside its source tree".into());
    }
    if kind == "capture" && closed_by.is_none() {
        return Err("capture requires explicit closing evidence; its meaning remains the evaluator's responsibility".into());
    }
    let inv = scan(&source, kind, closed_by)?;
    let bytes = serde_json::to_vec_pretty(&inv)?;
    exclusive(out, &bytes)?;
    Ok(
        serde_json::json!({"sha256":hash(&bytes),"files":inv.files.len(),"bytes":inv.files.iter().map(|m|m.bytes).sum::<u64>(),"closure":"caller-declared evidence; not independently validated","inventory":out}),
    )
}
pub fn copy(source: &Path, inventory: &Inventory, out: &Path) -> Result<()> {
    verify(source, inventory)?;
    let source = root(source)?;
    let parent = fs::canonicalize(
        out.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
    if parent.starts_with(&source) {
        return Err("copy destination must be outside source".into());
    }
    fs::create_dir(out)?;
    for dir in &inventory.directories {
        fs::create_dir_all(out.join(relative(dir)?))?;
    }
    for file in &inventory.files {
        let target = out.join(relative(&file.path)?);
        let src = checked(&source, &file.path)?;
        let mut input = fs::File::open(src)?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)?;
        std::io::copy(&mut input, &mut output)?;
        output.sync_all()?;
    }
    verify(source.as_path(), inventory)?;
    verify(out, inventory)?;
    Ok(())
}
