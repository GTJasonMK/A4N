use super::super::{ChapterContentImportError, ImportChapterContentInput, assign_chapter_content};
use super::ImportChapterContentTarget;
use a4n_core::models::chapter::chapter::{self, STATUS_IMPORTED};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

pub(super) async fn upsert_import_chapter_content<C>(
    db: &C,
    target: ImportChapterContentTarget,
    input: ImportChapterContentInput,
    word_count: i32,
    now: NaiveDateTime,
) -> Result<chapter::Model, ChapterContentImportError>
where
    C: ConnectionTrait,
{
    let project_id = input.project_id;
    let chapter_number = input.chapter_number;
    let mut chapter = upsert_chapter(db, target.existing, input, word_count, now).await?;
    chapter.outline_id =
        crate::project::link_chapter_to_project_outline_if_needed(db, project_id, chapter_number)
            .await
            .map_err(ChapterContentImportError::database)?;
    Ok(chapter)
}

async fn upsert_chapter<C>(
    db: &C,
    existing: Option<chapter::Model>,
    input: ImportChapterContentInput,
    word_count: i32,
    now: NaiveDateTime,
) -> Result<chapter::Model, ChapterContentImportError>
where
    C: ConnectionTrait,
{
    let normalized_title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let summary = Some(format!("第{}章：{}字", input.chapter_number, word_count));
    if let Some(existing) = existing {
        let mut active: chapter::ActiveModel = existing.into();
        if let Some(title) = normalized_title {
            active.title = Set(Some(title));
        }
        assign_chapter_content(
            &mut active,
            input.content,
            word_count,
            STATUS_IMPORTED,
            summary,
            now,
        );
        return active
            .update(db)
            .await
            .map_err(ChapterContentImportError::database);
    }

    let mut active = chapter::ActiveModel {
        project_id: Set(input.project_id),
        chapter_number: Set(input.chapter_number),
        title: Set(normalized_title),
        outline_id: Set(None),
        created_at: Set(now),
        ..Default::default()
    };
    assign_chapter_content(
        &mut active,
        input.content,
        word_count,
        STATUS_IMPORTED,
        summary,
        now,
    );
    active
        .insert(db)
        .await
        .map_err(ChapterContentImportError::database)
}
