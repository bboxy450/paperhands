//! Append-only research notes with retained text and per-record offline embeddings.
use crate::{Result, research::inventory as inv};
use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
use clap_how_to::{Index, PreparedIndex, Source};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
mod volumes;
use volumes::{Layout, indexed_bytes, query_budget};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: u32,
    kind: String,
    title: String,
    text: String,
    source: Option<String>,
    recorded_at_ms: u64,
    search: PreparedIndex,
}

pub fn command() -> Command {
    let root = || {
        Arg::new("root")
            .long("root")
            .help("Local research store; memory lives beside experiments and snapshots")
    };
    let page = |cmd: Command| {
        cmd.arg(
            Arg::new("offset")
                .long("offset")
                .default_value("0")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("limit")
                .long("limit")
                .default_value("20")
                .value_parser(clap::value_parser!(u32).range(1..=100)),
        )
    };
    Command::new("memory")
        .about("Grow and question a local research datapack; notes are append-only source material")
        .subcommand(Command::new("volumes").about("Verify all memory volumes and inspect capacity and the exact catalog head").arg(root()))
        .subcommand(Command::new("rollover").about("Seal the current memory volume and append a fresh one; existing note IDs/bytes stay in place")
            .arg(root()).arg(Arg::new("expected-head").long("expected-head").required(true).help("Exact memory volumes catalog_head, or none for the first rollover"))
            .arg(Arg::new("expected-note-set").long("expected-note-set").required(true).help("Exact note_set_sha256 from memory volumes; refuses concurrent append changes"))
            .arg(Arg::new("dry-run").long("dry-run").action(ArgAction::SetTrue)))
        .subcommand(
            Command::new("add")
                .about("Retain a UTF-8 note and its offline search vectors under a content hash")
                .arg(root())
                .arg(Arg::new("file").long("file"))
                .arg(
                    Arg::new("text")
                        .long("text")
                        .help("Exact inline UTF-8 source text, at most 16 KiB"),
                )
                .group(
                    ArgGroup::new("content")
                        .args(["file", "text"])
                        .required(true),
                )
                .arg(Arg::new("title").long("title").required(true))
                .arg(Arg::new("kind").long("kind").required(true).value_parser([
                    "observation",
                    "source",
                    "hypothesis",
                    "lesson",
                    "question",
                    "direction",
                ]))
                .arg(
                    Arg::new("source")
                        .long("source")
                        .help("Declared citation/reference; never fetched or treated as verified"),
                ),
        )
        .subcommand(page(
            Command::new("list")
                .about("List verified note identities and metadata")
                .arg(root()),
        ))
        .subcommand(
            Command::new("read")
                .about("Verify and read an original research note")
                .arg(root())
                .arg(Arg::new("id").required(true)),
        )
        .subcommand(
            Command::new("search")
                .visible_alias("ask")
                .about("Ask a question; retrieve cited notes offline, without generating an answer")
                .arg(root())
                .arg(Arg::new("query").required(true))
                .arg(
                    Arg::new("limit")
                        .long("limit")
                        .default_value("5")
                        .value_parser(clap::value_parser!(u32).range(1..=20)),
                ),
        )
}

