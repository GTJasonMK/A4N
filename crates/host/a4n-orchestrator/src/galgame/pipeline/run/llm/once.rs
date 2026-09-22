use a4n_error::AppError;
use serde_json::Value;

use crate::galgame::OrchestratorContext;
use a4n_prompt::rendering::PromptRenderer;

mod execution;
mod preparation;
mod restoration;

use execution::execute_galgame_llm_once;
use preparation::prepare_galgame_llm_once;
use restoration::restore_galgame_llm_once;

pub(in crate::galgame::pipeline::run) struct GalgameLlmOnceInput<'a> {
    pub(in crate::galgame::pipeline::run) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::galgame::pipeline::run) project_id: i64,
    pub(in crate::galgame::pipeline::run) root_project_id: i64,
    pub(in crate::galgame::pipeline::run) workflow_run_id: i64,
    pub(in crate::galgame::pipeline::run) workflow: &'a str,
    pub(in crate::galgame::pipeline::run) step_key: String,
    pub(in crate::galgame::pipeline::run) call_order: i32,
    pub(in crate::galgame::pipeline::run) prompt_name: &'a str,
    pub(in crate::galgame::pipeline::run) user_prompt: String,
}

struct PreparedGalgameLlmOnce {
    system_prompt: String,
    fingerprint_json: Value,
    input_fingerprint_sha256: String,
}

impl OrchestratorContext {
    pub(in crate::galgame::pipeline::run) async fn run_galgame_llm_once<T, F>(
        &self,
        input: GalgameLlmOnceInput<'_>,
        validate: F,
    ) -> Result<(Value, T), AppError>
    where
        T: serde::Serialize,
        F: Fn(Value) -> Result<T, AppError>,
    {
        let prepared = prepare_galgame_llm_once(&input).await?;
        self.mark_workflow_current_step(input.workflow_run_id, &input.step_key)
            .await?;
        if let Some(restored) = restore_galgame_llm_once(self, &input, &prepared, &validate).await?
        {
            return Ok(restored);
        }
        execute_galgame_llm_once(self, &input, prepared, validate).await
    }
}
