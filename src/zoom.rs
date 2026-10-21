//! Read-only, bounded descriptions of retained research-loop state.
use crate::{
    Result,
    research::{inventory as inv, plan, runner},
};
use clap::{Arg, ArgMatches, Command};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

const DEFAULT_WINDOW_MS: u64 = 2 * 86_400_000;
const MAX_CLUSTER_NOTES: u32 = 1024;
const MAX_EXPERIMENTS: usize = 1024;
const MAX_TASK_REVISIONS: usize = 4096;
const MAX_WAKE_CLAIMS: usize = 4096;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Thresholds {
    pub refusal_count: u64,
    pub unreviewed_age_ms: u64,
    pub monoculture_retirements: u64,
    pub cluster_threshold: f32,
    pub no_new_evidence_ms: u64,
    pub spent_wakes: u64,
}
impl Default for Thresholds {
    fn default() -> Self {
        Self {
            refusal_count: 3,
            unreviewed_age_ms: 86_400_000,
            monoculture_retirements: 5,
            cluster_threshold: 0.65,
            no_new_evidence_ms: 172_800_000,
            spent_wakes: 2,
        }
    }
}
impl Thresholds {
    pub fn validate(&self) -> Result<()> {
        if !(1..=4096).contains(&self.refusal_count)
            || !(1..=4096).contains(&self.monoculture_retirements)
            || !(1..=4096).contains(&self.spent_wakes)
            || self.unreviewed_age_ms == 0
            || self.no_new_evidence_ms == 0
            || !self.cluster_threshold.is_finite()
            || !(0.0..=1.0).contains(&self.cluster_threshold)
        {
            return Err("zoom thresholds require positive bounded counts, positive ages and a finite cluster threshold in [0,1]".into());
        }
        Ok(())
    }
}
pub fn command() -> Command {
    Command::new("zoom")
        .about("Describe retained research-loop signals and premise questions; start no work")
        .arg(Arg::new("root").long("root"))
        .arg(
            Arg::new("since")
                .long("since")
                .value_parser(clap::value_parser!(u64))
                .help("Inclusive UTC epoch milliseconds; defaults to the past 48 hours"),
        )
        .arg(
            Arg::new("refusal-count")
                .long("refusal-count")
                .default_value("3")
                .value_parser(clap::value_parser!(u64).range(1..=4096)),
        )
        .arg(
            Arg::new("unreviewed-age-ms")
                .long("unreviewed-age-ms")
                .default_value("86400000")
                .value_parser(clap::value_parser!(u64).range(1..)),
        )
        .arg(
            Arg::new("monoculture-retirements")
                .long("monoculture-retirements")
                .default_value("5")
                .value_parser(clap::value_parser!(u64).range(1..=4096)),
        )
        .arg(
            Arg::new("cluster-threshold")
                .long("cluster-threshold")
                .default_value("0.65")
                .value_parser(clap::value_parser!(f32)),
        )
        .arg(
            Arg::new("no-new-evidence-ms")
                .long("no-new-evidence-ms")
                .default_value("172800000")
                .value_parser(clap::value_parser!(u64).range(1..)),
        )
        .arg(
            Arg::new("spent-wakes")
                .long("spent-wakes")
                .default_value("2")
                .value_parser(clap::value_parser!(u64).range(1..=4096)),
        )
}
pub fn execute(args: &ArgMatches) -> Result<Value> {
    let thresholds = Thresholds {
        refusal_count: *args.get_one::<u64>("refusal-count").unwrap(),
        unreviewed_age_ms: *args.get_one::<u64>("unreviewed-age-ms").unwrap(),
        monoculture_retirements: *args.get_one::<u64>("monoculture-retirements").unwrap(),
        cluster_threshold: *args.get_one::<f32>("cluster-threshold").unwrap(),
        no_new_evidence_ms: *args.get_one::<u64>("no-new-evidence-ms").unwrap(),
        spent_wakes: *args.get_one::<u64>("spent-wakes").unwrap(),
    };
    inspect(
        &crate::project::resolve(args.get_one::<String>("root").map(String::as_str))?,
        args.get_one::<u64>("since").copied(),
        &thresholds,
    )
}