fn store(root: &Path, write: bool) -> Result<(PathBuf, fs::File)> {
    if write {
        fs::create_dir_all(root)?;
    }
    let root = inv::root(root)?;
    let dir = if write {
        inv::subdirectory(&root, "memory")?
    } else {
        inv::checked(&root, "memory")?
    };
    inv::root(&dir)?;
    let lock_path = dir.join("memory.lock");
    match fs::symlink_metadata(&lock_path) {
        Ok(_) => {
            inv::digest(&lock_path)?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    let lock = fs::OpenOptions::new()
        .create(write)
        .read(true)
        .write(write)
        .truncate(false)
        .open(&lock_path)?;
    if write {
        fs2::FileExt::lock_exclusive(&lock)?;
    } else {
        fs2::FileExt::lock_shared(&lock)?;
    }
    Ok((dir, lock))
}

fn documents(title: &str, text: &str) -> Vec<clap_how_to::Document> {
    let docs = clap_how_to::markdown_documents("record", text, None);
    if docs.is_empty() {
        return vec![clap_how_to::Document {
            title: title.to_owned(),
            text: text.to_owned(),
            source: Source::Documentation {
                source: "record".into(),
                section: title.into(),
                line: 1,
                url: None,
            },
        }];
    }
    docs.into_iter()
        .map(|mut doc| {
            doc.title = format!("{title}: {}", doc.title);
            doc
        })
        .collect()
}

fn read(dir: &Path, id: &str) -> Result<Record> {
    if !inv::valid_hash(id) {
        return Err("memory identity must be a lowercase SHA-256".into());
    }
    let bytes = inv::read_bytes(&inv::checked(dir, &format!("{id}.json"))?)?;
    if inv::hash(&bytes) != id {
        return Err("memory record bytes changed".into());
    }
    let record: Record = serde_json::from_slice(&bytes)?;
    if record.schema_version != 1
        || record.search.documents != documents(&record.title, &record.text)
    {
        return Err("memory record/index source binding mismatch".into());
    }
    Index::from_prepared(record.search.clone())?;
    Ok(record)
}

fn metadata(id: &str, r: &Record) -> Value {
    json!({"id":id,"kind":r.kind,"title":r.title,"recorded_at_ms":r.recorded_at_ms,"source":r.source,"text_bytes":r.text.len(),"source_status":"research material; assertions are not independently verified"})
}

pub fn initialize(root: &Path) -> Result<()> {
    store(root, true)?;
    Ok(())
}
pub fn catalog(root: &Path) -> Result<Vec<Value>> {
    Ok(catalog_with_coverage(root)?.0)
}
pub fn catalog_with_coverage(root: &Path) -> Result<(Vec<Value>, Value)> {
    let (dir, _lock) = store(root, false)?;
    let layout = Layout::load(&dir, false)?;
    let mut notes = Vec::new();
    let coverage = layout.visit(|part, id, record| {
        notes.push(part.note_metadata(id, &record));
        Ok(())
    })?;
    notes.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    Ok((notes, coverage))
}
/// Exact immutable objects and mean-passage vectors for project graph navigation.
pub fn graph_objects(root: &Path) -> Result<Vec<(Value, Vec<f32>)>> {
    graph_objects_selected(root, None)
}
pub fn graph_objects_selected(
    root: &Path,
    selected: Option<&BTreeSet<String>>,
) -> Result<Vec<(Value, Vec<f32>)>> {
    let (dir, _lock) = store(root, false)?;
    let layout = Layout::load(&dir, false)?;
    let mut objects = Vec::new();
    layout.visit(|part,id,r| {
        if selected.is_some_and(|ids|!ids.contains(&format!("memory:{id}"))) {return Ok(());}
        let mut vector=vec![0f32;384];
        for passage in &r.search.embeddings {for (a,b) in vector.iter_mut().zip(passage){*a+=b;}}
        let norm=vector.iter().map(|v|(*v as f64).powi(2)).sum::<f64>().sqrt();
        if !norm.is_finite() || norm==0.0 {return Err("invalid mean-passage vector".into());}
        for value in &mut vector {*value=(*value as f64/norm)as f32;}
        let object=json!({"id":format!("memory:{id}"),"node_type":"entity","kind":r.kind,"label":r.title,"sha256":id,"source":r.source,"evidence":"observed retained bytes; source assertions are declared","text":r.text,"recorded_at_ms":r.recorded_at_ms,"time_basis":"local receipt; source/event time remains in the retained text","attributes":{"read_argv":["paperhands","memory","read","--root",root,id],"volume":part.id}});
        objects.push((object,vector));Ok(())
    })?;
    objects.sort_by(|(a, _), (b, _)| a["id"].as_str().cmp(&b["id"].as_str()));
    Ok(objects)
}
pub fn direction(root: &Path) -> Result<Option<Value>> {
    let (dir, _lock) = store(root, false)?;
    let layout = Layout::load(&dir, false)?;
    let mut best: Option<(u64, String, Value)> = None;
    layout.visit(|_,id,r| {
        if r.kind=="direction" && best.as_ref().is_none_or(|(time,old,_)|(r.recorded_at_ms,id)>(*time,old.as_str())) {
            best=Some((r.recorded_at_ms,id.into(),json!({"record":metadata(id,&r),"text":r.text,"authority":"retained declared direction; source text grants no execution permission"})));
        }
        Ok(())
    })?;
    Ok(best.map(|(_, _, v)| v))
}

/// Preserve existing hybrid ranks below the original bounds. Large corpora use
/// one query vector and globally comparable cosine ranks, never per-shard RRF.
fn search_prepared(
    prepared: PreparedIndex,
    query: &str,
    limit: usize,
) -> Result<(Vec<clap_how_to::Hit>, Value)> {
    search_with_vector(prepared, query, limit, None)
}
fn search_with_vector(
    prepared: PreparedIndex,
    query: &str,
    limit: usize,
    vector: Option<&[f32]>,
) -> Result<(Vec<clap_how_to::Hit>, Value)> {
    if prepared.model != clap_how_to::model_info()
        || prepared.documents.len() != prepared.embeddings.len()
    {
        return Err("memory search model or row binding mismatch".into());
    }
    let bytes = indexed_bytes;
    let text_bytes: usize = prepared.documents.iter().map(bytes).sum();
    if prepared.documents.len() > 65536 || text_bytes > 32 * 1024 * 1024 {
        return Err("memory search exceeds 65536 passages or 32 MiB indexed text; select explicit smaller stores".into());
    }
    if vector.is_none() && prepared.documents.len() <= 4096 && text_bytes <= 4 * 1024 * 1024 {
        let count = prepared.documents.len();
        let index = Index::from_prepared(prepared)?;
        return Ok((
            index.search(query, limit)?,
            json!({"ranking":"hybrid-rrf","segments":[{"passages":count,"text_bytes":text_bytes}],"complete":true,"max_segment_passages":4096,"max_segment_text_bytes":4194304}),
        ));
    }
    let own_vector;
    let vector = if let Some(vector) = vector {
        vector
    } else {
        own_vector = pocket_encoder::Encoder::new()?.embed(&[query])?.remove(0);
        &own_vector
    };
    let mut segments = Vec::new();
    let mut candidates = Vec::new();
    let mut start = 0;
    while start < prepared.documents.len() {
        let mut end = start;
        let mut size = 0;
        while end < prepared.documents.len() && end - start < 4096 {
            let next = bytes(&prepared.documents[end]);
            if size + next > 4 * 1024 * 1024 {
                break;
            }
            size += next;
            end += 1;
        }
        if end == start {
            return Err("individual memory passage exceeds search segment bounds".into());
        }
        // Validate exact model identity and every retained row; no truncation.
        let index = pocket_encoder::Index::from_embeddings(
            prepared.model.clone(),
            prepared.embeddings[start..end].to_vec(),
        )?;
        for hit in index.search_embedding(&prepared.model, vector, limit)? {
            candidates.push((start + hit.index, hit.score));
        }
        segments.push(json!({"passages":end-start,"text_bytes":size}));
        start = end;
    }
    candidates.sort_by(|(a, x), (b, y)| y.total_cmp(x).then(a.cmp(b)));
    let hits = candidates
        .into_iter()
        .take(limit)
        .map(|(i, score)| clap_how_to::Hit {
            document: prepared.documents[i].clone(),
            score: score as f64,
            semantic_score: Some(score),
        })
        .collect();
    Ok((
        hits,
        json!({"ranking":"global-semantic-cosine","segments":segments,"complete":true,"max_segment_passages":4096,"max_segment_text_bytes":4194304,"max_total_passages":65536,"max_total_text_bytes":33554432,"ties":"original stable note-hash/passage order; no cross-segment RRF comparison"}),
    ))
}

pub fn execute(args: &ArgMatches) -> Result<Value> {
    let (action, args) = args
        .subcommand()
        .ok_or("choose a memory operation; read paperhands help memory")?;
    let get = |name: &str| args.get_one::<String>(name).expect("required argument");
    let root = crate::project::resolve(args.get_one::<String>("root").map(String::as_str))?;
    let dry = action == "rollover" && args.get_flag("dry-run");
    let write = action == "add" || (action == "rollover" && !dry);
    let (dir, _lock) = store(&root, write)?;
    let layout = Layout::load(&dir, write)?;
    let result: Result<Value> = match action {
        "volumes" => Ok(layout.visit(|_, _, _| Ok(()))?),
        "rollover" => layout.rollover(&dir, get("expected-head"), get("expected-note-set"), dry),
        "add" => {
            if layout.active_count() >= volumes::MAX_NOTES {
                return Err("active memory volume has 1024 notes; inspect memory volumes then use memory rollover --expected-head before appending".into());
            }
            let title = get("title");
            if title.trim().is_empty() || title.len() > 200 || title.chars().any(char::is_control) {
                return Err("note title must contain 1..200 UTF-8 bytes without controls".into());
            }
            let text = if let Some(text) = args.get_one::<String>("text") {
                text.clone()
            } else {
                let path = Path::new(get("file"));
                if fs::symlink_metadata(path)?.len() > 16 * 1024 {
                    return Err("note source exceeds 16 KiB".into());
                }
                String::from_utf8(inv::read_bytes(path)?)?
            };
            if text.trim().is_empty() || text.len() > 16 * 1024 {
                return Err("note source must contain 1..16,384 UTF-8 bytes".into());
            }
            let source = args.get_one::<String>("source").cloned();
            if source
                .as_ref()
                .is_some_and(|s| s.len() > 2048 || s.chars().any(char::is_control))
            {
                return Err("source reference exceeds 2048 bytes or contains controls".into());
            }
            let index = Index::new(documents(title, &text))?;
            let record = Record {
                schema_version: 1,
                kind: get("kind").clone(),
                title: title.clone(),
                text,
                source,
                recorded_at_ms: inv::now(),
                search: index.prepared().clone(),
            };
            let bytes = serde_json::to_vec_pretty(&record)?;
            let id = inv::hash(&bytes);
            let coverage = layout.visit(|_, _, _| Ok(()))?;
            query_budget(
                coverage["indexed_passages"].as_u64().unwrap() as usize
                    + record.search.documents.len(),
                coverage["indexed_text_bytes"].as_u64().unwrap() as usize
                    + record
                        .search
                        .documents
                        .iter()
                        .map(indexed_bytes)
                        .sum::<usize>(),
            )?;
            if layout.locate(&id).is_ok() {
                return Err("note identity already exists in memory catalog".into());
            }
            inv::exclusive(&layout.active_dir().join(format!("{id}.json")), &bytes)?;
            Ok(
                json!({"record":layout.parts.last().unwrap().note_metadata(&id,&record),"volume":layout.parts.last().unwrap().id,"catalog_head":layout.head,"active_notes":layout.active_count()+1,"append_only":true,"offline":true,"time_basis":"local recording time; source/event time remains declared in the original text"}),
            )
        }
        "read" => {
            let id = get("id");
            let record = read(layout.locate(id)?, id)?;
            Ok(
                json!({"record":layout.part_for(id)?.note_metadata(id,&record),"text":record.text,"catalog_head":layout.head,"integrity_status":"verified","scope":"selected note bytes/index and catalog structure/member bindings; use memory volumes for all-note verification"}),
            )
        }
        "list" => {
            let mut records = Vec::new();
            let coverage = layout.visit(|part, id, r| {
                records.push(part.note_metadata(id, &r));
                Ok(())
            })?;
            records.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
            let offset = *args.get_one::<usize>("offset").unwrap();
            let limit = *args.get_one::<u32>("limit").unwrap() as usize;
            Ok(
                json!({"items":records.iter().skip(offset).take(limit).collect::<Vec<_>>(),"total":records.len(),"offset":offset,"limit":limit,"memory_coverage":coverage,"integrity_status":"verified"}),
            )
        }
        "search" => {
            let query = get("query");
            clap_how_to::validate_query(query)?;
            let memory_coverage = layout.visit(|_, _, _| Ok(()))?;
            let limit = *args.get_one::<u32>("limit").unwrap() as usize;
            let vector = if layout.parts.len() > 1 {
                Some(pocket_encoder::Encoder::new()?.embed(&[query])?.remove(0))
            } else {
                None
            };
            let mut candidates = Vec::new();
            let mut partitions = Vec::new();
            for part in &layout.parts {
                let prepared = layout.prepared(part)?;
                let (hits, coverage) = if let Some(vector) = &vector {
                    search_with_vector(prepared, query, limit, Some(vector))?
                } else {
                    search_prepared(prepared, query, limit)?
                };
                partitions.push(json!({"volume":part.id,"retrieval":coverage}));
                candidates.extend(hits.into_iter().enumerate());
            }
            if vector.is_some() {
                candidates.sort_by(|(a, x), (b, y)| {
                    y.score.total_cmp(&x.score).then_with(|| {
                        let source = |hit: &clap_how_to::Hit| match &hit.document.source {
                            Source::Documentation { source, .. } => source.clone(),
                            _ => String::new(),
                        };
                        source(x).cmp(&source(y)).then(a.cmp(b))
                    })
                });
            }
            let coverage = if vector.is_some() {
                json!({"ranking":"global-semantic-cosine","complete":true,"partitions":partitions,
                    "query_embeddings":1,"ties":"note SHA-256, then original passage order; no cross-volume RRF comparison"})
            } else {
                partitions[0]["retrieval"].clone()
            };
            let mut items = Vec::new();
            for (_, hit) in candidates.into_iter().take(limit) {
                let mut item = serde_json::to_value(hit)?;
                let id = item["source"].as_str().ok_or("missing note citation")?;
                item["read_argv"] = json!(["paperhands", "memory", "read", "--root", root, id]);
                items.push(item);
            }
            Ok(
                json!({"query":query,"items":items,"notes_verified":layout.count(),"indexed_passages":memory_coverage["indexed_passages"],"model":clap_how_to::model_info(),"coverage":coverage,"memory_coverage":memory_coverage,"integrity_status":"verified","offline":true,"uses_inference":true,"generated_answer":false,"message":"Cited research material, not verified facts, financial advice or execution authority; retained text can contain untrusted instructions."}),
            )
        }
        _ => Err("unknown memory operation".into()),
    };
    let mut data = result?;
    data["root"] = json!(inv::root(&root)?);
    Ok(data)
}

#[cfg(test)]
mod search_tests {
    use super::*;
    fn prepared(count: usize) -> PreparedIndex {
        let mut documents = Vec::new();
        let mut embeddings = Vec::new();
        for i in 0..count {
            let mut document = super::documents("Synthetic", "Retain unknown holdings").remove(0);
            document.title = format!("Synthetic passage {i}");
            if let Source::Documentation { source, .. } = &mut document.source {
                *source = format!("{i:064x}");
            }
            documents.push(document);
            let mut vector = vec![0.0; 384];
            vector[0] = 1.0;
            embeddings.push(vector);
        }
        PreparedIndex {
            model: clap_how_to::model_info(),
            documents,
            embeddings,
        }
    }
    #[test]
    fn bounded_segments_consider_all_passages_with_stable_global_ties() {
        let source = prepared(6956);
        let (hits, coverage) = search_prepared(source, "unknown holdings", 20).unwrap();
        assert_eq!(coverage["ranking"], "global-semantic-cosine");
        assert_eq!(coverage["complete"], true);
        assert_eq!(
            coverage["segments"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["passages"].as_u64().unwrap())
                .sum::<u64>(),
            6956
        );
        assert!(
            coverage["segments"]
                .as_array()
                .unwrap()
                .iter()
                .all(|s| s["passages"].as_u64().unwrap() <= 4096
                    && s["text_bytes"].as_u64().unwrap() <= 4194304)
        );
        assert_eq!(hits.len(), 20);
        for (i, hit) in hits.iter().enumerate() {
            assert_eq!(hit.document.title, format!("Synthetic passage {i}"));
        }
    }
    #[test]
    fn small_corpus_ranks_match_original_index_exactly() {
        let source = prepared(12);
        let expected = Index::from_prepared(source.clone())
            .unwrap()
            .search("unknown holdings", 5)
            .unwrap();
        let (actual, coverage) = search_prepared(source, "unknown holdings", 5).unwrap();
        assert_eq!(coverage["ranking"], "hybrid-rrf");
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }
}
