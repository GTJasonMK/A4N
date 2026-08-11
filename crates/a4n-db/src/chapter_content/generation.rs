use a4n_core::models::chapter::chapter::{self, STATUS_DRAFT};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use super::{
    ChapterGenerationExistingRecord, ChapterGenerationLifecycleError,
    GeneratedChapterContentRecordInput,
};
use crate::generation::delete_generation_run_targets_for;

pub async fn begin_chapter_generation_record<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
    now: NaiveDateTime,
) -> Result<Option<ChapterGenerationExistingRecord>, ChapterGenerationLifecycleError>
where
    C: ConnectionTrait,
{
    let Some(chapter) = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await
        .map_err(ChapterGenerationLifecycleError::database)?
    else {
        return Ok(None);
    };

    if chapter.is_generating() {
        return Err(ChapterGenerationLifecycleError::InvalidState {
            current: chapter.status,
            operation: format!("生成第{chapter_number}章"),
        });
    }

    let previous_status = chapter.status.clone();
    let mut active: chapter::ActiveModel = chapter.into();
    active.status = Set(chapter::STATUS_GENERATING.to_string());
    active.updated_at = Set(now);
    let chapter = active
        .update(db)
        .await
        .map_err(ChapterGenerationLifecycleError::database)?;
    Ok(Some(ChapterGenerationExistingRecord {
        id: chapter.id,
        previous_status,
        title: chapter.title,
        summary: chapter.summary,
    }))
}

pub async fn restore_chapter_generation_status_record<C>(
    db: &C,
    chapter_id: i64,
    previous_status: String,
    now: NaiveDateTime,
) -> Result<bool, ChapterGenerationLifecycleError>
where
    C: ConnectionTrait,
{
    let Some(current) = chapter::Entity::find_by_id(chapter_id)
        .one(db)
        .await
        .map_err(ChapterGenerationLifecycleError::database)?
    else {
        return Ok(false);
    };

    if !current.is_generating() {
        return Ok(false);
    }

    let mut active: chapter::ActiveModel = current.into();
    active.status = Set(previous_status);
    active.updated_at = Set(now);
    active
        .update(db)
        .await
        .map_err(ChapterGenerationLifecycleError::database)?;
    Ok(true)
}

/// Writes the canonical chapter result into an already-open transaction.
///
/// Generation terminal completion uses this entry so the chapter row and the
/// generation terminal/checkpoint commit share one database transaction.
pub async fn finish_generated_chapter_content_record_in_transaction<C>(
    db: &C,
    input: GeneratedChapterContentRecordInput,
) -> Result<chapter::Model, ChapterGenerationLifecycleError>
where
    C: ConnectionTrait,
{
    let outline_id = match input.source_outline.as_ref() {
        Some(source) => {
            if source.project_id != input.project_id
                || source.chapter_number != input.chapter_number
            {
                return Err(ChapterGenerationLifecycleError::InvalidArgument(
                    "正文生成来源章纲与目标章节不一致".to_string(),
                ));
            }
            crate::project::formal_chapter_outline_id_if_unchanged(db, source)
                .await
                .map_err(ChapterGenerationLifecycleError::database)?
        }
        None => None,
    };
    let updated_chapter = match input.existing_chapter_id {
        Some(chapter_id) => {
            let current = chapter::Entity::find_by_id(chapter_id)
                .one(db)
                .await
                .map_err(ChapterGenerationLifecycleError::database)?
                .ok_or_else(|| {
                    ChapterGenerationLifecycleError::NotFound(format!("章节不存在: {chapter_id}"))
                })?;
            if !current.is_generating() {
                return Err(ChapterGenerationLifecycleError::InvalidState {
                    current: current.status,
                    operation: "完成章节生成".to_string(),
                });
            }

            let mut active: chapter::ActiveModel = current.into();
            active.content = Set(input.content.clone());
            active.word_count = Set(input.word_count);
            active.title = Set(Some(input.title.clone()));
            active.summary = Set(Some(input.summary.clone()));
            active.outline_id = Set(outline_id);
            active.style_tags = Set(None);
            active.analysis_data = Set(None);
            active.status = Set(STATUS_DRAFT.to_string());
            active.updated_at = Set(input.now);
            active
                .update(db)
                .await
                .map_err(ChapterGenerationLifecycleError::database)?
        }
        None => chapter::ActiveModel {
            project_id: Set(input.project_id),
            chapter_number: Set(input.chapter_number),
            title: Set(Some(input.title.clone())),
            content: Set(input.content.clone()),
            word_count: Set(input.word_count),
            status: Set(STATUS_DRAFT.to_string()),
            summary: Set(Some(input.summary.clone())),
            outline_id: Set(outline_id),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(input.now),
            updated_at: Set(input.now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ChapterGenerationLifecycleError::database)?,
    };

    delete_generation_run_targets_for(db, "chapter_content", &[updated_chapter.id]).await?;
    delete_generation_run_targets_for(db, "chapter_analysis", &[updated_chapter.id]).await?;
    Ok(updated_chapter)
}