fn exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}
fn timestamp(value: &Value, field: &str, now: u64) -> Result<u64> {
    value[field]
        .as_u64()
        .filter(|time| *time <= now)
        .ok_or_else(|| format!("missing, invalid or future {field}").into())
}
fn names(root: &Path, collection: &str, cap: usize) -> Result<Vec<PathBuf>> {
    if !exists(&root.join(collection))? {
        return Ok(vec![]);
    }
    let directory = inv::checked(root, collection)?;
    inv::root(&directory)?;
    let mut result = Vec::new();
    for entry in fs::read_dir(directory)? {
        let name = entry?
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF-8 zoom member")?;
        inv::name(&name)?;
        let path = inv::checked(root, &format!("{collection}/{name}"))?;
        inv::root(&path)?;
        result.push(path);
        if result.len() > cap {
            return Err(format!(
                "zoom {collection} scan exceeds {cap} members; coverage is unknown"
            )
            .into());
        }
    }
    result.sort();
    Ok(result)
}

#[derive(Debug)]
struct Experiment {
    id: String,
    created_at_ms: u64,
    finished_at_ms: Option<u64>,
    review: Option<Value>,
    process_status: Option<String>,
}
fn experiments(root: &Path, now: u64) -> Result<Vec<Experiment>> {
    let mut results = Vec::new();
    for directory in names(root, "experiments", MAX_EXPERIMENTS)? {
        let lock_path = inv::checked(&directory, "run.lock")?;
        inv::digest(&lock_path)?;
        let lock = fs::File::open(lock_path)?;
        fs2::FileExt::try_lock_shared(&lock)
            .map_err(|_| "zoom execution is busy; experiment coverage is unknown")?;
        let (_, p, frozen) = plan::load(&directory)?;
        if frozen.created_at_ms > now {
            return Err("future experiment creation clock".into());
        }
        let result = if exists(&directory.join("result.json"))? {
            Some(runner::verified_result(&directory)?)
        } else {
            None
        };
        let process_status = result
            .as_ref()
            .map(|value| -> Result<String> {
                let status = value["process_status"]
                    .as_str()
                    .ok_or("missing or invalid native result process_status")?;
                // runner::step can return spawn_failed; run retains that status.
                if ![
                    "command_succeeded",
                    "failed",
                    "timed_out",
                    "resource_limited",
                    "runner_failed",
                    "spawn_failed",
                ]
                .contains(&status)
                {
                    return Err("unsupported native result process_status".into());
                }
                Ok(status.to_owned())
            })
            .transpose()?;
        if let Some(result) = &result
            && (result["id"] != p.id || result["plan_sha256"] != frozen.plan_sha256)
        {
            return Err("zoom result/plan binding changed".into());
        }
        let finished_at_ms = result
            .as_ref()
            .map(|v| timestamp(v, "finished_at_ms", now))
            .transpose()?;
        let review = if exists(&directory.join("review.json"))? {
            let review: Value = inv::read_sealed(&directory.join("review.json"))?;
            if review["schema_version"] != 1
                || review["plan_sha256"] != frozen.plan_sha256
                || !review["disposition"].as_str().is_some_and(|s| {
                    ["continue", "revise", "nominate", "inconclusive", "retire"].contains(&s)
                })
                || (result.is_some()
                    && review["result_sha256"].as_str()
                        != Some(inv::digest(&directory.join("result.json"))?.as_str()))
                || (result.is_none() && !review["result_sha256"].is_null())
            {
                return Err("zoom review does not bind the current result/plan".into());
            }
            timestamp(&review, "reviewed_at_ms", now)?;
            Some(review)
        } else {
            None
        };
        results.push(Experiment {
            id: p.id,
            created_at_ms: frozen.created_at_ms,
            finished_at_ms,
            review,
            process_status,
        });
    }
    Ok(results)
}
fn task_revisions(root: &Path, now: u64) -> Result<Vec<Value>> {
    if !exists(&root.join("tasks"))? {
        return Err("no prepared native task collection".into());
    }
    let (revisions, _) = crate::tasks::graph_objects(root)?;
    if revisions.len() > MAX_TASK_REVISIONS {
        return Err("zoom task revision bound exceeded".into());
    }
    for value in &revisions {
        timestamp(value, "recorded_at_ms", now)?;
        if !value["attributes"]["state"]
            .as_str()
            .is_some_and(|s| ["open", "active", "blocked", "done", "retired"].contains(&s))
        {
            return Err("unsupported native task state".into());
        }
    }
    Ok(revisions)
}
fn note_catalog(root: &Path, now: u64) -> Result<Vec<Value>> {
    if !exists(&root.join("memory"))? {
        return Err("no prepared native memory collection".into());
    }
    let (notes, _) = crate::memory::catalog_with_coverage(root)?;
    for note in &notes {
        timestamp(note, "recorded_at_ms", now)?;
    }
    Ok(notes)
}
fn wakes(history: &Value, now: u64) -> Result<Vec<Value>> {
    if history["complete"] != true {
        return Err("automation history coverage is unknown".into());
    }
    let wakes = history["wakes"]
        .as_array()
        .ok_or("missing native automation claim inventory")?;
    if wakes.len() > MAX_WAKE_CLAIMS {
        return Err("zoom automation claim bound exceeded".into());
    }
    let mut ids = BTreeSet::new();
    for wake in wakes {
        timestamp(wake, "claimed_at_ms", now)?;
        if !wake["id"]
            .as_str()
            .is_some_and(|id| ids.insert(id.to_owned()))
        {
            return Err("duplicate or missing native wake claim identity".into());
        }
    }
    Ok(wakes.clone())
}

