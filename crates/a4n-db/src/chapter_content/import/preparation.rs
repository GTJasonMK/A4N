use super::super::{ChapterContentImportError, ImportChapterContentTarget};
use a4n_core::models::chapter::chapter;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub async fn prepare_import_chapter_content<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
    target_generating_operation: &str,
) -> Result<ImportChapterContentTarget, ChapterContentImportError>
where
    C: ConnectionTrait,
{
    ensure_previous_chapter_content_ready(db, project_id, chapter_number).await?;
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

async fn ensure_previous_chapter_content_ready<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<(), ChapterContentImportError>
where
    C: ConnectionTrait,
{
    if chapter_number <= 1 {
        return Ok(());
    }

    let previous_number = chapter_number - 1;
    let previous = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(previous_number))
        .one(db)
        .await
        .map_err(ChapterContentImportError::database)?;
    let Some(previous) = previous else {
        return Err(ChapterContentImportError::PreviousChapterMissing {
            previous_number,
            chapter_number,
        });
    };
    if previous.is_generating() {
        return Err(ChapterContentImportError::PreviousChapterGenerating {
            previous_number,
            chapter_number,
            status: previous.status,
        });
    }
    if !previous.has_available_content() {
        return Err(ChapterContentImportError::PreviousChapterNoContent {
            previous_number,
            chapter_number,
        });
    }
    Ok(())
}
