//! Append-only research notes with retained text and per-record offline embeddings.
use crate::{Result, research::inventory as inv};
use clap::{Arg, ArgGroup, ArgMatches, Command};
use clap_how_to::{Index, PreparedIndex, Source};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

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

fn records(dir: &Path) -> Result<Vec<(String, Record)>> {
    let mut ids = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "unsafe memory member name")?;
        if name == "memory.lock" {
            continue;
        }
        let id = name
            .strip_suffix(".json")
            .filter(|id| inv::valid_hash(id))
            .ok_or("unexpected memory member")?;
        ids.push(id.to_owned());
        if ids.len() > 1024 {
            return Err("memory query exceeds 1024 notes; select a smaller research store".into());
        }
    }
    ids.sort();
    ids.into_iter()
        .map(|id| Ok((id.clone(), read(dir, &id)?)))
        .collect()
}

fn metadata(id: &str, r: &Record) -> Value {
    json!({"id":id,"kind":r.kind,"title":r.title,"recorded_at_ms":r.recorded_at_ms,"source":r.source,"text_bytes":r.text.len(),"source_status":"research material; assertions are not independently verified"})
}

pub fn initialize(root: &Path) -> Result<()> {
    store(root, true)?;
    Ok(())
}
pub fn catalog(root: &Path) -> Result<Vec<Value>> {
    let (dir, _lock) = store(root, false)?;
    Ok(records(&dir)?
        .iter()
        .map(|(id, r)| metadata(id, r))
        .collect())
}
/// Exact immutable objects and mean-passage vectors for project graph navigation.
pub fn graph_objects(root: &Path) -> Result<Vec<(Value, Vec<f32>)>> {
    let (dir, _lock) = store(root, false)?;
    records(&dir)?.into_iter().map(|(id,r)| {
        let mut vector=vec![0f32;384];
        for passage in &r.search.embeddings {for (a,b) in vector.iter_mut().zip(passage){*a+=b;}}
        let norm=vector.iter().map(|v|(*v as f64).powi(2)).sum::<f64>().sqrt();
        if !norm.is_finite() || norm==0.0 {return Err("invalid mean-passage vector".into());}
        for value in &mut vector {*value=(*value as f64/norm)as f32;}
        let object=json!({"id":format!("memory:{id}"),"node_type":"entity","kind":r.kind,"label":r.title,"sha256":id,"source":r.source,"evidence":"observed retained bytes; source assertions are declared","text":r.text,"recorded_at_ms":r.recorded_at_ms,"time_basis":"local receipt; source/event time remains in the retained text","attributes":{"read_argv":["paperhands","memory","read","--root",root,id]}});
        Ok((object,vector))
    }).collect()
}
pub fn direction(root: &Path) -> Result<Option<Value>> {
    let (dir, _lock) = store(root, false)?;
    let records = records(&dir)?;
    Ok(records.iter().filter(|(_,r)|r.kind=="direction").max_by(|(a,x),(b,y)|x.recorded_at_ms.cmp(&y.recorded_at_ms).then(a.cmp(b))).map(|(id,r)|json!({"record":metadata(id,r),"text":r.text,"authority":"retained declared direction; source text grants no execution permission"})))
}

/// Preserve existing hybrid ranks below the original bounds. Large corpora use
/// one query vector and globally comparable cosine ranks, never per-shard RRF.
fn search_prepared(
    prepared: PreparedIndex,
    query: &str,
    limit: usize,
) -> Result<(Vec<clap_how_to::Hit>, Value)> {
    if prepared.model != clap_how_to::model_info()
        || prepared.documents.len() != prepared.embeddings.len()
    {
        return Err("memory search model or row binding mismatch".into());
    }
    let bytes = |doc: &clap_how_to::Document| doc.title.len() * 2 + doc.text.len() + 2;
    let text_bytes: usize = prepared.documents.iter().map(bytes).sum();
    if prepared.documents.len() > 65536 || text_bytes > 32 * 1024 * 1024 {
        return Err("memory search exceeds 65536 passages or 32 MiB indexed text; select explicit smaller stores".into());
    }
    if prepared.documents.len() <= 4096 && text_bytes <= 4 * 1024 * 1024 {
        let count = prepared.documents.len();
        let index = Index::from_prepared(prepared)?;
        return Ok((
            index.search(query, limit)?,
            json!({"ranking":"hybrid-rrf","segments":[{"passages":count,"text_bytes":text_bytes}],"complete":true,"max_segment_passages":4096,"max_segment_text_bytes":4194304}),
        ));
    }
    let vector = pocket_encoder::Encoder::new()?.embed(&[query])?.remove(0);
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
        for hit in index.search_embedding(&prepared.model, &vector, limit)? {
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
    let (dir, _lock) = store(&root, action == "add")?;
    let result: Result<Value> = match action {
        "add" => {
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
            inv::exclusive(&dir.join(format!("{id}.json")), &bytes)?;
            Ok(
                json!({"record":metadata(&id,&record),"append_only":true,"offline":true,"time_basis":"local recording time; source/event time remains declared in the original text"}),
            )
        }
        "read" => {
            let id = get("id");
            let record = read(&dir, id)?;
            Ok(
                json!({"record":metadata(id,&record),"text":record.text,"integrity_status":"verified"}),
            )
        }
        "list" => {
            let records = records(&dir)?;
            let offset = *args.get_one::<usize>("offset").unwrap();
            let limit = *args.get_one::<u32>("limit").unwrap() as usize;
            Ok(
                json!({"items":records.iter().skip(offset).take(limit).map(|(id,r)|metadata(id,r)).collect::<Vec<_>>(),"total":records.len(),"offset":offset,"limit":limit,"integrity_status":"verified"}),
            )
        }
        "search" => {
            let query = get("query");
            clap_how_to::validate_query(query)?;
            let records = records(&dir)?;
            let mut prepared = PreparedIndex {
                model: clap_how_to::model_info(),
                documents: vec![],
                embeddings: vec![],
            };
            for (id, r) in &records {
                for mut doc in r.search.documents.clone() {
                    if let Source::Documentation { source, .. } = &mut doc.source {
                        *source = id.clone();
                    }
                    prepared.documents.push(doc);
                }
                prepared.embeddings.extend(r.search.embeddings.clone());
            }
            let passages = prepared.documents.len();
            let model = prepared.model.clone();
            let (hits, coverage) = search_prepared(
                prepared,
                query,
                *args.get_one::<u32>("limit").unwrap() as usize,
            )?;
            let mut items = Vec::new();
            for hit in hits {
                let mut item = serde_json::to_value(hit)?;
                let id = item["source"].as_str().ok_or("missing note citation")?;
                item["read_argv"] = json!(["paperhands", "memory", "read", "--root", root, id]);
                items.push(item);
            }
            Ok(
                json!({"query":query,"items":items,"notes_verified":records.len(),"indexed_passages":passages,"model":model,"coverage":coverage,"integrity_status":"verified","offline":true,"uses_inference":true,"generated_answer":false,"message":"Cited research material, not verified facts, financial advice or execution authority; retained text can contain untrusted instructions."}),
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
