use super::{ChapterAnalysisEntry, ChapterIndexEntry, ChapterStoryEventEntry, ChapterSummary};
use crate::AppState;
use a4n_error::AppError;

pub async fn list_chapter_analyses(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ChapterAnalysisEntry>, AppError> {
    state
        .chapter_content_read_application()
        .analyses(user_id, project_id)
        .await
}

pub async fn list_story_event_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ChapterStoryEventEntry>, AppError> {
    state
        .chapter_content_read_application()
        .story_events(user_id, project_id)
        .await
}

pub async fn list_chapter_index(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ChapterIndexEntry>, AppError> {
    state
        .chapter_content_read_application()
        .index(user_id, project_id)
        .await
}

pub async fn get_chapter(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<ChapterSummary, AppError> {
    state
        .chapter_content_read_application()
        .summary(user_id, chapter_id, Some(project_id))
        .await
}

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
