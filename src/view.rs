//! Renderer-neutral views derived from the same verified evidence as inspection.
use crate::{
    Result,
    research::{batch, inventory as inv, runner},
};
use clap::{Arg, ArgMatches, Command};
use serde_json::{Value, json};
use std::path::Path;

pub const RESOURCE_URI: &str = "ui://paperhands/evidence";
pub const RESOURCE_MIME: &str = "text/html;profile=mcp-app";

pub fn command() -> Command {
    let leaf = |name| {
        Command::new(name)
            .about("Verify retained evidence and return paged graph, chart and table data")
            .arg(Arg::new("dir").long("dir").required(true))
            .arg(
                Arg::new("offset")
                    .long("offset")
                    .default_value("0")
                    .value_parser(clap::value_parser!(usize)),
            )
            .arg(
                Arg::new("limit")
                    .long("limit")
                    .default_value("40")
                    .value_parser(clap::value_parser!(u32).range(1..=50)),
            )
    };
    Command::new("view").about("Create private evidence views for any frontend, with offline Pele/Mermaid presentation")
        .subcommand(leaf("experiment")).subcommand(leaf("batch")).subcommand(leaf("desk")).subcommand(leaf("graph"))
}

pub fn execute(args: &ArgMatches) -> Result<Value> {
    let (kind, args) = args.subcommand().ok_or("choose view experiment or batch")?;
    let dir = Path::new(args.get_one::<String>("dir").unwrap());
    let offset = *args.get_one::<usize>("offset").unwrap();
    let limit = *args.get_one::<u32>("limit").unwrap() as usize;
    if kind == "desk" {
        return desk(dir, offset, limit);
    }
    if kind == "graph" {
        return research_graph(dir, offset, limit);
    }
    let evidence = match kind {
        "experiment" => runner::inspect(dir)?,
        "batch" => batch::inspect(dir)?,
        _ => return Err("unsupported view".into()),
    };
    let id = evidence["id"].as_str().unwrap_or("unfinished-batch");
    let rows = match kind {
        "experiment" => evidence["result"]["steps"].as_array().into_iter().flatten().enumerate().map(|(i,s)|json!({
            "id":format!("step-{i:03}"),"process_status":s["process_status"],"evidence_status":evidence["evidence_status"],
            "exit_code":s["exit_code"],"wall_time_ms":s["wall_time_ms"],"cpu_time_ms":s["cpu_time_ms"],
            "peak_rss_bytes":s["reported_peak_rss_bytes"],"stdout_bytes":s["stdout"]["observed_bytes"],"stderr_bytes":s["stderr"]["observed_bytes"]
        })).collect::<Vec<_>>(),
        _ => evidence["members"].as_array().into_iter().flatten().map(|m|json!({
            "id":m["id"],"process_status":m["process_status"],"evidence_status":m["evidence_status"],
            "exit_code":m["exit_code"],"wall_time_ms":m["wall_time_ms"],
            "cpu_time_ms":m["steps"].as_array().map(|steps|steps.iter().filter_map(|s|s["cpu_time_ms"].as_u64()).sum::<u64>()),
            "peak_rss_bytes":m["steps"].as_array().and_then(|steps|steps.iter().filter_map(|s|s["reported_peak_rss_bytes"].as_u64()).max()),
            "stdout_bytes":m["steps"].as_array().map(|steps|steps.iter().filter_map(|s|s["stdout"]["observed_bytes"].as_u64()).sum::<u64>()),
            "stderr_bytes":m["steps"].as_array().map(|steps|steps.iter().filter_map(|s|s["stderr"]["observed_bytes"].as_u64()).sum::<u64>())
        })).collect::<Vec<_>>()
    };
    let total = rows.len();
    let selected = rows
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let mut nodes = vec![
        json!({"id":"root","kind":kind,"label":id,"process_status":evidence["process_status"]}),
    ];
    let mut edges = Vec::new();
    let mut source = format!("flowchart LR\n  root[\"{}\"]\n", mermaid_label(id));
    for (i, row) in selected.iter().enumerate() {
        let node = format!("n{i}");
        nodes.push(json!({"id":node,"kind":if kind=="batch"{"attempt"}else{"command"},"label":row["id"],"process_status":row["process_status"],"evidence_status":row["evidence_status"]}));
        edges.push(json!({"source":"root","target":node,"relationship":if kind=="batch"{"contains"}else{"executes"}}));
        source.push_str(&format!(
            "  {node}[\"{} / {}\"]\n  root --> {node}\n",
            mermaid_label(row["id"].as_str().unwrap_or("unknown")),
            mermaid_label(row["process_status"].as_str().unwrap_or("unknown"))
        ));
    }
    let next_offset = offset.saturating_add(selected.len());
    let coverage = json!({"offset":offset,"limit":limit,"returned":selected.len(),"total":total,"complete_selection":offset==0&&selected.len()==total,"next_offset":if next_offset<total{Some(next_offset)}else{None},"integrity_scope":if kind=="batch"{"all completed batch members, including members outside this page"}else{"whole attempt, including retained steps outside this page"}});
    Ok(json!({
        "schema":"paperhands.view/v1","generated_at_ms":inv::now(),"visibility":"private-local-evidence","kind":kind,"id":id,
        "process_status":evidence["process_status"],"evidence_status":evidence["evidence_status"],
        "review_scope":if kind=="batch"{"historical completion only; inspect individual attempts for current reviews"}else{"current verified attempt inspection"},
        "integrity_status":if kind=="batch"{evidence["integrity_status"].clone()}else{json!(if evidence["result"].is_null(){"not_sealed"}else{"verified"})},
        "bindings":{"plan_sha256":evidence["plan_sha256"],"result_scope":evidence["result_scope"]},
        "coverage":coverage,"graph":{"nodes":nodes,"edges":edges},
        "datasets":[{"id":"processes","columns":[
            {"name":"id","type":"string"},{"name":"process_status","type":"string"},{"name":"evidence_status","type":"string","nullable":true},
            {"name":"exit_code","type":"number","nullable":true},{"name":"wall_time_ms","type":"number","unit":"ms","nullable":true},
            {"name":"cpu_time_ms","type":"number","unit":"ms","nullable":true},{"name":"peak_rss_bytes","type":"number","unit":"bytes","nullable":true},
            {"name":"stdout_bytes","type":"number","unit":"bytes","nullable":true},{"name":"stderr_bytes","type":"number","unit":"bytes","nullable":true}],"rows":selected}],
        "views":[{"id":"lineage","type":"graph","graph":"graph"},{"id":"duration","type":"bar","dataset":"processes","x":"id","y":"wall_time_ms","color":"process_status"},{"id":"process-table","type":"table","dataset":"processes"}],
        "presentation":{"language":"mermaid","source":source,"renderer":"pele","resource_uri":RESOURCE_URI,"rendered_by":"client","links":false,"images":false},
        "markdown":format!("### Paperhands {kind} evidence\n\n```mermaid\n{source}```\n\nThis is a private, paged process view. Null measurements are unknown. It does not establish independent policies, financial fills or profit."),
        "limitations":["No financial metrics or portfolio returns are inferred.","Shared labels and resource measurements do not establish independent evidence.","Unfinished output is not sealed; an empty page does not establish no planned or active work.","This private view is not the allowlisted public export."]
    }))
}

