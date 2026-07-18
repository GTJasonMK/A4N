use a4n_core::models::chapter::chapter;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use super::queries::chapter_for_post_process;
use super::{ChapterContentEditError, StoreChapterAnalysisRecordInput};

pub async fn store_chapter_analysis_record<C>(
    db: &C,
    chapter: chapter::Model,
    input: StoreChapterAnalysisRecordInput,
) -> Result<chapter::Model, ChapterContentEditError>
where
    C: ConnectionTrait,
{
    let mut active: chapter::ActiveModel = chapter.into();
    active.analysis_data = Set(Some(input.analysis_data));
    if let Some(summary) = input.summary {
        active.summary = Set(Some(summary));
    }
    if let Some(style_tags) = input.style_tags {
        active.style_tags = Set(Some(style_tags));
    }
    active.updated_at = Set(input.now);
    active
        .update(db)
        .await
        .map_err(ChapterContentEditError::database)
}

pub async fn update_chapter_style_tags_record<C>(
    db: &C,
    chapter_id: i64,
    tags: &[String],
    now: NaiveDateTime,
) -> Result<(), ChapterContentEditError>
where
    C: ConnectionTrait,
{
    let chapter = chapter_for_post_process(db, chapter_id).await?;
    let mut active: chapter::ActiveModel = chapter.into();
    active.style_tags = Set(Some(serde_json::json!(tags)));
    active.updated_at = Set(now);
    active
        .update(db)
        .await
        .map_err(ChapterContentEditError::database)?;
    Ok(())
}
