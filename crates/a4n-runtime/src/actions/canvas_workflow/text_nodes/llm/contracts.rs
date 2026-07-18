use super::super::super::node_lifecycle::CanvasRunHandle;
use a4n_llm::provider::UsageInfo;
use serde_json::Value;

pub(super) struct CanvasLlmNodeLifecycle {
    pub(super) run: CanvasRunHandle,
    pub(super) project_id: i64,
    pub(super) graph_id: i64,
    pub(super) mode: String,
    pub(super) step_key: String,
    pub(super) input_json: Value,
    pub(super) generation_spec_id: i64,
}

pub(super) struct CanvasLlmCheckpointWrite {
    pub(super) call_order: i32,
    pub(super) input_fingerprint_json: Value,
}

pub(super) struct CanvasLlmCallResult {
    pub(super) call_key: String,
    pub(super) response_content: String,
    pub(super) usage: Option<UsageInfo>,
    pub(super) finish_reason: Option<String>,
    pub(super) restored_parsed_text: Option<String>,
    pub(super) checkpoint_write: Option<CanvasLlmCheckpointWrite>,
}

pub(super) struct ValidatedCanvasLlmText {
    pub(super) transformed_text: String,
    pub(super) usage: Option<UsageInfo>,
    pub(super) finish_reason: Option<String>,
}