/// The automation module provides an immutable verified snapshot under its lock.
pub fn inspect(root: &Path, since_ms: Option<u64>, thresholds: &Thresholds) -> Result<Value> {
    let history = match crate::automation::zoom_history(root) {
        Ok(value) => value,
        Err(error) => json!({"complete":false,"error":error.to_string()}),
    };
    inspect_with_automation(root, since_ms, thresholds, &history)
}
/// Tick already holds its exclusive automation lock; this path never relocks or reads it.
pub fn inspect_with_automation(
    root: &Path,
    since_ms: Option<u64>,
    thresholds: &Thresholds,
    automation: &Value,
) -> Result<Value> {
    thresholds.validate()?;
    let root = inv::root(root)?;
    let now = inv::now();
    let since = since_ms.unwrap_or_else(|| now.saturating_sub(DEFAULT_WINDOW_MS));
    if since > now {
        return Err("zoom --since must not be in the future".into());
    }
    let notes = note_catalog(&root, now);
    let experiments = experiments(&root, now);
    let tasks = task_revisions(&root, now);
    let wakes = wakes(automation, now);
    let graph = graph_links(&root);
    build(
        &root,
        now,
        since,
        thresholds,
        notes,
        experiments,
        tasks,
        wakes,
        graph,
    )
}

