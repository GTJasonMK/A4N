use super::ChapterSummary;
use crate::AppState;
use a4n_error::AppError;

pub async fn list_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ChapterSummary>, AppError> {
    state
        .chapter_content_read_application()
        .list(user_id, project_id)
        .await
}
