use a4n_error::AppError;

use crate::galgame::{GalgamePipelineRunOptions, GalgameScriptDetail, OrchestratorContext};
use a4n_prompt::rendering::PromptRenderer;

use super::contracts::GalgamePipelineExecution;
use super::early_stages::run_galgame_pipeline_early_stages;
use super::finalization::{persist_galgame_pipeline_script, plan_galgame_pipeline_assets};
use super::late_stages::run_galgame_pipeline_late_stages;
use super::preparation::prepare_galgame_pipeline;

impl OrchestratorContext {
    pub(crate) async fn run_pipeline(
        &self,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        options: GalgamePipelineRunOptions,
    ) -> Result<GalgameScriptDetail, AppError> {
        let prompt_context = prepare_galgame_pipeline(self, project_id, &options).await?;
        let execution = GalgamePipelineExecution {
            service: self,
            project_id,
            prompt_renderer,
            options: &options,
            prompt_context: &prompt_context,
        };
        let early = run_galgame_pipeline_early_stages(&execution).await?;
        let late = run_galgame_pipeline_late_stages(&execution, &early).await?;
        let detail = persist_galgame_pipeline_script(&execution, &early, &late).await?;
        plan_galgame_pipeline_assets(&execution, &detail).await?;
        self.get_script_detail(project_id).await
    }
}
