use crate::AppState;
use a4n_error::AppError;

pub async fn preflight_manga_pipeline(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    page_number: Option<u32>,
    stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
) -> Result<(), AppError> {
    state
        .manga_pipeline_execution_application()
        .preflight(
            user_id,
            project_id,
            chapter_number,
            page_number,
            stop_after,
            state,
        )
        .await
}
