use super::{ImportChapterInput, ImportedChapterSummary};
use crate::AppState;
use a4n_core::{error::AppError, models::chapter::chapter::STATUS_IMPORTED};

pub async fn import_chapter(
    state: &AppState,
    user_id: i64,
    input: ImportChapterInput,
) -> Result<ImportedChapterSummary, AppError> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    let (chapter, word_count) = state
        .chapter_content()
        .import_chapter_validated(
            user_id,
            &host_port,
            input.project_id,
            input.chapter_number,
            input.content,
            input.title,
        )
        .await?;
    Ok(ImportedChapterSummary {
        id: chapter.id,
        project_id: chapter.project_id,
        chapter_number: chapter.chapter_number,
        title: chapter.title,
        status: STATUS_IMPORTED.to_string(),
        word_count,
    })
}
