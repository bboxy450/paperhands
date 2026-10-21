//! Opt-in local memory partitions. The original memory lock coordinates every
//! cooperating reader/writer; original note bytes and identities never move.
use super::*;
use std::collections::BTreeSet;

pub(super) const MAX_NOTES: usize = 1024;
const MAX_VOLUMES: usize = 16;
const MARKER: &[u8] = b"{\"schema\":\"paperhands.memory-volumes/v1\"}\n";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Member {
    id: String,
    bytes: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rollover {
    schema_version: u32,
    sequence: usize,
    previous: Option<String>,
    closed_volume: String,
    members: Vec<Member>,
    next_volume: String,
    model: pocket_encoder::ModelInfo,
    recorded_at_ms: u64,
}
pub(super) struct Part {
    pub id: String,
    pub dir: PathBuf,
    members: Vec<Member>,
    sealed_by: Option<String>,
    _lock: Option<fs::File>,
}
pub(super) struct Layout {
    pub parts: Vec<Part>,
    pub head: Option<String>,
}
impl Part {
    pub fn relative_directory(&self) -> String {
        if self.id == "primary" {
            "memory".into()
        } else {
            format!("memory-volumes/{}", self.id)
        }
    }
    pub fn note_metadata(&self, id: &str, record: &Record) -> Value {
        let mut result = metadata(id, record);
        result["volume"] = json!(self.id);
        result["retained_relative_path"] =
            json!(format!("{}/{id}.json", self.relative_directory()));
        result
    }
}
fn exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}
fn members(dir: &Path, primary: bool, partitioned: bool) -> Result<Vec<Member>> {
    inv::root(dir)?;
    let mut members = Vec::new();
    for entry in fs::read_dir(dir)? {
        let name = entry?
            .file_name()
            .into_string()
            .map_err(|_| "invalid memory member")?;
        if name == "memory.lock" || (primary && partitioned && name == "volumes-v1.json") {
            continue;
        }
        let id = name
            .strip_suffix(".json")
            .filter(|id| inv::valid_hash(id))
            .ok_or("unexpected memory member")?;
        let path = inv::checked(dir, &name)?;
        let meta = fs::symlink_metadata(path)?;
        if !meta.is_file() {
            return Err("memory member must be a regular file".into());
        }
        members.push(Member {
            id: id.into(),
            bytes: meta.len(),
        });
        if members.len() > MAX_NOTES {
            return Err("memory volume exceeds 1024 notes; refusing incomplete coverage".into());
        }
    }
    members.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(members)
}
impl Layout {
    // Caller holds the original memory.lock for this layout's whole lifetime.
    pub fn load(primary: &Path, write: bool) -> Result<Self> {
        let marker = primary.join("volumes-v1.json");
        let directory = primary
            .parent()
            .ok_or("missing research root")?
            .join("memory-volumes");
        let partitioned = exists(&marker)?;
        if partitioned {
            if inv::read_bytes(&marker)? != MARKER {
                return Err("invalid memory-volume marker".into());
            }
            inv::root(&inv::checked(primary.parent().unwrap(), "memory-volumes")?)?;
        } else if exists(&directory)? {
            return Err(
                "memory-volume storage has no format marker; interrupted or changed catalog".into(),
            );
        }
        let mut layout = Self {
            parts: vec![Part {
                id: "primary".into(),
                dir: primary.into(),
                members: members(primary, true, partitioned)?,
                sealed_by: None,
                _lock: None,
            }],
            head: None,
        };
        if partitioned {
            let names = fs::read_dir(&directory)?
                .map(|e| e.map(|e| e.file_name()))
                .collect::<std::io::Result<BTreeSet<_>>>()?;
            if names.is_empty() || names.len() % 3 != 0 || names.len() / 3 >= MAX_VOLUMES {
                return Err("incomplete or oversized memory-volume catalog".into());
            }
            let count = names.len() / 3;
            let expected = (1..=count)
                .flat_map(|i| {
                    [
                        format!("rollover-{i:06}.json"),
                        format!("rollover-{i:06}.sha256"),
                        format!("volume-{i:06}"),
                    ]
                })
                .map(std::ffi::OsString::from)
                .collect::<BTreeSet<_>>();
            if names != expected {
                return Err("memory-volume catalog member set/sequence changed".into());
            }
            for sequence in 1..=count {
                let event_path = inv::checked(&directory, &format!("rollover-{sequence:06}.json"))?;
                let event: Rollover = inv::read_sealed(&event_path)?;
                let active = layout.parts.last_mut().unwrap();
                let next = format!("volume-{sequence:06}");
                if event.schema_version != 1
                    || event.sequence != sequence
                    || event.previous != layout.head
                    || event.closed_volume != active.id
                    || event.next_volume != next
                    || event.members != active.members
                    || event.model != pocket_encoder::model_info()
                {
                    return Err("memory-volume seal/parent/model/member binding changed".into());
                }
                let sha = inv::digest(&event_path)?;
                active.sealed_by = Some(sha.clone());
                let path = inv::checked(&directory, &next)?;
                inv::root(&path)?;
                let lock_path = inv::checked(&path, "memory.lock")?;
                inv::digest(&lock_path)?;
                let mut options = fs::OpenOptions::new();
                options.read(true).write(write && sequence == count);
                let lock = options.open(lock_path)?;
                if write && sequence == count {
                    fs2::FileExt::lock_exclusive(&lock)?;
                } else {
                    fs2::FileExt::lock_shared(&lock)?;
                }
                layout.parts.push(Part {
                    id: next,
                    members: members(&path, false, true)?,
                    dir: path,
                    sealed_by: None,
                    _lock: Some(lock),
                });
                layout.head = Some(sha);
            }
        }
        let mut ids = BTreeSet::new();
        for part in &layout.parts {
            for member in &part.members {
                if !ids.insert(&member.id) {
                    return Err("duplicate note identity across memory volumes".into());
                }
            }
        }
        Ok(layout)
    }
    pub fn count(&self) -> usize {
        self.parts.iter().map(|p| p.members.len()).sum()
    }
    pub fn active_count(&self) -> usize {
        self.parts.last().unwrap().members.len()
    }
    pub fn active_dir(&self) -> &Path {
        &self.parts.last().unwrap().dir
    }
    pub fn note_set(&self) -> Result<String> {
        let bindings = self
            .parts
            .iter()
            .flat_map(|p| p.members.iter().map(|m| (&p.id, &m.id, m.bytes)))
            .collect::<Vec<_>>();
        Ok(inv::hash(&serde_json::to_vec(&bindings)?))
    }
    pub fn part_for(&self, id: &str) -> Result<&Part> {
        if !inv::valid_hash(id) {
            return Err("memory identity must be a lowercase SHA-256".into());
        }
        self.parts
            .iter()
            .find(|p| p.members.iter().any(|m| m.id == id))
            .ok_or_else(|| "note is absent from selected memory catalog".into())
    }
    pub fn locate(&self, id: &str) -> Result<&Path> {
        Ok(self.part_for(id)?.dir.as_path())
    }
    pub fn visit(
        &self,
        mut visitor: impl FnMut(&Part, &str, Record) -> Result<()>,
    ) -> Result<Value> {
        let mut volumes = Vec::new();
        let (mut total_passages, mut total_text) = (0, 0);
        for part in &self.parts {
            let (mut passages, mut text_bytes) = (0, 0);
            for member in &part.members {
                let record = read(&part.dir, &member.id)?;
                passages += record.search.documents.len();
                text_bytes += record
                    .search
                    .documents
                    .iter()
                    .map(indexed_bytes)
                    .sum::<usize>();
                query_budget(total_passages + passages, total_text + text_bytes)?;
                visitor(part, &member.id, record)?;
            }
            total_passages += passages;
            total_text += text_bytes;
            query_budget(total_passages, total_text)?;
            volumes.push(json!({"id":part.id,"relative_directory":part.relative_directory(),"notes":part.members.len(),"closed":part.sealed_by.is_some(),
                "seal_sha256":part.sealed_by,"indexed_passages":passages,"indexed_text_bytes":text_bytes}));
        }
        Ok(
            json!({"schema":"paperhands.memory-volumes/v1","catalog_head":self.head,"note_set_sha256":self.note_set()?,
            "active_volume":self.parts.last().unwrap().id,"active_notes":self.active_count(),
            "notes_total":self.count(),"planned_volumes":self.parts.len(),"verified_volumes":volumes,
            "complete":true,"max_volumes":MAX_VOLUMES,"max_notes_per_volume":MAX_NOTES,
            "indexed_passages":total_passages,"indexed_text_bytes":total_text,
            "max_total_passages":65536,"max_total_text_bytes":33554432,
            "rollover_argv":if self.active_count()>0 && self.parts.len()<MAX_VOLUMES {Some(json!(["paperhands","memory","rollover","--root",self.parts[0].dir.parent(),"--expected-head",self.head.as_deref().unwrap_or("none"),"--expected-note-set",self.note_set()?]))} else {None},
            "scope":"cooperating local memory writers; no cross-collection snapshot or distributed merge"}),
        )
    }
    pub fn rollover(
        &self,
        primary: &Path,
        expected: &str,
        expected_notes: &str,
        dry: bool,
    ) -> Result<Value> {
        if expected != self.head.as_deref().unwrap_or("none") {
            return Err(
                "memory catalog head changed; inspect memory volumes before rollover".into(),
            );
        }
        if expected_notes != self.note_set()? {
            return Err("memory note set changed; inspect memory volumes before rollover".into());
        }
        if self.parts.len() >= MAX_VOLUMES {
            return Err("memory catalog already has 16 volumes".into());
        }
        let coverage = self.visit(|_, _, _| Ok(()))?;
        if self.active_count() == 0 {
            return Err("cannot roll over an empty memory volume".into());
        }
        let sequence = self.parts.len();
        let next = format!("volume-{sequence:06}");
        let active = self.parts.last().unwrap();
        let event = Rollover {
            schema_version: 1,
            sequence,
            previous: self.head.clone(),
            closed_volume: active.id.clone(),
            members: active.members.clone(),
            next_volume: next.clone(),
            model: pocket_encoder::model_info(),
            recorded_at_ms: inv::now(),
        };
        let mut head = None;
        if !dry {
            let directory = primary
                .parent()
                .ok_or("missing research root")?
                .join("memory-volumes");
            if self.head.is_none() {
                // The marker makes old whole-store readers refuse instead of
                // silently reporting just the original partition as complete.
                inv::exclusive(&primary.join("volumes-v1.json"), MARKER)?;
                fs::create_dir(&directory)?;
                fs::File::open(primary.parent().unwrap())?.sync_all()?;
            }
            let target = inv::subdirectory(&directory, &next)?;
            inv::exclusive(&target.join("memory.lock"), b"")?;
            head = Some(inv::sealed_json(
                &directory.join(format!("rollover-{sequence:06}.json")),
                &event,
            )?);
        }
        Ok(
            json!({"dry_run":dry,"previous_head":self.head,"catalog_head":head,
            "closed_volume":active.id,"closed_notes":active.members.len(),"active_volume":next,
            "original_note_bytes_changed":false,"coverage_before":coverage,
            "interruption":"partial rollover refuses reads; no automatic cleanup, overwrite or replay",
            "compatibility":"rollover is opt-in; old whole-store readers refuse the marker; all writers must use this format"}),
        )
    }
    pub fn prepared(&self, part: &Part) -> Result<PreparedIndex> {
        let mut prepared = PreparedIndex {
            model: clap_how_to::model_info(),
            documents: vec![],
            embeddings: vec![],
        };
        for member in &part.members {
            let record = read(&part.dir, &member.id)?;
            for mut doc in record.search.documents {
                if let Source::Documentation { source, .. } = &mut doc.source {
                    *source = member.id.clone();
                }
                prepared.documents.push(doc);
            }
            prepared.embeddings.extend(record.search.embeddings);
        }
        Ok(prepared)
    }
}
pub(super) fn indexed_bytes(doc: &clap_how_to::Document) -> usize {
    doc.title.len() * 2 + doc.text.len() + 2
}
pub(super) fn query_budget(passages: usize, bytes: usize) -> Result<()> {
    if passages > 65536 || bytes > 32 * 1024 * 1024 {
        return Err("combined memory exceeds 65536 passages or 32 MiB indexed text; rollover does not lift this query bound".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn query_boundaries_are_inclusive_and_rollover_does_not_lift_them() {
        assert!(super::query_budget(65536, 33554432).is_ok());
        assert!(super::query_budget(65537, 1).is_err());
        assert!(super::query_budget(1, 33554433).is_err());
    }
}