fn graph_links(root: &Path) -> Result<Vec<(String, String)>> {
    if !exists(&root.join("graph"))? {
        return Ok(vec![]);
    }
    let directory = inv::checked(root, "graph")?;
    let lock_path = inv::checked(&directory, "graph.lock")?;
    inv::digest(&lock_path)?;
    let lock = fs::File::open(lock_path)?;
    fs2::FileExt::lock_shared(&lock)?;
    // Verify while this shared lock also protects the following exact-link scan.
    crate::graph::attention(root)?;
    let mut links = Vec::new();
    for entry in fs::read_dir(&directory)? {
        let name = entry?
            .file_name()
            .into_string()
            .map_err(|_| "invalid graph member")?;
        if name == "graph.lock" {
            continue;
        }
        let id = name
            .strip_suffix(".json")
            .filter(|id| inv::valid_hash(id))
            .ok_or("unexpected native graph member")?;
        let path = inv::checked(&directory, &name)?;
        if inv::digest(&path)? != id {
            return Err("native provenance link bytes changed".into());
        }
        let value: Value = inv::read(&path)?;
        links.push((
            value["source"]
                .as_str()
                .ok_or("missing provenance source")?
                .to_owned(),
            value["target"]
                .as_str()
                .ok_or("missing provenance target")?
                .to_owned(),
        ));
        if links.len() > 8192 {
            return Err("zoom graph link bound exceeded".into());
        }
    }
    Ok(links)
}
fn unknown(kind: &str, reason: impl ToString) -> Value {
    let mut value = json!({"kind":kind,"fired":null,"evidence":[],"reason":reason.to_string()});
    for key in match kind {
        "repeated_refusal" => &["count", "same_reason", "since_ms"][..],
        "unreviewed_output" => &["count", "oldest_age_ms"][..],
        "monoculture" => &["retired", "clusters", "dominant_share"][..],
        "no_new_evidence" => &["age_ms"][..],
        _ => &["wakes", "task_state_changes"][..],
    } {
        value[key] = Value::Null;
    }
    value
}
fn refused(tasks: &[Value], since: u64, threshold: u64) -> Result<Value> {
    let mut groups = BTreeMap::<(String, String), Vec<&Value>>::new();
    for task in tasks.iter().filter(|v| {
        v["recorded_at_ms"].as_u64().is_some_and(|at| at >= since)
            && v["attributes"]["state"] == "blocked"
    }) {
        let id = task["logical_id"]
            .as_str()
            .ok_or("missing native task identity")?;
        let note = task["text"]
            .as_str()
            .ok_or("missing native blocked-task note")?;
        if !note.trim().is_empty() {
            groups
                .entry((id.to_owned(), note.to_owned()))
                .or_default()
                .push(task);
        }
    }
    let mut groups = groups.into_iter().collect::<Vec<_>>();
    groups.sort_by(|(a, x), (b, y)| y.len().cmp(&x.len()).then(a.cmp(b)));
    let selected = groups.first();
    let count = selected.map_or(0, |(_, events)| events.len());
    Ok(
        json!({"kind":"repeated_refusal","count":count,"same_reason":selected.map(|((_,note),_)|note),"task":selected.map(|((id,_),_)|id),"since_ms":selected.and_then(|(_,events)|events.iter().filter_map(|v|v["recorded_at_ms"].as_u64()).min()).unwrap_or(since),"evidence":selected.map(|(_,events)|events.iter().map(|v|v["id"].clone()).collect::<Vec<_>>()).unwrap_or_default(),"fired":count as u64>=threshold,"basis":"same task identity and identical nonempty native blocked-event note; declared reason, not independently verified refusal"}),
    )
}
fn changes(tasks: &[Value], since: u64) -> Result<u64> {
    let mut histories = BTreeMap::<&str, Vec<&Value>>::new();
    for task in tasks {
        histories
            .entry(
                task["logical_id"]
                    .as_str()
                    .ok_or("missing native task identity")?,
            )
            .or_default()
            .push(task);
    }
    let mut count = 0;
    for history in histories.values_mut() {
        history.sort_by_key(|v| v["attributes"]["sequence"].as_u64());
        for pair in history.windows(2) {
            if pair[1]["recorded_at_ms"]
                .as_u64()
                .is_some_and(|time| time >= since)
                && pair[0]["attributes"]["state"] != pair[1]["attributes"]["state"]
            {
                count += 1;
            }
        }
    }
    Ok(count)
}
fn monoculture(
    root: &Path,
    tasks: &[Value],
    experiments: &[Experiment],
    links: &[(String, String)],
    since: u64,
    thresholds: &Thresholds,
) -> Result<Value> {
    let mut retired = Vec::<(u64, String)>::new();
    let mut histories = BTreeMap::<&str, Vec<&Value>>::new();
    for task in tasks {
        histories
            .entry(task["logical_id"].as_str().ok_or("missing task identity")?)
            .or_default()
            .push(task);
    }
    for history in histories.values_mut() {
        history.sort_by_key(|v| v["attributes"]["sequence"].as_u64());
        for pair in history.windows(2) {
            if pair[1]["attributes"]["state"] == "retired"
                && pair[0]["attributes"]["state"] != "retired"
            {
                let at = pair[1]["recorded_at_ms"]
                    .as_u64()
                    .ok_or("missing retirement clock")?;
                if at >= since {
                    retired.push((
                        at,
                        pair[1]["id"]
                            .as_str()
                            .ok_or("missing task revision")?
                            .to_owned(),
                    ));
                }
            }
        }
    }
    for experiment in experiments {
        if let Some(review) = &experiment.review
            && review["disposition"] == "retire"
        {
            let at = review["reviewed_at_ms"]
                .as_u64()
                .ok_or("missing retirement clock")?;
            if at >= since {
                retired.push((at, format!("experiment:{}", experiment.id)));
            }
        }
    }
    retired.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let total = retired.len();
    retired.truncate(thresholds.monoculture_retirements as usize);
    let evidence = retired.iter().map(|(_, id)| id.clone()).collect::<Vec<_>>();
    if total < thresholds.monoculture_retirements as usize {
        return Ok(
            json!({"kind":"monoculture","retired":retired.len(),"retirements_in_window":total,"clusters":null,"dominant_share":null,"evidence":evidence,"fired":false,"reason":"fewer than the configured number of native retirements; no concentration inferred"}),
        );
    }
    let args = crate::graph::command().try_get_matches_from([
        "graph",
        "clusters",
        "--root",
        root.to_str().ok_or("non-UTF-8 root")?,
        "--threshold",
        &thresholds.cluster_threshold.to_string(),
        "--max-notes",
        &MAX_CLUSTER_NOTES.to_string(),
    ])?;
    let clustered = crate::graph::execute(&args)?;
    let mut cluster_for = BTreeMap::<String, String>::new();
    for group in clustered["groups"]
        .as_array()
        .ok_or("missing native note clusters")?
    {
        let seed = group["seed"].as_str().ok_or("missing cluster seed")?;
        for member in group["members"]
            .as_array()
            .ok_or("missing cluster members")?
        {
            let id = member["object"]["id"]
                .as_str()
                .ok_or("missing note citation")?;
            cluster_for.insert(id.to_owned(), seed.to_owned());
        }
    }
    let mut counts = BTreeMap::<String, usize>::new();
    for (_, id) in &retired {
        let targets = links
            .iter()
            .filter_map(|(from, to)| {
                if from == id {
                    cluster_for.get(to)
                } else if to == id {
                    cluster_for.get(from)
                } else {
                    None
                }
            })
            .collect::<BTreeSet<_>>();
        if targets.len() != 1 {
            return Err("native retirement has no unambiguous explicit provenance link to one retained-note cluster".into());
        }
        *counts
            .entry((*targets.into_iter().next().unwrap()).clone())
            .or_default() += 1;
    }
    let dominant = counts.values().copied().max().unwrap_or(0) as f64 / retired.len() as f64;
    Ok(
        json!({"kind":"monoculture","retired":retired.len(),"retirements_in_window":total,"clusters":counts.len(),"dominant_share":dominant,"evidence":evidence,"fired":counts.len()==1,"threshold":thresholds.cluster_threshold,"model":clustered["model"],"basis":"explicit native provenance links to deterministic retained-note semantic clusters; not causal families or independent evidence"}),
    )
}

