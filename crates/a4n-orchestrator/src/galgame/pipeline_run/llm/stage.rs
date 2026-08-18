use a4n_core::error::AppError;
use a4n_llm::provider::ChatMessage;
use serde_json::Value;

use crate::galgame::{GalgamePipelineStartStage, OrchestratorContext};
use a4n_prompt::rendering::PromptRenderer;

use super::super::progress::ensure_galgame_pipeline_not_cancelled;

mod execution;
mod preparation;
mod restoration;

use execution::execute_galgame_llm_stage;
use preparation::prepare_galgame_llm_stage;
use restoration::restore_galgame_llm_stage;

pub(in crate::galgame::pipeline_run) struct GalgameLlmStageInput<'a> {
    pub(in crate::galgame::pipeline_run) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::galgame::pipeline_run) project_id: i64,
    pub(in crate::galgame::pipeline_run) root_project_id: i64,
    pub(in crate::galgame::pipeline_run) workflow_run_id: i64,
    pub(in crate::galgame::pipeline_run) stage: GalgamePipelineStartStage,
    pub(in crate::galgame::pipeline_run) checkpoint_key: &'a str,
    pub(in crate::galgame::pipeline_run) prompt_name: &'a str,
    pub(in crate::galgame::pipeline_run) user_prompt: String,
    pub(in crate::galgame::pipeline_run) request_user_prompt: String,
    pub(in crate::galgame::pipeline_run) shared_shooting_script_projection: &'a Value,
    pub(in crate::galgame::pipeline_run) restore_from_checkpoint: bool,
    pub(in crate::galgame::pipeline_run) restore_required: bool,
}

struct PreparedGalgameLlmStage {
    request_messages: Vec<ChatMessage>,
    fingerprint_json: Value,
    input_fingerprint_sha256: String,
}

impl OrchestratorContext {
    pub(in crate::galgame::pipeline_run) async fn run_galgame_llm_stage<F>(
        &self,
        input: GalgameLlmStageInput<'_>,
        validate: F,
    ) -> Result<Value, AppError>
    where
        F: Fn(Value) -> Result<Value, AppError>,
    {
        ensure_galgame_pipeline_not_cancelled(self, input.workflow_run_id).await?;
        self.mark_workflow_current_step(input.workflow_run_id, input.stage.key())
            .await?;

        let prepared = prepare_galgame_llm_stage(&input).await?;
        if let Some(restored) =
            restore_galgame_llm_stage(self, &input, &prepared, &validate).await?
        {
            return Ok(restored);
        }
        execute_galgame_llm_stage(self, &input, prepared, validate).await
    }
}
