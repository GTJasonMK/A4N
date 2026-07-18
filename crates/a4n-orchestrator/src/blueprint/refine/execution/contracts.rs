use a4n_core::{blueprint::ParsedBlueprintOptimization, models::project::novel_blueprint};
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

pub(super) struct PreparedBlueprintRefine {
    pub(super) blueprint: novel_blueprint::Model,
    pub(super) next_version: String,
    pub(super) request: ChatRequest,
    pub(super) input_fingerprint_json: Value,
    pub(super) call_key: String,
    pub(super) workflow_run_id: i64,
}

pub(super) struct BlueprintRefinementPersistenceInput {
    pub(super) project_id: i64,
    pub(super) blueprint: novel_blueprint::Model,
    pub(super) next_version: String,
    pub(super) parsed: ParsedBlueprintOptimization,
}