#[allow(clippy::too_many_arguments)]
fn build(
    root: &Path,
    now: u64,
    since: u64,
    thresholds: &Thresholds,
    notes: Result<Vec<Value>>,
    experiments: Result<Vec<Experiment>>,
    tasks: Result<Vec<Value>>,
    wakes: Result<Vec<Value>>,
    graph: Result<Vec<(String, String)>>,
) -> Result<Value> {
    let mut gaps = Vec::new();
    for (collection, error) in [
        ("memory", notes.as_ref().err()),
        ("experiments", experiments.as_ref().err()),
        ("tasks", tasks.as_ref().err()),
        ("automation", wakes.as_ref().err()),
        ("graph", graph.as_ref().err()),
    ] {
        if let Some(error) = error {
            gaps.push(json!({"collection":collection,"error":error.to_string()}));
        }
    }
    let mut signals = vec![
        unknown("repeated_refusal", "native task history unavailable"),
        unknown("unreviewed_output", "native output history unavailable"),
        unknown(
            "monoculture",
            "native retirement/provenance history unavailable",
        ),
        unknown("no_new_evidence", "native recording history unavailable"),
        unknown(
            "spent_without_change",
            "native claim/task history unavailable",
        ),
    ];
    if let Ok(tasks) = &tasks {
        signals[0] = refused(tasks, since, thresholds.refusal_count)?;
    }
    signals[0]["lifecycle_errors"] = experiments
        .as_ref()
        .ok()
        .map(|items| {
            items
                .iter()
                .filter(|item| {
                    item.finished_at_ms.is_some_and(|at| at >= since)
                        && item.process_status.as_deref().is_some_and(|s| {
                            [
                                "failed",
                                "runner_failed",
                                "timed_out",
                                "resource_limited",
                                "spawn_failed",
                                "interrupted",
                            ]
                            .contains(&s)
                        })
                })
                .count()
        })
        .map_or(Value::Null, |count| json!(count));
    if let Ok(experiments) = &experiments {
        // A current backlog is not dropped merely because it predates the lookback window.
        let unreviewed = experiments
            .iter()
            .filter(|item| item.review.is_none() && item.finished_at_ms.is_some())
            .collect::<Vec<_>>();
        let oldest = unreviewed
            .iter()
            .filter_map(|item| item.finished_at_ms)
            .min()
            .map(|time| now - time);
        signals[1] = json!({"kind":"unreviewed_output","count":unreviewed.len(),"oldest_age_ms":oldest,"evidence":unreviewed.iter().map(|e|e.id.as_str()).collect::<Vec<_>>(),"fired":oldest.is_some_and(|age|age>thresholds.unreviewed_age_ms),"basis":"all current native completed outputs lacking a bound review, including backlog before --since"});
    }
    if let (Ok(tasks), Ok(experiments), Ok(links)) = (&tasks, &experiments, &graph) {
        signals[2] = match monoculture(root, tasks, experiments, links, since, thresholds) {
            Ok(signal) => signal,
            Err(error) => {
                gaps.push(
                    json!({"collection":"retirement_cluster_mapping","error":error.to_string()}),
                );
                unknown("monoculture", error)
            }
        };
    }
    if let (Ok(notes), Ok(experiments)) = (&notes, &experiments) {
        let latest = notes
            .iter()
            .filter_map(|v| v["recorded_at_ms"].as_u64())
            .chain(experiments.iter().map(|e| e.created_at_ms))
            .max();
        let age = latest.map(|time| now - time);
        signals[3] = json!({"kind":"no_new_evidence","age_ms":age,"fired":age.map(|age|age>thresholds.no_new_evidence_ms),"evidence":[],"basis":"latest native note recording or frozen experiment creation across verified history; not independent evidence, novelty, or substantive review"});
        if latest.is_none() {
            gaps.push(json!({"collection":"native_recording_clock","error":"no retained native note or experiment recording time establishes an age"}));
        }
    }
    if let (Ok(tasks), Ok(wakes)) = (&tasks, &wakes) {
        let changes = changes(tasks, since)?;
        let claims = wakes
            .iter()
            .filter(|v| {
                v["claimed_at_ms"]
                    .as_u64()
                    .is_some_and(|time| time >= since)
            })
            .collect::<Vec<_>>();
        signals[4] = json!({"kind":"spent_without_change","wakes":claims.len(),"task_state_changes":changes,"evidence":claims.iter().map(|v|v["id"].clone()).collect::<Vec<_>>(),"fired":claims.len() as u64>=thresholds.spent_wakes&&changes==0,"basis":"spent native intent claims, including ambiguous outcomes; task changes exclude initial creation and same-state note updates; not delivery or completed work"});
    }
    let fired = signals
        .iter()
        .filter(|s| s["fired"] == true)
        .map(|s| s["kind"].clone())
        .collect::<Vec<_>>();
    let coverage_complete = gaps.is_empty() && signals.iter().all(|s| s["fired"].is_boolean());
    let stalled = signals[0]["fired"] == true
        || (signals[1]["fired"] == true
            && signals[1]["oldest_age_ms"]
                .as_u64()
                .is_some_and(|age| age >= thresholds.unreviewed_age_ms.saturating_mul(2)))
        || (signals[4]["fired"] == true && signals[3]["fired"] == true);
    let verdict = if !coverage_complete {
        None
    } else if stalled {
        Some("stalled")
    } else if !fired.is_empty() {
        Some("drifting")
    } else {
        Some("moving")
    };
    let prompts = [
        "Which premise or admission condition would let the repeatedly blocked task progress?",
        "Which retained outputs need substantive review before another experiment?",
        "Which mechanism outside this semantic neighborhood deserves a discriminating test?",
        "What new observation could change the current explanation? Recording freshness alone is not novelty.",
        "What should the next authorized wake change rather than merely repeat?",
    ];
    let mut questions = signals
        .iter()
        .zip(prompts)
        .filter(|(signal, _)| signal["fired"] == true)
        .map(|(_, question)| question)
        .collect::<Vec<_>>();
    if !coverage_complete {
        questions.push("Which missing or unverifiable retained records must be inspected before describing the loop?");
    }
    Ok(
        json!({"schema":"paperhands.zoom/v1","generated_at_ms":now,"window_ms":now-since,"since_ms":since,"signals":signals,"signals_fired":fired,"verdict":verdict,"verdict_rule":"paperhands.zoom.verdict/v1","coverage_complete":coverage_complete,"gaps":gaps,"questions":questions,"thresholds":thresholds,"limitations":["Only retained native records are inspected; local receipt clocks do not establish market/event time or independent new evidence.","Blocked task reasons are caller-declared exact notes; lifecycle errors are counted separately. Imported note prose, P&L and retirement alone do not establish refusal or successful research.","Unreviewed-output backlog and the latest recording clock use all verified retained history. Other event counts use the inclusive --since window.","Monoculture needs the configured number of native retirements plus unambiguous explicit graph links to retained-note clusters. Missing mappings remain unknown; no retired prose is embedded. Semantic clusters are navigation, not causal families or independent observations.","Unknown required coverage makes the verdict null, including an empty recording history. A verdict describes signals and grants no action authority.","Each collection is verified under its own existing lock; no cross-collection transactional snapshot is claimed.","No provider, evaluator, order, task mutation or promotion is performed."],"bounds":{"experiments":MAX_EXPERIMENTS,"task_revisions":MAX_TASK_REVISIONS,"wake_claims":MAX_WAKE_CLAIMS,"cluster_notes":MAX_CLUSTER_NOTES,"graph_links":8192},"execution_started":false,"uses_inference":false}),
    )
}
