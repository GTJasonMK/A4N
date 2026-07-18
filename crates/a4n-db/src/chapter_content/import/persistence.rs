use super::super::{
    ChapterContentImportError, ImportChapterContentInput, ImportChapterContentTarget,
};
use a4n_core::models::chapter::{
    chapter::{self, STATUS_IMPORTED},
    chapter_outline,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub async fn upsert_import_chapter_content<C>(
    db: &C,
    target: ImportChapterContentTarget,
    input: ImportChapterContentInput,
) -> Result<chapter::Model, ChapterContentImportError>
where
    C: ConnectionTrait,
{
    let chapter = upsert_chapter(db, target.existing, &input).await?;
    upsert_import_outline(db, &input).await?;
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

async fn upsert_import_outline<C>(
    db: &C,
    input: &ImportChapterContentInput,
) -> Result<(), ChapterContentImportError>
where
    C: ConnectionTrait,
{
    let outline_title = input
        .normalized_title
        .clone()
        .or_else(|| Some(format!("第{}章", input.chapter_number)));
    let outline_summary = format!("导入章节：{}字，摘要待生成", input.word_count);
    let existing_outline = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(input.project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(input.chapter_number))
        .one(db)
        .await
        .map_err(ChapterContentImportError::database)?;

    if let Some(outline) = existing_outline {
        let mut active: chapter_outline::ActiveModel = outline.into();
        active.title = Set(outline_title);
        active.summary = Set(Some(outline_summary));
        active.word_count_target = Set(Some(input.word_count));
        active.updated_at = Set(input.now);
        active
            .update(db)
            .await
            .map_err(ChapterContentImportError::database)?;
    } else {
        chapter_outline::ActiveModel {
            project_id: Set(input.project_id),
            part_id: Set(None),
            chapter_number: Set(input.chapter_number),
            title: Set(outline_title),
            summary: Set(Some(outline_summary)),
            beats: Set(None),
            key_scenes: Set(None),
            pacing_score: Set(None),
            foreshadowing_setup: Set(None),
            foreshadowing_payoff: Set(None),
            characters_involved: Set(None),
            word_count_target: Set(Some(input.word_count)),
            status: Set("draft".to_string()),
            created_at: Set(input.now),
            updated_at: Set(input.now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ChapterContentImportError::database)?;
    }
    Ok(())
}
