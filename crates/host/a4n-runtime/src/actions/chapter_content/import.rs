use super::{ChapterSummary, ImportChapterInput};
use crate::AppState;
use a4n_error::AppError;

pub async fn import_chapter(
    state: &AppState,
    user_id: i64,
    input: ImportChapterInput,
) -> Result<ChapterSummary, AppError> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    let chapter_id = state
        .chapter_content_mutation_application()
        .import(user_id, &host_port, input)
        .await?;
    state
        .chapter_content_read_application()
        .summary(user_id, chapter_id, None)
        .await
}
