use a4n_llm::provider::ChatRequest;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use std::sync::Arc;

use crate::generation::{
    checkpoint::GenerationCheckpointWriter,
    events::{GenerationEventPublisher, GenerationTaskSession},
};

use super::super::contracts::BlueprintStageSpec;

pub(in crate::blueprint::generation) struct BlueprintStageRunInput<'a> {
    pub(in crate::blueprint::generation) project_id: i64,
    pub(in crate::blueprint::generation) spec: &'a BlueprintStageSpec,
    pub(in crate::blueprint::generation) publisher: &'a dyn GenerationEventPublisher,
    pub(in crate::blueprint::generation) session: &'a Arc<GenerationTaskSession>,
    pub(in crate::blueprint::generation) checkpoint_writer: &'a GenerationCheckpointWriter,
    pub(in crate::blueprint::generation) cancel_token: CancellationToken,
    pub(in crate::blueprint::generation) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::blueprint::generation) base_prompt: &'a str,
    pub(in crate::blueprint::generation) user_payload_json: &'a str,
    pub(in crate::blueprint::generation) previous_context: Option<&'a Value>,
    pub(in crate::blueprint::generation) workflow_run_id: i64,
}

pub(super) struct PreparedBlueprintStage<'a> {
    pub(super) spec: &'a BlueprintStageSpec,
    pub(super) publisher: &'a dyn GenerationEventPublisher,
    pub(super) session: &'a Arc<GenerationTaskSession>,
    pub(super) checkpoint_writer: &'a GenerationCheckpointWriter,
    pub(super) cancel_token: CancellationToken,
    pub(super) request: ChatRequest,
    pub(super) workflow_run_id: i64,
    pub(super) fingerprint_json: Value,
}

pub(in crate::blueprint::generation) struct BlueprintStageExecution {
    pub(in crate::blueprint::generation) output: Value,
    pub(in crate::blueprint::generation) request: ChatRequest,
}

pub(super) struct BlueprintStageLlmOutput {
    pub(super) response_content: String,
    pub(super) usage: Option<Value>,
    pub(super) finish_reason: Option<String>,
}
