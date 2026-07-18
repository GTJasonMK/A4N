use a4n_llm::provider::ChatRequest;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::blueprint::BlueprintProgressReporter;

use super::super::contracts::BlueprintStageSpec;

pub(in crate::blueprint::generation) struct BlueprintStageRunInput<'a> {
    pub(in crate::blueprint::generation) project_id: i64,
    pub(in crate::blueprint::generation) spec: &'a BlueprintStageSpec,
    pub(in crate::blueprint::generation) progress: BlueprintProgressReporter<'a>,
    pub(in crate::blueprint::generation) cancel_token: Option<&'a CancellationToken>,
    pub(in crate::blueprint::generation) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::blueprint::generation) base_prompt: &'a str,
    pub(in crate::blueprint::generation) user_payload_json: &'a str,
    pub(in crate::blueprint::generation) previous_context: Option<&'a Value>,
    pub(in crate::blueprint::generation) workflow_run_id: Option<i64>,
}

pub(super) struct PreparedBlueprintStage<'a> {
    pub(super) spec: &'a BlueprintStageSpec,
    pub(super) progress: BlueprintProgressReporter<'a>,
    pub(super) cancel_token: Option<&'a CancellationToken>,
    pub(super) request: ChatRequest,
    pub(super) workflow_run_id: i64,
    pub(super) call_key: String,
    pub(super) fingerprint_json: Value,
}

pub(super) struct BlueprintStageLlmOutput {
    pub(super) response_content: String,
    pub(super) usage: Option<Value>,
    pub(super) finish_reason: Option<String>,
}
