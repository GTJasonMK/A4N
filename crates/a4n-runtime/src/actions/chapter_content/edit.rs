use super::{ChapterSummary, UpdateChapterContentInput, projections};
use crate::AppState;
use a4n_core::error::AppError;

pub async fn update_chapter_content(
    state: &AppState,
    user_id: i64,
    input: UpdateChapterContentInput,
) -> Result<ChapterSummary, AppError> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    let updated = state
        .chapter_content()
        .update_chapter_content_validated(
            user_id,
            &host_port,
            input.chapter_id,
            input.content,
            input.title,
        )
        .await?;
    Ok(projections::chapter_summary(&updated))
}
