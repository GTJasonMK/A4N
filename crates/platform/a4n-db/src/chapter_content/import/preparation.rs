use super::super::ChapterContentImportError;
use super::ImportChapterContentTarget;
use a4n_db_models::models::chapter::chapter;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub(super) async fn prepare_import_chapter_content<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
    target_generating_operation: &str,
) -> Result<ImportChapterContentTarget, ChapterContentImportError>
where
    C: ConnectionTrait,
{
    let existing = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await
        .map_err(ChapterContentImportError::database)?;

    if let Some(existing) = existing.as_ref()
        && existing.is_generating()
    {
        return Err(ChapterContentImportError::TargetChapterGenerating {
            chapter_number: existing.chapter_number,
            status: existing.status.clone(),
            operation: target_generating_operation.to_string(),
        });
    }

    Ok(ImportChapterContentTarget { existing })
}
