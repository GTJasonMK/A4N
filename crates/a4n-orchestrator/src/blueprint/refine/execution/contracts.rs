use a4n_core::blueprint::ParsedBlueprintOptimization;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

pub(super) struct PreparedBlueprintRefine {
    pub(super) expected_source_snapshot: Value,
    pub(super) request: ChatRequest,
    pub(super) input_fingerprint_json: Value,
    pub(super) call_key: String,
    pub(super) workflow_run_id: i64,
}

pub(super) struct BlueprintRefinementPersistenceInput {
    pub(super) project_id: i64,
    pub(super) expected_source_snapshot: Value,
    pub(super) parsed: ParsedBlueprintOptimization,
}
