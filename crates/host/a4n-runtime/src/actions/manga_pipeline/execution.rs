use crate::AppState;
use a4n_error::AppError;

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
