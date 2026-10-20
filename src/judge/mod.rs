//! Narrow probabilistic judges, retained provider evidence and offline evals.
mod analysis;
mod provider;

use crate::{Result, research::inventory as inv};
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_CASES: usize = 4096;
const MAX_REQUEST_BYTES: usize = 65_536;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub group: String,
    pub fold: usize,
    pub text: String,
    pub failure: Option<bool>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dataset {
    pub schema: String,
    pub id: String,
    pub data_use: String,
    pub folds: usize,
    pub cases: Vec<Case>,
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Rubric {
    pub id: String,
    pub failure: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pricing {
    pub input_usd_per_million: f64,
    pub output_usd_per_million: f64,
    pub source: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub max_calls: usize,
    pub wall_time_ms: u64,
    pub request_timeout_ms: u64,
    pub max_total_request_bytes: usize,
    pub max_response_bytes: usize,
    pub max_total_response_bytes: usize,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatasetRef {
    pub path: String,
    pub sha256: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: String,
    pub id: String,
    pub model: String,
    pub dataset: DatasetRef,
    pub rubric: Rubric,
    pub limits: Limits,
    pub pricing: Option<Pricing>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Score {
    pub id: String,
    pub status: String,
    pub score: Option<f64>,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(default)]
    pub wall_time_ms: Option<u64>,
    #[serde(default)]
    pub error: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scores {
    pub schema: String,
    pub id: String,
    pub dataset_sha256: String,
    pub provider: String,
    pub model: String,
    pub rubric: Rubric,
    pub pricing: Option<Pricing>,
    pub cases: Vec<Score>,
}

fn required(name: &'static str) -> Arg {
    Arg::new(name).long(name).required(true)
}
pub fn command() -> Command {
    Command::new("judge").about("Retain narrow decision-model scores; tune and calibrate evals offline without taking trading actions")
        .subcommand(Command::new("validate").about("Validate a pinned TypeSafe plan and exact dataset; no credentials, network or writes").arg(required("plan")))
        .subcommand(Command::new("run").about("Spend one fresh judge identity on bounded TypeSafe calls; retain every case and failure, with no retries").arg(required("plan")).arg(Arg::new("root").long("root")).arg(Arg::new("key-file").long("key-file").help("Private TYPESAFE_API_KEY env file; otherwise environment then ~/.config/paperhands/typesafe.env")))
        .subcommand(Command::new("inspect").about("Verify all retained run/analysis bytes; distinguish an unsealed interrupted run").arg(required("dir")))
        .subcommand(Command::new("analyze").about("Use saved scores and fixed grouped folds for constrained thresholds, calibration and cost comparisons; no model calls")
            .arg(required("dataset")).arg(required("scores").action(ArgAction::Append)).arg(required("out"))
            .arg(required("min-precision").value_parser(clap::value_parser!(f64)))
            .arg(required("min-recall").value_parser(clap::value_parser!(f64)))
            .arg(Arg::new("beta").long("beta").default_value("1").value_parser(clap::value_parser!(f64))))
}
fn path<'a>(args: &'a ArgMatches, key: &str) -> &'a Path {
    Path::new(args.get_one::<String>(key).unwrap())
}
pub fn execute(args: &ArgMatches) -> Result<Value> {
    let (action, args) = args
        .subcommand()
        .ok_or("choose judge validate, run, inspect or analyze")?;
    match action {
        "validate" => {
            let p = prepare(path(args, "plan"))?;
            Ok(
                json!({"validated":true,"plan_sha256":inv::hash(&p.plan_bytes),"dataset_sha256":p.plan.dataset.sha256,"cases":p.dataset.cases.len(),"limits":p.plan.limits,"endpoint":provider::ENDPOINT,"network_used":false,"writes":false,"labels_sent":false,"max_request_bytes":MAX_REQUEST_BYTES}),
            )
        }
        "run" => {
            let p = prepare(path(args, "plan"))?;
            let root = crate::project::resolve(args.get_one::<String>("root").map(String::as_str))?;
            let key = provider::credential(args.get_one::<String>("key-file").map(Path::new))?;
            let client = provider::client()?;
            run_prepared(p, &root, &key, |body, timeout, cap| {
                provider::post(&client, &key, body, timeout, cap)
            })
        }
        "inspect" => inspect(path(args, "dir")),
        "analyze" => analysis::analyze(
            path(args, "dataset"),
            &args
                .get_many::<String>("scores")
                .unwrap()
                .map(|s| Path::new(s).to_path_buf())
                .collect::<Vec<_>>(),
            path(args, "out"),
            *args.get_one::<f64>("min-precision").unwrap(),
            *args.get_one::<f64>("min-recall").unwrap(),
            *args.get_one::<f64>("beta").unwrap(),
        ),
        _ => Err("unsupported judge operation".into()),
    }
}

fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    if fs::symlink_metadata(path)?.len() > MAX_FILE_BYTES {
        return Err("judge JSON exceeds 8 MiB".into());
    }
    inv::read_bytes(path)
}
pub(super) fn dataset(bytes: &[u8]) -> Result<Dataset> {
    let data: Dataset = serde_json::from_slice(bytes)?;
    if data.schema != "paperhands.judge-dataset/v1" {
        return Err("unsupported judge dataset schema".into());
    }
    inv::name(&data.id)?;
    if !["synthetic", "development", "holdout", "forward"].contains(&data.data_use.as_str())
        || !(2..=20).contains(&data.folds)
        || data.cases.is_empty()
        || data.cases.len() > MAX_CASES
    {
        return Err(
            "judge dataset requires a data-use label, 2..20 fixed folds and 1..4096 cases".into(),
        );
    }
    let mut ids = BTreeSet::new();
    let mut groups = BTreeMap::new();
    let mut folds = BTreeSet::new();
    for row in &data.cases {
        inv::name(&row.id)?;
        inv::name(&row.group)?;
        if !ids.insert(&row.id)
            || row.fold >= data.folds
            || row.text.trim().is_empty()
            || row.text.len() > 60_000
        {
            return Err("duplicate/invalid judge case, fold or text (maximum 60000 bytes)".into());
        }
        if groups
            .insert(&row.group, row.fold)
            .is_some_and(|fold| fold != row.fold)
        {
            return Err("related judge group crosses training/validation folds".into());
        }
        folds.insert(row.fold);
    }
    if folds.len() != data.folds {
        return Err("every declared judge fold must have cases".into());
    }
    Ok(data)
}
fn rubric(r: &Rubric) -> Result<()> {
    inv::name(&r.id)?;
    if r.failure.trim().is_empty() || r.failure.len() > 16_384 {
        return Err("failure rubric must contain 1..16384 bytes".into());
    }
    Ok(())
}
fn pricing(p: &Option<Pricing>) -> Result<()> {
    if let Some(p) = p
        && (!p.input_usd_per_million.is_finite()
            || !p.output_usd_per_million.is_finite()
            || !(0.0..=1_000_000.0).contains(&p.input_usd_per_million)
            || !(0.0..=1_000_000.0).contains(&p.output_usd_per_million)
            || p.source.trim().is_empty()
            || p.source.len() > 4096)
    {
        return Err(
            "pricing requires finite nonnegative declared USD token rates and a source".into(),
        );
    }
    Ok(())
}
pub(super) fn scores(bytes: &[u8], data: &Dataset, data_hash: &str) -> Result<Scores> {
    let s: Scores = serde_json::from_slice(bytes)?;
    if s.schema != "paperhands.judge-scores/v1" || s.dataset_sha256 != data_hash {
        return Err("saved score schema or exact dataset hash mismatch".into());
    }
    inv::name(&s.id)?;
    rubric(&s.rubric)?;
    pricing(&s.pricing)?;
    if s.provider.trim().is_empty()
        || s.provider.len() > 80
        || s.model.trim().is_empty()
        || s.model.len() > 200
    {
        return Err("saved scores require provider and model identities".into());
    }
    let expected = data
        .cases
        .iter()
        .map(|c| c.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for row in &s.cases {
        if !seen.insert(row.id.as_str()) || !expected.contains(row.id.as_str()) {
            return Err("duplicate or unmatched saved score case".into());
        }
        if !["scored", "failed", "timed_out", "not_started"].contains(&row.status.as_str())
            || (row.status == "scored") != row.score.is_some()
            || row
                .score
                .is_some_and(|x| !x.is_finite() || !(0.0..=1.0).contains(&x))
        {
            return Err("saved score status/probability invalid; unknown must remain null".into());
        }
    }
    if seen != expected {
        return Err("saved score case set differs from the fixed dataset".into());
    }
    Ok(s)
}

struct Prepared {
    plan: Plan,
    dataset: Dataset,
    plan_bytes: Vec<u8>,
    data_bytes: Vec<u8>,
}
fn prepare(path: &Path) -> Result<Prepared> {
    let plan_bytes = read_bytes(path)?;
    let plan: Plan = serde_json::from_slice(&plan_bytes)?;
    if plan.schema != "paperhands.judge-plan/v1" {
        return Err("unsupported judge plan schema".into());
    }
    inv::name(&plan.id)?;
    rubric(&plan.rubric)?;
    pricing(&plan.pricing)?;
    let version = plan
        .model
        .strip_prefix("jev-")
        .ok_or("pin an exact Jev model, such as jev-1.13.0")?;
    let parts = version.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.bytes().all(|c| c.is_ascii_digit()))
    {
        return Err("Jev aliases are not allowed; pin an exact numeric model version".into());
    }
    let l = &plan.limits;
    if !(1..=MAX_CASES).contains(&l.max_calls)
        || !(1..=3_600_000).contains(&l.wall_time_ms)
        || !(1..=l.wall_time_ms).contains(&l.request_timeout_ms)
        || !(1..=8 * 1024 * 1024).contains(&l.max_total_request_bytes)
        || !(1..=1024 * 1024).contains(&l.max_response_bytes)
        || !(1..=16 * 1024 * 1024).contains(&l.max_total_response_bytes)
    {
        return Err("invalid judge call/time/request/response limits".into());
    }
    let parent = fs::canonicalize(
        path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
    let data_bytes = read_bytes(&inv::checked(&parent, &plan.dataset.path)?)?;
    if !inv::valid_hash(&plan.dataset.sha256) || inv::hash(&data_bytes) != plan.dataset.sha256 {
        return Err("judge dataset hash mismatch".into());
    }
    let dataset = dataset(&data_bytes)?;
    for case in &dataset.cases {
        if request(&plan, case)?.len() > MAX_REQUEST_BYTES {
            return Err("judge request exceeds 65536 bytes; shorten state/rubric".into());
        }
    }
    Ok(Prepared {
        plan,
        dataset,
        plan_bytes,
        data_bytes,
    })
}
fn request(plan: &Plan, case: &Case) -> Result<Vec<u8>> {
    // Labels, groups and fold assignments never leave this process.
    Ok(serde_json::to_vec(
        &json!({"model":plan.model,"state":case.text,"questions":{"failure":{"type":"noul","instructions":plan.rubric.failure}}}),
    )?)
}

pub(super) fn private_dir(path: &Path) -> Result<()> {
    let mut options = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        options.mode(0o700);
    }
    options.create(path)?;
    Ok(())
}
pub(super) fn write(path: &Path, bytes: &[u8]) -> Result<()> {
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
pub(super) fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    write(path, &serde_json::to_vec_pretty(value)?)
}
pub(super) fn seal(dir: &Path) -> Result<()> {
    let inventory = inv::scan(&dir.join("evidence"), "data", None)?;
    let bytes = serde_json::to_vec_pretty(&inventory)?;
    write(&dir.join("inventory.json"), &bytes)?;
    write_json(
        &dir.join("seal.json"),
        &json!({"sha256":inv::hash(&bytes),"schema":"paperhands.judge-seal/v1"}),
    )
}

fn parse_answer(bytes: &[u8], model: &str) -> Result<(f64, Usage)> {
    let body: Value = serde_json::from_slice(bytes).map_err(|_| "invalid_provider_json")?;
    if body["model"] != model {
        return Err("provider_model_pin_mismatch".into());
    }
    let answers = body["answers"]
        .as_object()
        .ok_or("missing_provider_answers")?;
    if answers.len() != 1 || body["answers"]["failure"]["type"] != "noul" {
        return Err("provider_answer_contract_mismatch".into());
    }
    let p = body["answers"]["failure"]["noul"]
        .as_f64()
        .ok_or("missing_failure_probability")?;
    if !p.is_finite() || !(0.0..=1.0).contains(&p) {
        return Err("invalid_failure_probability".into());
    }
    let usage = Usage {
        input_tokens: body["usage"]["input_tokens"]
            .as_u64()
            .ok_or("missing_provider_usage")?,
        output_tokens: body["usage"]["output_tokens"]
            .as_u64()
            .ok_or("missing_provider_usage")?,
    };
    Ok((p, usage))
}

fn run_prepared(
    mut p: Prepared,
    root: &Path,
    key: &str,
    mut call: impl FnMut(&[u8], Duration, usize) -> provider::Exchange,
) -> Result<Value> {
    let start = Instant::now();
    let root = inv::root(root)?;
    let collection = inv::subdirectory(&root, "judges")?;
    let dir = collection.join(&p.plan.id);
    private_dir(&dir)?; // Exclusive reservation. An interrupted identity remains spent.
    let evidence = dir.join("evidence");
    private_dir(&evidence)?;
    write(&evidence.join("plan.json"), &p.plan_bytes)?;
    write(&evidence.join("dataset.json"), &p.data_bytes)?;
    write_json(
        &evidence.join("started.json"),
        &json!({"schema":"paperhands.judge-start/v1","id":p.plan.id,"kind":"run","started_at_ms":inv::now(),"pid":std::process::id(),"engine_version":crate::VERSION,"plan_sha256":inv::hash(&p.plan_bytes),"dataset_sha256":p.plan.dataset.sha256,"endpoint":provider::ENDPOINT,"labels_sent":false,"automatic_retries":false,"max_request_bytes":MAX_REQUEST_BYTES}),
    )?;
    let mut records = Vec::new();
    let mut calls = 0;
    let mut sent = 0;
    let mut received = 0;
    let mut stop = None;
    for (index, case) in p.dataset.cases.iter().enumerate() {
        let body = request(&p.plan, case)?;
        let l = &p.plan.limits;
        let elapsed = start.elapsed().as_millis() as u64;
        let reason = stop.or_else(|| {
            if elapsed >= l.wall_time_ms {
                Some("total_wall_time_limit")
            } else if calls >= l.max_calls {
                Some("call_limit")
            } else if body.len() > l.max_total_request_bytes.saturating_sub(sent) {
                Some("request_byte_limit")
            } else if received >= l.max_total_response_bytes {
                Some("total_response_byte_limit")
            } else {
                None
            }
        });
        let mut row = Score {
            id: case.id.clone(),
            status: "not_started".into(),
            score: None,
            usage: None,
            wall_time_ms: None,
            error: reason.map(Into::into),
        };
        if reason.is_none() {
            write(&evidence.join(format!("{index:04}-request.json")), &body)?;
            let timeout = Duration::from_millis(
                l.request_timeout_ms.min(
                    l.wall_time_ms
                        .saturating_sub(start.elapsed().as_millis() as u64),
                ),
            );
            if timeout.is_zero() {
                row.error = Some("total_wall_time_limit".into());
            } else {
                write_json(
                    &evidence.join(format!("{index:04}-started.json")),
                    &json!({"case":case.id,"started_at_ms":inv::now(),"request_sha256":inv::hash(&body),"timeout_ms":timeout.as_millis()}),
                )?;
                let timeout = Duration::from_millis(
                    l.request_timeout_ms.min(
                        l.wall_time_ms
                            .saturating_sub(start.elapsed().as_millis() as u64),
                    ),
                );
                if timeout.is_zero() {
                    row.error = Some("total_wall_time_limit".into());
                    records.push(row);
                    continue;
                }
                calls += 1;
                sent += body.len();
                let then = Instant::now();
                let cap = l
                    .max_response_bytes
                    .min(l.max_total_response_bytes.saturating_sub(received));
                let response = call(&body, timeout, cap);
                let elapsed = then.elapsed();
                received += response.body.len();
                let (redacted, changed) = provider::redact(&response.body, key);
                write(
                    &evidence.join(format!("{index:04}-response.bin")),
                    &redacted,
                )?;
                let late =
                    elapsed >= timeout || start.elapsed().as_millis() as u64 >= l.wall_time_ms;
                let error = if late {
                    Some("request_or_total_wall_time_limit".into())
                } else if let Some(error) = response.error {
                    Some(error.into())
                } else if !response.status.is_some_and(|s| (200..300).contains(&s)) {
                    Some("provider_http_failure".into())
                } else {
                    match parse_answer(&response.body, &p.plan.model) {
                        Ok((score, usage)) => {
                            row.score = Some(score);
                            row.usage = Some(usage);
                            None
                        }
                        Err(error) => Some(error.to_string()),
                    }
                };
                row.status = if row.score.is_some() {
                    "scored"
                } else if late || response.error == Some("request_timeout") {
                    "timed_out"
                } else {
                    "failed"
                }
                .into();
                row.wall_time_ms = Some(elapsed.as_millis() as u64);
                row.error = error;
                write_json(
                    &evidence.join(format!("{index:04}-receipt.json")),
                    &json!({"http_status":response.status,"received_prefix_bytes":response.body.len(),"retained_bytes":redacted.len(),"credential_redacted":changed,"complete_body":response.error.is_none(),"row":row}),
                )?;
                if matches!(response.status, Some(401 | 403 | 429)) {
                    stop = Some("provider_auth_or_rate_limit");
                }
            }
        }
        records.push(row);
    }
    let scored = records.iter().filter(|r| r.status == "scored").count();
    let exit = if scored == records.len() { 0 } else { 3 };
    let saved = Scores {
        schema: "paperhands.judge-scores/v1".into(),
        id: p.plan.id.clone(),
        dataset_sha256: p.plan.dataset.sha256.clone(),
        provider: "typesafe".into(),
        model: p.plan.model.clone(),
        rubric: p.plan.rubric.clone(),
        pricing: p.plan.pricing.take(),
        cases: records,
    };
    write_json(&evidence.join("scores.json"), &saved)?;
    let result = json!({"schema":"paperhands.judge-result/v1","kind":"run","id":p.plan.id,"process_status":"completed","evidence_status":"unreviewed","exit_code":exit,"cases":saved.cases.len(),"scored":scored,"failed_or_unknown":saved.cases.len()-scored,"calls":calls,"request_bytes":sent,"received_prefix_bytes":received,"wall_time_ms":start.elapsed().as_millis() as u64,"finished_at_ms":inv::now(),"plan_sha256":inv::hash(&p.plan_bytes),"dataset_sha256":saved.dataset_sha256,"scores_sha256":inv::digest(&evidence.join("scores.json"))?,"actions_executed":false,"scope":"model judgments only; no experiment promotion, financial accounting, venue truth or orders"});
    write_json(&evidence.join("result.json"), &result)?;
    seal(&dir)?;
    Ok(
        json!({"dir":dir,"integrity_status":"verified","exit_code":exit,"result":result,"scores":evidence.join("scores.json")}),
    )
}

pub fn inspect(dir: &Path) -> Result<Value> {
    let dir = inv::root(dir)?;
    let evidence = inv::checked(&dir, "evidence")?;
    if !dir.join("seal.json").exists() {
        let started: Value =
            serde_json::from_slice(&read_bytes(&inv::checked(&evidence, "started.json")?)?)?;
        return Ok(
            json!({"id":started["id"],"kind":started["kind"],"dir":dir,"started":started,"process_status":"unknown","evidence_status":"unsealed","integrity_status":"not_sealed","exit_code":3,"scope":"interrupted or still running; do not resume/reuse this identity"}),
        );
    }
    let seal: Value = serde_json::from_slice(&read_bytes(&inv::checked(&dir, "seal.json")?)?)?;
    let inventory = inv::load(
        &inv::checked(&dir, "inventory.json")?,
        seal["sha256"]
            .as_str()
            .ok_or("judge seal has no inventory hash")?,
    )?;
    inv::verify(&evidence, &inventory)?;
    let result: Value =
        serde_json::from_slice(&read_bytes(&inv::checked(&evidence, "result.json")?)?)?;
    Ok(
        json!({"id":result["id"],"kind":result["kind"],"dir":dir,"integrity_status":"verified","process_status":"completed","evidence_status":"unreviewed","result":result,"exit_code":0,"scope":"verified retained bytes, not independent financial or probability validation"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(dir: &Path, id: &str) -> std::path::PathBuf {
        let data = json!({"schema":"paperhands.judge-dataset/v1","id":"fixture","data_use":"synthetic","folds":2,"cases":[
            {"id":"a-pass","group":"a","fold":0,"text":"Evidence supports the claim.","failure":false},
            {"id":"a-fail","group":"a","fold":0,"text":"Claim without source.","failure":true},
            {"id":"b-pass","group":"b","fold":1,"text":"Evidence supports the claim.","failure":false},
            {"id":"b-fail","group":"b","fold":1,"text":"Claim without source.","failure":true}]});
        let data = serde_json::to_vec(&data).unwrap();
        fs::write(dir.join("dataset.json"), &data).unwrap();
        let plan = json!({"schema":"paperhands.judge-plan/v1","id":id,"model":"jev-1.13.0","dataset":{"path":"dataset.json","sha256":inv::hash(&data)},"rubric":{"id":"source","failure":"Does the trace contain an unsupported claim?"},"limits":{"max_calls":4,"wall_time_ms":10000,"request_timeout_ms":2000,"max_total_request_bytes":65536,"max_response_bytes":4096,"max_total_response_bytes":16384},"pricing":null});
        let path = dir.join("plan.json");
        fs::write(&path, serde_json::to_vec(&plan).unwrap()).unwrap();
        path
    }
    fn success() -> provider::Exchange {
        provider::Exchange {status:Some(200),body:serde_json::to_vec(&json!({"model":"jev-1.13.0","answers":{"failure":{"type":"noul","noul":0.4}},"usage":{"input_tokens":12,"output_tokens":0}})).unwrap(),error:None}
    }

    #[test]
    fn retained_runs_keep_partial_failures_redact_credentials_and_spend_identity_once() {
        let temp = tempfile::tempdir().unwrap();
        let plan = fixture(temp.path(), "partial");
        let mut calls = 0;
        let result = run_prepared(
            prepare(&plan).unwrap(),
            temp.path(),
            "fake-private-key-123",
            |body, _, _| {
                calls += 1;
                let value: Value = serde_json::from_slice(body).unwrap();
                assert!(
                    value.get("failure").is_none()
                        && value.get("fold").is_none()
                        && value.get("group").is_none()
                );
                if calls == 1 {
                    success()
                } else {
                    provider::Exchange {
                        status: Some(401),
                        body: b"reflected fake-private-key-123".to_vec(),
                        error: None,
                    }
                }
            },
        )
        .unwrap();
        assert_eq!(result["exit_code"], 3);
        assert_eq!(calls, 2);
        let dir = temp.path().join("judges/partial");
        let saved: Scores =
            serde_json::from_slice(&fs::read(dir.join("evidence/scores.json")).unwrap()).unwrap();
        assert_eq!(
            saved
                .cases
                .iter()
                .map(|r| r.status.as_str())
                .collect::<Vec<_>>(),
            vec!["scored", "failed", "not_started", "not_started"]
        );
        assert_eq!(saved.cases[2].score, None);
        let raw = fs::read_to_string(dir.join("evidence/0001-response.bin")).unwrap();
        assert!(!raw.contains("fake-private-key-123") && raw.contains("[REDACTED]"));
        assert_eq!(inspect(&dir).unwrap()["result"]["exit_code"], 3);
        assert!(
            run_prepared(
                prepare(&plan).unwrap(),
                temp.path(),
                "fake-private-key-123",
                |_, _, _| panic!("duplicate must not execute")
            )
            .is_err()
        );
        fs::write(dir.join("evidence/scores.json"), b"{}").unwrap();
        assert!(inspect(&dir).is_err());
    }

    #[test]
    fn concurrent_identity_is_refused_and_late_success_remains_a_timeout() {
        let temp = tempfile::tempdir().unwrap();
        let plan = fixture(temp.path(), "concurrent");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let worker_barrier = barrier.clone();
        let root = temp.path().to_path_buf();
        let worker_plan = plan.clone();
        let worker = std::thread::spawn(move || {
            let mut first = true;
            run_prepared(
                prepare(&worker_plan).unwrap(),
                &root,
                "fake-key",
                |_, _, _| {
                    if first {
                        first = false;
                        worker_barrier.wait();
                        worker_barrier.wait();
                    }
                    success()
                },
            )
            .unwrap()
        });
        barrier.wait();
        assert!(
            run_prepared(
                prepare(&plan).unwrap(),
                temp.path(),
                "fake-key",
                |_, _, _| panic!("concurrent duplicate")
            )
            .is_err()
        );
        barrier.wait();
        assert_eq!(worker.join().unwrap()["exit_code"], 0);
        let mut late = prepare(&plan).unwrap();
        late.plan.id = "late".into();
        late.plan.limits.request_timeout_ms = 5;
        late.plan.limits.max_calls = 1;
        late.plan_bytes = serde_json::to_vec(&late.plan).unwrap();
        let result = run_prepared(late, temp.path(), "fake-key", |_, _, _| {
            std::thread::sleep(Duration::from_millis(20));
            success()
        })
        .unwrap();
        assert_eq!(result["exit_code"], 3);
        let saved: Scores = serde_json::from_slice(
            &fs::read(temp.path().join("judges/late/evidence/scores.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(saved.cases[0].status, "timed_out");
        assert_eq!(saved.cases[0].score, None);
        assert_eq!(saved.cases[1].status, "not_started");
    }

    #[test]
    fn provider_contract_and_frozen_dataset_refuse_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let plan = fixture(temp.path(), "pin");
        let bytes = success().body;
        assert_eq!(parse_answer(&bytes, "jev-1.13.0").unwrap().0, 0.4);
        assert!(parse_answer(&bytes, "jev-1.14.0").is_err());
        assert!(
            parse_answer(
                br#"{"model":"jev-1.13.0","answers":{"failure":{"type":"noul","noul":2}}}"#,
                "jev-1.13.0"
            )
            .is_err()
        );
        let mut p: Value = serde_json::from_slice(&fs::read(&plan).unwrap()).unwrap();
        p["dataset"]["path"] = json!("../escape.json");
        fs::write(&plan, serde_json::to_vec(&p).unwrap()).unwrap();
        assert!(prepare(&plan).is_err());
        let plan = fixture(temp.path(), "pin");
        fs::write(temp.path().join("dataset.json"), b"{}").unwrap();
        assert!(prepare(&plan).is_err());
    }
}