pub fn research_graph(root: &Path, offset: usize, limit: usize) -> Result<Value> {
    let argv = vec![
        "graph".into(),
        "inspect".into(),
        "--root".into(),
        root.to_str().ok_or("graph path must be UTF-8")?.into(),
        "--offset".into(),
        offset.to_string(),
        "--limit".into(),
        limit.to_string(),
    ];
    let result = crate::runtime::invoke(&argv, &json!({}));
    if result.exit_code != 0 {
        return Err(result.value["error"]["message"]
            .as_str()
            .unwrap_or("graph verification failed")
            .into());
    }
    let data = &result.value["data"];
    let objects = data["nodes"].as_array().ok_or("graph nodes missing")?;
    let names = objects
        .iter()
        .enumerate()
        .map(|(i, n)| (n["id"].as_str().unwrap(), format!("n{i}")))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut source = String::from("flowchart LR\n");
    for object in objects {
        let kind = object["kind"].as_str().unwrap_or("object");
        let detail = if let Some(state) = object["attributes"]["state"].as_str() {
            format!(
                "{kind} {state} r{}",
                object["attributes"]["sequence"].as_u64().unwrap_or(0)
            )
        } else {
            kind.into()
        };
        source.push_str(&format!(
            "  {}[\"{} / {}\"]\n",
            names[object["id"].as_str().unwrap()],
            mermaid_label(object["label"].as_str().unwrap_or("unknown")),
            mermaid_label(&detail)
        ));
    }
    let edges = data["edges"].as_array().ok_or("graph edges missing")?;
    for edge in edges.iter().take(256) {
        let mut label = mermaid_label(edge["predicate"].as_str().unwrap_or("relation"));
        if let Some(priority) = edge["weights"]["priority"].as_f64() {
            label.push_str(&format!(" p {priority:.2}"));
        }
        if let Some(similarity) = edge["weights"]["similarity"].as_f64() {
            label.push_str(&format!(" s {similarity:.2}"));
        }
        source.push_str(&format!(
            "  {} -->|\"{label}\"| {}\n",
            names[edge["source"].as_str().unwrap()],
            names[edge["target"].as_str().unwrap()]
        ));
    }
    if objects.is_empty() {
        source.push_str("  empty[\"No objects on this page\"]\n");
    }
    let rows=objects.iter().map(|n|json!({"id":n["id"],"label":n["label"],"kind":n["kind"],"state":n["attributes"]["state"],"revision":n["attributes"]["sequence"],"recorded_at_ms":n["recorded_at_ms"],"sha256":n["sha256"]})).collect::<Vec<_>>();
    Ok(
        json!({"schema":"paperhands.view/v1","generated_at_ms":inv::now(),"visibility":"private-local-research","kind":"graph","id":"research-dag","integrity_status":"verified","snapshot_sha256":data["snapshot_sha256"],
        "review_scope":"exact retained object revisions and declared links; similarity and priority do not establish truth",
        "coverage":{"offset":offset,"limit":limit,"returned":objects.len(),"total":data["coverage"]["total_objects"],"total_relations":data["coverage"]["total_relations"],"returned_relations":edges.len(),"drawn_relations":edges.len().min(256),"whole_graph_verified":true},
        "graph":{"nodes":objects,"edges":edges,"acyclic":true},"datasets":[{"id":"objects","columns":[{"name":"id","type":"string"},{"name":"label","type":"string"},{"name":"kind","type":"string"},{"name":"state","type":"string","nullable":true},{"name":"revision","type":"number","nullable":true},{"name":"recorded_at_ms","type":"number","unit":"local epoch ms"},{"name":"sha256","type":"string"}],"rows":rows}],
        "views":[{"id":"research-dag","type":"graph","graph":"graph"},{"id":"object-ledger","type":"table","dataset":"objects"}],"presentation":{"language":"mermaid","source":source,"renderer":"pele","resource_uri":RESOURCE_URI,"rendered_by":"client"},
        "markdown":format!("### Paperhands research DAG\n\n```mermaid\n{source}```\n\np = declared attention; s = measured semantic cosine. Edges cite exact revisions. This private page does not validate source claims or independent market evidence."),
        "limitations":["Whole retained graph verified; only relationships between selected page nodes are returned.","Rendered edges are capped at 256; typed edge data retains the complete selected page.","Graph consistency is per collection, not an atomic cross-collection transaction."]}),
    )
}

