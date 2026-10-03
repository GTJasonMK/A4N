use a4n_error::AppError;

use crate::galgame::pipeline::prompt::galgame_source_markers;
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
    ) -> Result<Option<GalgameScriptDetail>, AppError> {
        let prompt_context =
            prepare_galgame_pipeline(self, project_id, &options, prompt_renderer).await?;
        let source_markers = galgame_source_markers(&prompt_context.chapters)?;
        let execution = GalgamePipelineExecution {
            service: self,
            project_id,
            prompt_renderer,
            options: &options,
            prompt_context: &prompt_context,
            source_markers: &source_markers,
        };
        let Some(early) = run_galgame_pipeline_early_stages(&execution).await? else {
            return Ok(None);
        };
        let late = run_galgame_pipeline_late_stages(&execution, &early).await?;
        let detail = persist_galgame_pipeline_script(&execution, &early, &late).await?;
        if options.stop_after != Some(a4n_api_contract::galgame::GalgamePipelineStopAfter::Script) {
            plan_galgame_pipeline_assets(&execution, &detail).await?;
        }
        self.get_script_detail(project_id).await.map(Some)
    }
}
