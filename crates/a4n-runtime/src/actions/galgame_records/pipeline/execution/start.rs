use super::super::contracts::StartGalgamePipelineInput;
use crate::{AppState, prompt_renderer::RuntimePromptRenderer};
use a4n_orchestrator::galgame::{
    GalgamePipelineApplicationInput, GalgameProgressReporter, GalgameScriptDetail,
};

pub async fn start_galgame_pipeline(
    state: &AppState,
    user_id: i64,
    input: StartGalgamePipelineInput,
) -> Result<GalgameScriptDetail, String> {
    start_galgame_pipeline_with_progress(state, user_id, input, None).await
}

pub async fn start_galgame_pipeline_with_progress(
    state: &AppState,
    user_id: i64,
    input: StartGalgamePipelineInput,
    progress_reporter: Option<GalgameProgressReporter>,
) -> Result<GalgameScriptDetail, String> {
    let prompt_renderer = RuntimePromptRenderer::new(state);
    state
        .galgame()
        .run_galgame_pipeline_for_user_validated(
            user_id,
            GalgamePipelineApplicationInput {
                project_id: input.project_id,
                task_id: input.task_id,
                start_from_stage: input.start_from_stage,
                resume_from_checkpoint: input.resume_from_checkpoint,
                progress_reporter,
            },
            &prompt_renderer,
        )
        .await
        .map_err(|error| error.to_string())
}
