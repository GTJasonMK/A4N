use super::super::{ChapterContentImportError, ImportChapterContentInput};
use super::persistence::upsert_import_chapter_content;
use super::preparation::prepare_import_chapter_content;
use a4n_core::models::chapter::chapter;
use chrono::Utc;
use sea_orm::ConnectionTrait;

pub async fn import_chapter_content_record<C>(
    db: &C,
    input: ImportChapterContentInput,
    target_generating_operation: &str,
) -> Result<chapter::Model, ChapterContentImportError>
where
    C: ConnectionTrait,
{
    if input.chapter_number <= 0 {
        return Err(ChapterContentImportError::InvalidArgument(
            "章节序号必须大于 0".to_string(),
        ));
    }
    if input.content.trim().is_empty() {
        return Err(ChapterContentImportError::InvalidArgument(
            "章节正文不能为空".to_string(),
        ));
    }
    let word_count = i32::try_from(input.content.chars().count()).map_err(|_| {
        ChapterContentImportError::InvalidArgument("导入章节正文字符数超出 i32 范围".to_string())
    })?;
    let target = prepare_import_chapter_content(
        db,
        input.project_id,
        input.chapter_number,
        target_generating_operation,
    )
    .await?;
    upsert_import_chapter_content(db, target, input, word_count, Utc::now().naive_utc()).await
}
