use super::{
    inventory as inv, plan,
    runner::{self, Budget},
};
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeSet, VecDeque},
    fs,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    pub schema_version: u32,
    pub id: String,
    pub attempts: Vec<String>,
    pub max_parallel: usize,
    pub wall_time_ms: u64,
    pub total_output_bytes: u64,
    pub total_artifact_bytes: u64,
}
pub fn run(store: &Path, path: &Path) -> Result<Value> {
    let store = inv::root(store)?;
    let batch: Batch = inv::read(path)?;
    inv::name(&batch.id)?;
    if batch.schema_version != 1
        || batch.attempts.is_empty()
        || batch.attempts.len() > 4096
        || !(1..=256).contains(&batch.max_parallel)
        || !(1..=86_400_000).contains(&batch.wall_time_ms)
        || batch.total_output_bytes == 0
        || batch.total_artifact_bytes == 0
    {
        return Err("invalid explicit batch bounds".into());
    }
    let mut unique = BTreeSet::new();
    let mut plans = Vec::new();
    for id in &batch.attempts {
        inv::name(id)?;
        if !unique.insert(id) {
            return Err("duplicate batch member".into());
        }
        let dir = store.join("experiments").join(id);
        let (_, _, f) = plan::load(&dir)?;
        if dir.join("start.json").exists() || dir.join("queue.json").exists() {
            return Err("batch member identity already spent or queued".into());
        }
        plans.push(json!({"id":id,"plan_sha256":f.plan_sha256}));
    }
    inv::subdirectory(&store, "batches")?;
    let dir = store.join("batches").join(&batch.id);
    fs::create_dir(&dir)?;
    inv::sealed_json(&dir.join("plan.json"), &batch)?;
    fs::create_dir(dir.join("members"))?;
    for id in &batch.attempts {
        inv::sealed_json(
            &store.join("experiments").join(id).join("queue.json"),
            &json!({"batch":batch.id,"queued_at_ms":inv::now()}),
        )?;
    }
    inv::sealed_json(
        &dir.join("start.json"),
        &json!({"started_at_ms":inv::now(),"runner_pid":std::process::id(),"members":plans}),
    )?;
    let began = Instant::now();
    let budget = Arc::new(Budget {
        deadline: began + Duration::from_millis(batch.wall_time_ms),
        output_limit: batch.total_output_bytes,
        artifact_limit: batch.total_artifact_bytes,
        output: AtomicU64::new(0),
        artifacts: AtomicU64::new(0),
    });
    let queue = Arc::new(Mutex::new(VecDeque::from(batch.attempts.clone())));
    let active = AtomicU64::new(0);
    let peak = AtomicU64::new(0);
    let failures = Mutex::new(Vec::<String>::new());
    std::thread::scope(|scope| {
        for _ in 0..batch.max_parallel.min(batch.attempts.len()) {
            let queue = queue.clone();
            let budget = budget.clone();
            let store = &store;
            let dir = &dir;
            let batch = &batch;
            let active = &active;
            let peak = &peak;
            let failures = &failures;
            scope.spawn(move||loop{
                let id=queue.lock().expect("batch queue").pop_front();let Some(id)=id else{break;};
                let result=if budget.exhausted(){json!({"process_status":"not_started_budget","id":id,"exit_code":3})}else{
                    let n=active.fetch_add(1,Ordering::SeqCst)+1;peak.fetch_max(n,Ordering::SeqCst);
                    let result=std::panic::catch_unwind(||runner::run(&store.join("experiments").join(&id),Some(budget.clone()),Some(&batch.id)));
                    active.fetch_sub(1,Ordering::SeqCst);
                    match result {Ok(Ok(v))=>v,Ok(Err(e))=>json!({"id":id,"process_status":"runner_failed","error":e.to_string(),"exit_code":3}),Err(_)=>json!({"id":id,"process_status":"runner_interrupted","exit_code":3})}
                };
                if let Err(e)=inv::sealed_json(&dir.join("members").join(format!("{id}.json")),&result){failures.lock().expect("batch failures").push(e.to_string());}
            });
        }
    });
    let mut members = Vec::new();
    let mut states = std::collections::BTreeMap::<String, u64>::new();
    let mut cpu = 0u64;
    let mut rss = 0u64;
    for id in &batch.attempts {
        let path = dir.join("members").join(format!("{id}.json"));
        let value: Value = if path.exists() {
            inv::read_sealed(&path)?
        } else {
            json!({"id":id,"process_status":"record_missing","exit_code":3})
        };
        *states
            .entry(value["process_status"].as_str().unwrap_or("invalid").into())
            .or_default() += 1;
        if let Some(steps) = value["steps"].as_array() {
            for s in steps {
                cpu += s["cpu_time_ms"].as_u64().unwrap_or(0);
                rss = rss.max(s["reported_peak_rss_bytes"].as_u64().unwrap_or(0));
            }
        }
        members.push(value);
    }
    let all_ok = members.iter().all(|v| v["exit_code"] == 0);
    let result = json!({"id":batch.id,"process_status":"batch_finished","evidence_status":"unreviewed","planned_members":batch.attempts.len(),"members":members,"states":states,"wall_time_ms":began.elapsed().as_millis(),"observed_output_bytes":budget.output.load(Ordering::SeqCst),"charged_artifact_bytes":budget.artifacts.load(Ordering::SeqCst),"cpu_time_ms":cpu,"max_reported_child_rss_bytes":rss,"memory_scope":"maximum child peak, not simultaneous batch RSS","peak_workers":peak.load(Ordering::SeqCst),"record_failures":failures.into_inner().expect("failures"),"exit_code":if all_ok{0}else{3},"promotion":"none"});
    inv::sealed_json(&dir.join("result.json"), &result)?;
    Ok(result)
}
pub fn inspect(dir: &Path) -> Result<Value> {
    let _: Batch = inv::read_sealed(&dir.join("plan.json"))?;
    if dir.join("result.json").exists() {
        inv::read_sealed(&dir.join("result.json"))
    } else {
        Ok(
            json!({"process_status":"batch_running_or_interrupted","evidence_status":"unreviewed","scope":"inspect member start/queue/result records; no automatic restart"}),
        )
    }
}
