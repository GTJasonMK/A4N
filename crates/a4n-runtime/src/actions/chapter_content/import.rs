use super::{ChapterSummary, ImportChapterInput, projections};
use crate::AppState;
use a4n_core::error::AppError;

pub async fn import_chapter(
    state: &AppState,
    user_id: i64,
    input: ImportChapterInput,
) -> Result<ChapterSummary, AppError> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    let chapter = state
        .orchestrator()
        .import_chapter_validated(
            user_id,
            &host_port,
            input.project_id,
            input.chapter_number,
            input.content,
            input.title,
        )
        .await?;
    projections::chapter_summary_with_sync_state(state, &chapter).await
}
