use a4n_core::error::AppError;
use a4n_db::workflow::mark_workflow_current_step;
use serde_json::Value;

use crate::galgame::OrchestratorContext;
use crate::workflow::workflow_write_app_error;
use a4n_prompt::rendering::PromptRenderer;

mod execution;
mod preparation;
mod restoration;

use execution::execute_galgame_llm_once;
use preparation::prepare_galgame_llm_once;
use restoration::restore_galgame_llm_once;

pub(in crate::galgame::pipeline_run) struct GalgameLlmOnceInput<'a> {
    pub(in crate::galgame::pipeline_run) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::galgame::pipeline_run) project_id: i64,
    pub(in crate::galgame::pipeline_run) root_project_id: i64,
    pub(in crate::galgame::pipeline_run) workflow_run_id: i64,
    pub(in crate::galgame::pipeline_run) workflow: &'a str,
    pub(in crate::galgame::pipeline_run) step_key: String,
    pub(in crate::galgame::pipeline_run) call_order: i32,
    pub(in crate::galgame::pipeline_run) prompt_name: &'a str,
    pub(in crate::galgame::pipeline_run) user_prompt: String,
}

struct PreparedGalgameLlmOnce {
    system_prompt: String,
    fingerprint_json: Value,
    input_fingerprint_sha256: String,
}

impl OrchestratorContext {
    pub(in crate::galgame::pipeline_run) async fn run_galgame_llm_once<F>(
        &self,
        input: GalgameLlmOnceInput<'_>,
        validate: F,
    ) -> Result<Value, AppError>
    where
        F: Fn(Value) -> Result<Value, AppError>,
    {
        let prepared = prepare_galgame_llm_once(&input).await?;
        mark_workflow_current_step(
            self.database.connection(),
            input.workflow_run_id,
            &input.step_key,
        )
        .await
        .map_err(workflow_write_app_error)?;
        if let Some(restored) = restore_galgame_llm_once(self, &input, &prepared, &validate).await?
        {
            return Ok(restored);
        }
        execute_galgame_llm_once(self, &input, prepared, validate).await
    }
}
