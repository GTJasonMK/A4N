use super::super::{
    ChapterContentImportError, ImportChapterContentInput, ImportChapterContentTarget,
};
use a4n_core::models::chapter::chapter::{self, STATUS_IMPORTED};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

pub async fn upsert_import_chapter_content<C>(
    db: &C,
    target: ImportChapterContentTarget,
    input: ImportChapterContentInput,
) -> Result<chapter::Model, ChapterContentImportError>
where
    C: ConnectionTrait,
{
    let mut chapter = upsert_chapter(db, target.existing, &input).await?;
    chapter.outline_id = crate::project::link_chapter_to_project_outline_if_needed(
        db,
        input.project_id,
        input.chapter_number,
    )
    .await
    .map_err(ChapterContentImportError::database)?;
    Ok(chapter)
}

async fn upsert_chapter<C>(
    db: &C,
    existing: Option<chapter::Model>,
    input: &ImportChapterContentInput,
) -> Result<chapter::Model, ChapterContentImportError>
where
    C: ConnectionTrait,
{
    let summary = Some(format!(
        "第{}章：{}字",
        input.chapter_number, input.word_count
    ));
    if let Some(existing) = existing {
        let mut active: chapter::ActiveModel = existing.into();
        if let Some(title) = input.normalized_title.clone() {
            active.title = Set(Some(title));
        }
        active.content = Set(input.content.clone());
        active.word_count = Set(input.word_count);
        active.status = Set(STATUS_IMPORTED.to_string());
        active.summary = Set(summary);
        active.style_tags = Set(None);
        active.analysis_data = Set(None);
        active.updated_at = Set(input.now);
        return active
            .update(db)
            .await
            .map_err(ChapterContentImportError::database);
    }

    chapter::ActiveModel {
        project_id: Set(input.project_id),
        chapter_number: Set(input.chapter_number),
        title: Set(input.normalized_title.clone()),
        content: Set(input.content.clone()),
        word_count: Set(input.word_count),
        status: Set(STATUS_IMPORTED.to_string()),
        summary: Set(summary),
        style_tags: Set(None),
        analysis_data: Set(None),
        outline_id: Set(None),
        created_at: Set(input.now),
        updated_at: Set(input.now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(ChapterContentImportError::database)
}
