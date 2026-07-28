use crate::AppState;
use crate::actions::access;
use crate::prompt_renderer::RuntimePromptRenderer;
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;
use a4n_orchestrator::manga::{MangaPipelineApplicationInput, MangaPipelineStartStage};

pub async fn run_manga_pipeline(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    task_id: &str,
    resume_from_checkpoint: bool,
    start_from_stage: Option<MangaPipelineStartStage>,
) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
    access::verify_project_or_derived_family_editor(state, user_id, project_id).await?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    state
        .manga()
        .run_manga_pipeline_validated(MangaPipelineApplicationInput {
            project_id,
            chapter_number,
            task_id,
            resume_from_checkpoint,
            start_from_stage,
            prompt_renderer: &prompt_renderer,
        })
        .await
}

pub async fn preflight_manga_pipeline(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
) -> Result<(), AppError> {
    access::verify_project_or_derived_family_editor(state, user_id, project_id).await?;
    state
        .manga()
        .preflight_manga_pipeline_validated(project_id, chapter_number)
        .await
}
