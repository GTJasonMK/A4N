use super::super::{ChapterContentImportError, ImportChapterContentInput};
use super::persistence::upsert_import_chapter_content;
use super::preparation::prepare_import_chapter_content;
use a4n_core::models::chapter::chapter;
use sea_orm::ConnectionTrait;

pub async fn import_chapter_content_record<C>(
    db: &C,
    input: ImportChapterContentInput,
    target_generating_operation: &str,
) -> Result<chapter::Model, ChapterContentImportError>
where
    C: ConnectionTrait,
{
    let target = prepare_import_chapter_content(
        db,
        input.project_id,
        input.chapter_number,
        target_generating_operation,
    )
    .await?;
    upsert_import_chapter_content(db, target, input).await
}
