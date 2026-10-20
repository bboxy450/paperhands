//! Inspect the public payload compiled into this executable.
use crate::{GUIDES, SKILL, VERSION, research::inventory::hash};
use serde_json::{Value, json};

pub fn manifest() -> Value {
    let asset = |id: &str, bytes: &[u8], license: &str| json!({"id":id,"bytes":bytes.len(),"sha256":hash(bytes),"license":license});
    json!({"runtime_version":VERSION,"self_contained_runtime_assets":true,
        "guides":GUIDES.iter().map(|(id,path,text)|json!({"id":id,"source":path,"bytes":text.len(),"sha256":hash(text.as_bytes())})).collect::<Vec<_>>(),
        "skill":{"id":"paperhands","sha256":hash(SKILL.as_bytes()),"refresh":"every invocation"},
        "semantic_help":{"sha256":hash(include_bytes!("../assets/how-to.json")),"model":clap_how_to::model_info(),"network_required":false},
        "companion":{"routes_sha256":hash(include_bytes!("../assets/companion.json")),"mode":"recorded-state templates and cited retrieval; external agent supplies reasoning"},
        "research_encoder":{"crate":"pocket-encoder","version":"0.1.1","search_feature":true,"model":pocket_encoder::model_info(),"note_vectors":"retained once per note; coordinates and source bindings verified on read"},
        "rendering_assets":[
            asset("pele@1.6.0",include_bytes!("../assets/viewer/vendor/pele.global.js"),"MIT"),
            asset("dompurify@3.4.17",include_bytes!("../assets/viewer/vendor/purify.min.js"),"Apache-2.0"),
            asset("paperhands-viewer",include_bytes!("../assets/viewer/app.js"),"MIT")
        ],
        "examples":crate::examples::manifest(),
        "research_store":"project-bound append-only objects/text, task/research DAG and weighted provenance beside retained experiments/snapshots; live research data is not written into the installed executable",
        "external_evaluators":"declared executable and runtime dependencies remain external; no venue engine or Python environment is bundled",
        "inspiration":{"crate":"ontology","version":"1.0.0","patterns":["shared dispatch","bounded stdio MCP","typed next argv","embedded offline Pele/MCP Apps"],"notices":"assets/viewer/vendor/NOTICES.md"}
    })
}
