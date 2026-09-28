use crate::AppState;
use a4n_error::AppError;

pub async fn run_manga_pipeline(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    task_id: &str,
    resume_from_checkpoint: bool,
    stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
) -> Result<(), AppError> {
    let prompt_renderer = state;
    state
        .manga_pipeline_execution_application()
        .run(
            user_id,
            project_id,
            chapter_number,
            task_id.to_string(),
            resume_from_checkpoint,
            stop_after,
            prompt_renderer,
        )
        .await
}

pub async fn preflight_manga_pipeline(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
) -> Result<(), AppError> {
    state
        .manga_pipeline_execution_application()
        .preflight(user_id, project_id, chapter_number)
        .await
}
