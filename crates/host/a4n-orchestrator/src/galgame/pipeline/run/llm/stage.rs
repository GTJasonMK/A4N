use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatMessage;
use serde_json::Value;

use crate::galgame::{GalgamePipelineStartStage, OrchestratorContext};
use crate::source_markers::SourceMarkerMap;
use a4n_prompt::rendering::PromptRenderer;

use super::super::progress::ensure_galgame_pipeline_not_cancelled;

mod execution;
mod preparation;
mod restoration;

use execution::execute_galgame_llm_stage;
use preparation::prepare_galgame_llm_stage;
use restoration::restore_galgame_llm_stage;

pub(in crate::galgame::pipeline::run) struct GalgameLlmStageInput<'a> {
    pub(in crate::galgame::pipeline::run) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::galgame::pipeline::run) project_id: i64,
    pub(in crate::galgame::pipeline::run) root_project_id: i64,
    pub(in crate::galgame::pipeline::run) workflow_run_id: i64,
    pub(in crate::galgame::pipeline::run) stage: GalgamePipelineStartStage,
    pub(in crate::galgame::pipeline::run) checkpoint_key: &'a str,
    pub(in crate::galgame::pipeline::run) prompt_name: &'a str,
    pub(in crate::galgame::pipeline::run) user_prompt: String,
    pub(in crate::galgame::pipeline::run) request_user_prompt: String,
    pub(in crate::galgame::pipeline::run) shared_adaptation_projection: &'a Value,
    pub(in crate::galgame::pipeline::run) source_markers: &'a SourceMarkerMap,
    pub(in crate::galgame::pipeline::run) restore_from_checkpoint: bool,
    pub(in crate::galgame::pipeline::run) restore_required: bool,
}

struct PreparedGalgameLlmStage {
    request_messages: Vec<ChatMessage>,
    fingerprint_json: Value,
    input_fingerprint_sha256: String,
}

impl OrchestratorContext {
    pub(in crate::galgame::pipeline::run) async fn run_galgame_llm_stage<T, F>(
        &self,
        input: GalgameLlmStageInput<'_>,
        validate_canonical: F,
    ) -> Result<(Value, T), AppError>
    where
        T: serde::Serialize,
        F: Fn(Value) -> Result<T, AppError>,
    {
        ensure_galgame_pipeline_not_cancelled(self, input.workflow_run_id).await?;
        self.mark_workflow_current_step(input.workflow_run_id, input.stage.key())
            .await?;

        let validate_marked =
            |value| validate_canonical(input.source_markers.decode_output(&value)?);
        let prepared = prepare_galgame_llm_stage(&input).await?;
        if let Some(restored) =
            restore_galgame_llm_stage(self, &input, &prepared, &validate_marked).await?
        {
            return Ok(restored);
        }
        execute_galgame_llm_stage(self, &input, prepared, validate_marked).await
    }
}
