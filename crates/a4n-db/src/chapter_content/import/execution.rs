use super::super::{ChapterContentImportError, ImportChapterContentInput};
use super::persistence::upsert_import_chapter_content;
use super::preparation::prepare_import_chapter_content;
use a4n_core::models::chapter::chapter;
use sea_orm::{DatabaseConnection, TransactionTrait};

pub async fn import_chapter_content_record(
    db: &DatabaseConnection,
    input: ImportChapterContentInput,
    target_generating_operation: &str,
) -> Result<chapter::Model, ChapterContentImportError> {
    let tx = db
        .begin()
        .await
        .map_err(ChapterContentImportError::database)?;
    let target = prepare_import_chapter_content(
        &tx,
        input.project_id,
        input.chapter_number,
        target_generating_operation,
    )
    .await?;
    let chapter = upsert_import_chapter_content(&tx, target, input).await?;
    tx.commit()
        .await
        .map_err(ChapterContentImportError::database)?;
    Ok(chapter)
}
