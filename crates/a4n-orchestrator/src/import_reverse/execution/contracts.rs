use a4n_api_contract::import_reverse::ReverseEngineeringTarget;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

pub(super) struct PreparedImportReverseCall {
    pub(super) target: ReverseEngineeringTarget,
    pub(super) request: ChatRequest,
    pub(super) workflow_run_id: i64,
    pub(super) fingerprint_json: Value,
}