pub fn desk(root: &Path, offset: usize, limit: usize) -> Result<Value> {
    let data = crate::desk::inspect(root, offset, limit)?;
    let rows = data["rows"].as_array().ok_or("desk rows missing")?;
    let tasks = data["tasks"].as_array().ok_or("desk tasks missing")?;
    let mut nodes = vec![json!({"id":"root","kind":"desk","label":"Research desk"})];
    let mut edges = Vec::new();
    let mut source = String::from("flowchart LR\n  root[\"Research desk\"]\n");
    let selected = tasks.iter().chain(rows).take(50).collect::<Vec<_>>();
    let task_names = selected
        .iter()
        .enumerate()
        .filter(|(_, v)| v["depends_on"].is_array())
        .filter_map(|(i, v)| v["id"].as_str().map(|id| (id, format!("n{i}"))))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut omitted_edges = 0;
    for (i, item) in selected.iter().enumerate() {
        let name = format!("n{i}");
        let label = item["title"]
            .as_str()
            .or(item["id"].as_str())
            .unwrap_or("unknown");
        let state = item["state"]
            .as_str()
            .or(item["process_status"].as_str())
            .unwrap_or("unknown");
        nodes.push(json!({"id":name,"kind":item["kind"],"label":label,"state":state,"reference":item["id"],"revision_sha256":item["event_sha256"],"depends_on":item["depends_on"]}));
        source.push_str(&format!(
            "  {name}[\"{} / {}\"]\n",
            mermaid_label(label),
            mermaid_label(state)
        ));
        let dependencies = item["depends_on"].as_array();
        if let Some(deps) = dependencies.filter(|deps| !deps.is_empty()) {
            for dependency in deps {
                if let Some(parent) = dependency.as_str().and_then(|id| task_names.get(id)) {
                    if edges.len() < 256 {
                        edges.push(json!({"source":parent,"target":name,"relationship":"prerequisite_of","evidence":{"task":item["id"],"event_sha256":item["event_sha256"]}}));
                        source.push_str(&format!("  {parent} --> {name}\n"));
                    } else {
                        omitted_edges += 1;
                    }
                } else {
                    omitted_edges += 1;
                }
            }
        } else {
            edges.push(json!({"source":"root","target":name,"relationship":"tracks"}));
            source.push_str(&format!("  root --> {name}\n"));
        }
    }
    let total = data["coverage"]["total_attempts_and_batches"]
        .as_u64()
        .unwrap_or(0) as usize;
    let next = offset.saturating_add(rows.len());
    let integrity = if data["gaps"].as_array().is_some_and(|g| !g.is_empty()) {
        "gaps"
    } else if data["coverage"]["all_evidence_inspected"] == true {
        "verified"
    } else {
        "verified_selection"
    };
    Ok(
        json!({"schema":"paperhands.view/v1","generated_at_ms":data["generated_at_ms"],"visibility":"private-local-evidence","kind":"desk","id":"research-workspace","integrity_status":integrity,
        "review_scope":"explicit task decisions and current selected experiment inspections; no automatic promotion",
        "coverage":{"offset":offset,"limit":limit,"returned":rows.len(),"total":total,"complete_selection":offset==0&&rows.len()==total,"next_offset":if next<total{Some(next)}else{None}},
        "graph":{"nodes":nodes,"edges":edges,"coverage":{"selected":tasks.len()+rows.len(),"shown":(tasks.len()+rows.len()).min(50),"complete":tasks.len()+rows.len()<=50&&omitted_edges==0,"omitted_dependency_edges":omitted_edges,"direction":"prerequisite to dependent; status is a projection of the task DAG"}},
        "datasets":[{"id":"processes","columns":[{"name":"id","type":"string"},{"name":"kind","type":"string"},{"name":"process_status","type":"string"},{"name":"evidence_status","type":"string","nullable":true},{"name":"wall_time_ms","type":"number","unit":"ms","nullable":true},{"name":"exit_code","type":"number","nullable":true}],"rows":rows}],
        "views":[{"id":"research-map","type":"graph","graph":"graph"},{"id":"duration","type":"bar","dataset":"processes","x":"id","y":"wall_time_ms"},{"id":"research-frontier","type":"task-list","items":"desk.frontier"},{"id":"questions","type":"note-list","items":"desk.questions"}],
        "presentation":{"language":"mermaid","source":source,"renderer":"pele","resource_uri":RESOURCE_URI,"rendered_by":"client"},
        "markdown":format!("### Paperhands research desk\n\n{} notes · {} tasks · {total} experiments/batches. Integrity: {integrity}.\n\n```mermaid\n{source}```\n\nRead the structured frontier and coverage before choosing work. There is no automatic financial promotion.",data["notes_total"],data["tasks_total"]),
        "desk":data,"limitations":["Tasks and notes are explicit research material, not independent financial evidence.","Only selected experiments/batches are inspected; page coverage is explicit.","The binary supplies instruments and memory; the external agent supplies research reasoning."]}),
    )
}

fn mermaid_label(text: &str) -> String {
    text.chars()
        .take(100)
        .map(|c| {
            if c.is_ascii_alphanumeric() || " -_:./()".contains(c) {
                c
            } else {
                ' '
            }
        })
        .collect()
}

pub fn resource() -> Value {
    json!({"uri":RESOURCE_URI,"name":"paperhands-evidence","mimeType":RESOURCE_MIME,"description":"Embedded offline Pele graph, duration chart and process table","_meta":{"ui":{"csp":{"connectDomains":[],"resourceDomains":[],"frameDomains":[]},"prefersBorder":true}}})
}

pub fn resource_content() -> Value {
    let mut value = resource();
    let scripts = [
        include_str!("../assets/viewer/vendor/purify.min.js"),
        include_str!("../assets/viewer/vendor/pele.global.js"),
        include_str!("../assets/viewer/app.js"),
    ]
    .iter()
    .map(|s| format!("<script>{}</script>", s.replace("</script", "<\\/script")))
    .collect::<Vec<_>>()
    .join("\n");
    value["text"] = include_str!("../assets/viewer/index.html")
        .replace("<!-- SCRIPTS -->", &scripts)
        .into();
    value
}
