use a4n_domain::inspiration::InspirationMessageRecord;
use a4n_domain::source_text_hash;
use a4n_llm_provider_contract::provider::ChatRequest;
use serde_json::{Value, json};

pub(super) struct PreparedBlueprintRefine {
    pub(super) expected_source_snapshot: Value,
    pub(super) request: ChatRequest,
    pub(super) input_fingerprint_json: Value,
    pub(super) call_key: String,
    pub(super) workflow_run_id: i64,
}

pub(super) fn blueprint_refine_source_snapshot(
    source_rows: &a4n_db::outline::OutlineGenerationSourceRows,
    inspiration_messages: &[InspirationMessageRecord],
) -> Value {
    let mut snapshot = a4n_db::outline::outline_generation_source_snapshot(source_rows);
    let history = a4n_inspiration_core::inspiration_messages_llm_context(inspiration_messages);
    snapshot["inspiration_history_sha256"] = json!(source_text_hash(
        &serde_json::to_string(&history).expect("LLM context history must serialize")
    ));
    snapshot
}
