use a4n_llm::provider::ChatRequest;
use serde_json::Value;

pub(super) struct PreparedImportReverseCall {
    pub(super) prompt_name: String,
    pub(super) request: ChatRequest,
    pub(super) workflow_run_id: i64,
    pub(super) call_key: String,
    pub(super) fingerprint_json: Value,
}
