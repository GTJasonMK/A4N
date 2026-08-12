use super::super::{ChapterContentImportError, ImportChapterContentInput, assign_chapter_content};
use super::ImportChapterContentTarget;
use a4n_core::models::chapter::chapter::{self, STATUS_IMPORTED};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

pub(super) async fn upsert_import_chapter_content<C>(
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
        assign_chapter_content(
            &mut active,
            input.content.clone(),
            input.word_count,
            STATUS_IMPORTED,
            summary,
            input.now,
        );
        return active
            .update(db)
            .await
            .map_err(ChapterContentImportError::database);
    }

    let mut active = chapter::ActiveModel {
        project_id: Set(input.project_id),
        chapter_number: Set(input.chapter_number),
        title: Set(input.normalized_title.clone()),
        outline_id: Set(None),
        created_at: Set(input.now),
        ..Default::default()
    };
    assign_chapter_content(
        &mut active,
        input.content.clone(),
        input.word_count,
        STATUS_IMPORTED,
        summary,
        input.now,
    );
    active
        .insert(db)
        .await
        .map_err(ChapterContentImportError::database)
}
