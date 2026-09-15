use a4n_llm_provider_contract::provider::ChatRequest;
use serde_json::Value;

pub(super) struct PreparedBlueprintRefine {
    pub(super) expected_source_snapshot: Value,
    pub(super) request: ChatRequest,
    pub(super) input_fingerprint_json: Value,
    pub(super) call_key: String,
    pub(super) workflow_run_id: i64,
}
