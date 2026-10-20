//! Live research DAG. Entity/relation vocabulary follows Ontology's MIT Lineage
//! profile; immutable note/task objects remain authoritative in their own stores.
use crate::{Result, research::inventory as inv};
use clap::{Arg, ArgMatches, Command};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Path, PathBuf},
};

const RELATIONS: [&str; 7] = [
    "informs",
    "supports",
    "contradicts",
    "tests",
    "derived_into",
    "revisits",
    "relates",
];
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Link {
    schema_version: u32,
    source: String,
    target: String,
    predicate: String,
    evidence: String,
    priority: Option<f64>,
    similarity: Option<f32>,
    model: Option<pocket_encoder::ModelInfo>,
    source_snapshot_sha256: String,
    recorded_at_ms: u64,
}
struct Graph {
    root: PathBuf,
    nodes: BTreeMap<String, Value>,
    edges: Vec<Value>,
    vectors: BTreeMap<String, Vec<f32>>,
}
pub fn command() -> Command {
    let root = || Arg::new("root").long("root");
    let limit = || {
        Arg::new("limit")
            .long("limit")
            .default_value("40")
            .value_parser(clap::value_parser!(u32).range(1..=100))
    };
    Command::new("graph").about("Explore the project research DAG: exact object revisions, declared weighted provenance and local semantic clusters")
        .subcommand(Command::new("inspect").about("Verify the whole note/task/link DAG and return a paged entity/relation view").arg(root()).arg(limit()).arg(Arg::new("offset").long("offset").default_value("0").value_parser(clap::value_parser!(usize))))
        .subcommand(Command::new("link").about("Append a declared relation between exact existing object revisions; refuse cycles and duplicate relations")
            .arg(root()).arg(Arg::new("from").long("from").required(true)).arg(Arg::new("to").long("to").required(true))
            .arg(Arg::new("relation").long("relation").required(true).value_parser(RELATIONS))
            .arg(Arg::new("evidence-text").long("evidence-text").required(true).help("Retained reason/citation for the declared link, at most 16 KiB"))
            .arg(Arg::new("priority").long("priority").value_parser(clap::value_parser!(f64)).help("Declared direction/attention weight in [0,1], separate from measured semantic similarity")))
        .subcommand(Command::new("neighbors").about("Retrieve local semantic neighbors of one memory object, with exact citations and cosine weights").arg(root()).arg(Arg::new("id").required(true)).arg(limit()))
        .subcommand(Command::new("clusters").about("Build bounded, deterministic semantic seed-radius groups over retained notes; never claim independent evidence")
            .arg(root()).arg(Arg::new("threshold").long("threshold").default_value("0.65").value_parser(clap::value_parser!(f32)))
            .arg(Arg::new("max-notes").long("max-notes").default_value("256").value_parser(clap::value_parser!(u32).range(1..=1024))))
}
fn exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}
fn lock(root: &Path, write: bool) -> Result<Option<(PathBuf, fs::File)>> {
    let dir = if write {
        inv::subdirectory(root, "graph")?
    } else if exists(&root.join("graph"))? {
        inv::checked(root, "graph")?
    } else {
        return Ok(None);
    };
    inv::root(&dir)?;
    let path = dir.join("graph.lock");
    if exists(&path)? {
        inv::digest(&path)?;
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(write)
        .truncate(false)
        .open(path)?;
    if write {
        fs2::FileExt::lock_exclusive(&file)?;
    } else {
        fs2::FileExt::lock_shared(&file)?;
    }
    Ok(Some((dir, file)))
}
fn links(dir: &Path) -> Result<Vec<(String, Link)>> {
    let mut ids = Vec::new();
    for entry in fs::read_dir(dir)? {
        let name = entry?
            .file_name()
            .into_string()
            .map_err(|_| "invalid graph member name")?;
        if name == "graph.lock" {
            continue;
        }
        let id = name
            .strip_suffix(".json")
            .filter(|id| inv::valid_hash(id))
            .ok_or("unexpected graph member")?;
        ids.push(id.to_owned());
        if ids.len() > 8192 {
            return Err("research graph exceeds 8192 retained links".into());
        }
    }
    ids.sort();
    ids.into_iter()
        .map(|id| {
            let path = inv::checked(dir, &format!("{id}.json"))?;
            if fs::symlink_metadata(&path)?.len() > 32768 {
                return Err("graph link exceeds 32 KiB".into());
            }
            let bytes = inv::read_bytes(&path)?;
            if inv::hash(&bytes) != id {
                return Err("retained graph link bytes changed".into());
            }
            let link: Link = serde_json::from_slice(&bytes)?;
            if link.schema_version != 1
                || !RELATIONS.contains(&link.predicate.as_str())
                || link.evidence.trim().is_empty()
                || link.evidence.len() > 16384
                || link
                    .priority
                    .is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
                || link
                    .similarity
                    .is_some_and(|v| !v.is_finite() || !(-1.0..=1.0).contains(&v))
                || !inv::valid_hash(&link.source_snapshot_sha256)
            {
                return Err("invalid retained graph link".into());
            }
            Ok((id, link))
        })
        .collect()
}
impl Graph {
    fn collect(root: &Path, retained: &[(String, Link)]) -> Result<Self> {
        let root = inv::root(root)?;
        let mut nodes = BTreeMap::new();
        let mut vectors = BTreeMap::new();
        let mut edges = Vec::new();
        if exists(&root.join("memory"))? {
            for (node, vector) in crate::memory::graph_objects(&root)? {
                let id = node["id"]
                    .as_str()
                    .ok_or("missing memory object identity")?
                    .to_owned();
                vectors.insert(id.clone(), vector);
                nodes.insert(id, node);
            }
        }
        if exists(&root.join("tasks"))? {
            let (objects, relations) = crate::tasks::graph_objects(&root)?;
            for node in objects {
                nodes.insert(
                    node["id"]
                        .as_str()
                        .ok_or("missing task object identity")?
                        .to_owned(),
                    node,
                );
            }
            edges.extend(relations);
        }
        if nodes.len() > 4096 {
            return Err("research graph exceeds 4096 object revisions".into());
        }
        let mut graph = Self {
            root,
            nodes,
            edges,
            vectors,
        };
        for (id, link) in retained {
            let similarity = graph.similarity(&link.source, &link.target);
            if similarity != link.similarity
                || link.model != similarity.map(|_| pocket_encoder::model_info())
            {
                return Err("graph link/model/vector binding changed".into());
            }
            graph.edges.push(json!({"id":id,"source":link.source,"target":link.target,"predicate":link.predicate,"evidence":link.evidence,"weights":{"similarity":similarity,"priority":link.priority},"model":link.model,"recorded_at_ms":link.recorded_at_ms,"source_snapshot_sha256":link.source_snapshot_sha256,"origin":"declared research relation; not independently verified support or causation"}));
        }
        graph.validate()?;
        Ok(graph)
    }
    fn validate(&self) -> Result<()> {
        if self.edges.len() > 16384 {
            return Err("research DAG exceeds 16384 relations".into());
        }
        let mut degrees = self
            .nodes
            .keys()
            .map(|id| (id.as_str(), 0usize))
            .collect::<BTreeMap<_, _>>();
        let mut outgoing = BTreeMap::<&str, Vec<&str>>::new();
        for edge in &self.edges {
            let from = edge["source"].as_str().ok_or("missing relation source")?;
            let to = edge["target"].as_str().ok_or("missing relation target")?;
            if from == to || !degrees.contains_key(from) || !degrees.contains_key(to) {
                return Err("research DAG relation has a self/missing exact revision".into());
            }
            *degrees.get_mut(to).unwrap() += 1;
            outgoing.entry(from).or_default().push(to);
        }
        let mut ready = degrees
            .iter()
            .filter(|(_, n)| **n == 0)
            .map(|(id, _)| *id)
            .collect::<VecDeque<_>>();
        let mut visited = 0;
        while let Some(id) = ready.pop_front() {
            visited += 1;
            for next in outgoing.get(id).into_iter().flatten() {
                let n = degrees.get_mut(next).unwrap();
                *n -= 1;
                if *n == 0 {
                    ready.push_back(next);
                }
            }
        }
        if visited != self.nodes.len() {
            return Err("relation would create a cycle in the research DAG".into());
        }
        Ok(())
    }
    fn snapshot(&self) -> Result<String> {
        Ok(inv::hash(&serde_json::to_vec(
            &json!({"objects":self.nodes.iter().map(|(id,n)|json!([id,n["sha256"]])).collect::<Vec<_>>(),"relations":self.edges}),
        )?))
    }
    fn similarity(&self, from: &str, to: &str) -> Option<f32> {
        let a = self.vectors.get(from)?;
        let b = self.vectors.get(to)?;
        Some(
            a.iter()
                .zip(b)
                .map(|(x, y)| x * y)
                .sum::<f32>()
                .clamp(-1.0, 1.0),
        )
    }
    fn index(&self) -> Result<(Vec<&str>, pocket_encoder::Index)> {
        let ids = self.vectors.keys().map(String::as_str).collect::<Vec<_>>();
        let vectors = self.vectors.values().cloned().collect();
        Ok((
            ids,
            pocket_encoder::Index::from_embeddings(pocket_encoder::model_info(), vectors)?,
        ))
    }
    fn citation(&self, id: &str) -> Value {
        let n = &self.nodes[id];
        json!({"id":id,"label":n["label"],"sha256":n["sha256"],"read_argv":n["attributes"]["read_argv"]})
    }
}
pub fn execute(args: &ArgMatches) -> Result<Value> {
    let (action, args) = args
        .subcommand()
        .ok_or("choose graph inspect, link, neighbors or clusters")?;
    let root = crate::project::resolve(args.get_one::<String>("root").map(String::as_str))?;
    let root = inv::root(&root)?;
    let held = lock(&root, action == "link")?;
    let retained = held
        .as_ref()
        .map(|(dir, _)| links(dir))
        .transpose()?
        .unwrap_or_default();
    let mut graph = Graph::collect(&root, &retained)?;
    let snapshot = graph.snapshot()?;
    let mut data = match action {
        "inspect" => {
            let offset = *args.get_one::<usize>("offset").unwrap();
            let limit = *args.get_one::<u32>("limit").unwrap() as usize;
            let nodes = graph
                .nodes
                .values()
                .skip(offset)
                .take(limit)
                .cloned()
                .collect::<Vec<_>>();
            let ids = nodes
                .iter()
                .filter_map(|n| n["id"].as_str())
                .collect::<BTreeSet<_>>();
            let edges = graph
                .edges
                .iter()
                .filter(|e| {
                    ids.contains(e["source"].as_str().unwrap())
                        && ids.contains(e["target"].as_str().unwrap())
                })
                .cloned()
                .collect::<Vec<_>>();
            json!({"nodes":nodes,"edges":edges,"coverage":{"offset":offset,"limit":limit,"returned":ids.len(),"total_objects":graph.nodes.len(),"total_relations":graph.edges.len(),"returned_relations":edges.len(),"complete":ids.len()==graph.nodes.len(),"whole_graph_verified":true},"acyclic":true})
        }
        "link" => {
            let from = args.get_one::<String>("from").unwrap();
            let to = args.get_one::<String>("to").unwrap();
            let predicate = args.get_one::<String>("relation").unwrap();
            let evidence = args.get_one::<String>("evidence-text").unwrap();
            if evidence.trim().is_empty() || evidence.len() > 16384 {
                return Err("link evidence must contain 1..16,384 UTF-8 bytes".into());
            }
            let priority = args.get_one::<f64>("priority").copied();
            if priority.is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v)) {
                return Err("declared priority must be finite and in [0,1]".into());
            }
            if graph
                .edges
                .iter()
                .any(|e| e["source"] == *from && e["target"] == *to && e["predicate"] == *predicate)
            {
                return Err(
                    "research relation already exists; retained links are immutable".into(),
                );
            }
            let similarity = graph.similarity(from, to);
            let link = Link {
                schema_version: 1,
                source: from.clone(),
                target: to.clone(),
                predicate: predicate.clone(),
                evidence: evidence.clone(),
                priority,
                similarity,
                model: similarity.map(|_| pocket_encoder::model_info()),
                source_snapshot_sha256: snapshot.clone(),
                recorded_at_ms: inv::now(),
            };
            graph.edges.push(json!({"source":from,"target":to}));
            graph.validate()?;
            let bytes = serde_json::to_vec_pretty(&link)?;
            let id = inv::hash(&bytes);
            if retained.len() >= 8192 {
                return Err("research graph link limit reached".into());
            }
            inv::exclusive(
                &held
                    .as_ref()
                    .ok_or("graph lock missing")?
                    .0
                    .join(format!("{id}.json")),
                &bytes,
            )?;
            json!({"link_id":id,"link":link,"append_only":true,"execution_started":false,"weight_semantics":"similarity is measured mean-passage cosine; priority is declared attention, neither is truth or authority"})
        }
        "neighbors" => {
            let id = args.get_one::<String>("id").unwrap();
            let vector = graph
                .vectors
                .get(id)
                .ok_or("semantic neighbors require an exact memory:SHA256 object ID")?;
            let limit = *args.get_one::<u32>("limit").unwrap() as usize;
            let (ids, index) = graph.index()?;
            let items=index.search_embedding(index.model(),vector,limit+1)?.into_iter().filter(|hit|ids[hit.index]!=id).take(limit).map(|hit|json!({"object":graph.citation(ids[hit.index]),"weights":{"similarity":hit.score,"priority":null}})).collect::<Vec<_>>();
            json!({"query_object":graph.citation(id),"items":items,"model":index.model(),"vector_basis":"L2-normalized mean of each note's retained passage vectors","persisted_relations":false,"uses_inference":false})
        }
        "clusters" => {
            let threshold = *args.get_one::<f32>("threshold").unwrap();
            if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
                return Err("cluster threshold must be finite and in [0,1]".into());
            }
            let cap = *args.get_one::<u32>("max-notes").unwrap() as usize;
            if graph.vectors.len() > cap {
                return Err("note count exceeds the explicit clustering cap; raise --max-notes within its bound or select a smaller project".into());
            }
            let (ids, index) = graph.index()?;
            let mut assigned = BTreeSet::new();
            let mut groups = Vec::new();
            for seed in &ids {
                if assigned.contains(seed) {
                    continue;
                }
                let mut members = Vec::new();
                for hit in
                    index.search_embedding(index.model(), &graph.vectors[*seed], ids.len())?
                {
                    let id = ids[hit.index];
                    if hit.score >= threshold && !assigned.contains(&id) {
                        assigned.insert(id);
                        members.push(json!({"object":graph.citation(id),"seed_cosine":hit.score}));
                    }
                }
                // The exact seed is always retained despite F32 round-off at threshold 1.
                if !assigned.contains(seed) {
                    assigned.insert(*seed);
                    members.insert(0, json!({"object":graph.citation(seed),"seed_cosine":1.0}));
                }
                groups.push(json!({"seed":seed,"members":members}));
            }
            json!({"groups":groups,"notes":ids.len(),"max_notes":cap,"threshold":threshold,"model":index.model(),"algorithm":"deterministic greedy seed-radius groups; lexicographic exact object IDs; each member meets seed threshold, not every pair","vector_basis":"L2-normalized mean of retained passage vectors","persisted_relations":false,"uses_inference":false,"meaning":"semantic navigation only; not causal families, independent observations, economic aliases or financial validation"})
        }
        _ => return Err("unknown graph operation".into()),
    };
    data["schema"] = json!("paperhands.research-dag/v1");
    data["snapshot_sha256"] = json!(snapshot);
    data["root"] = json!(graph.root);
    data["integrity_status"] = json!("verified");
    data["visibility"] = json!("private-local-research");
    data["snapshot_scope"] = json!(if action == "link" {
        "verified graph before the new link"
    } else {
        "verified retained objects and relations as read"
    });
    data["consistency"] = json!(
        "each collection is locked and verified; no cross-collection point-in-time transaction is claimed"
    );
    data["scope"] = json!(
        "immutable memory objects, all retained task revisions and explicit links; lifecycle evidence has separate verified experiment/batch views"
    );
    Ok(data)
}

/// Ready work can use declared attention weights. Financial metrics and semantic
/// cosine never become task priorities. Historical revision bindings stay cited.
pub fn attention(root: &Path) -> Result<BTreeMap<String, Value>> {
    let root = inv::root(root)?;
    let mut result = BTreeMap::new();
    let Some((dir, _lock)) = lock(&root, false)? else {
        return Ok(result);
    };
    let retained = links(&dir)?;
    let graph = Graph::collect(&root, &retained)?;
    for edge in &graph.edges {
        let Some(priority) = edge["weights"]["priority"].as_f64() else {
            continue;
        };
        let target = edge["target"].as_str().ok_or("missing graph target")?;
        let Some(task) = graph.nodes[target]["logical_id"].as_str() else {
            continue;
        };
        let entry=result.entry(task.into()).or_insert_with(||json!({"priority":priority,"links":[],"basis":"maximum declared attention on incoming links to exact task revisions; inspect historical bindings; no financial or semantic score ranking"}));
        if priority > entry["priority"].as_f64().unwrap() {
            entry["priority"] = json!(priority);
        }
        entry["links"].as_array_mut().unwrap().push(json!({"link_id":edge["id"],"source":edge["source"],"target":target,"priority":priority}));
    }
    Ok(result)
}
