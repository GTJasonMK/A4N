use a4n_db_models::models::chapter::chapter::{self, STATUS_DRAFT};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, Set,
};

use super::{
    ChapterGenerationExistingRecord, ChapterGenerationLifecycleError,
    GeneratedChapterContentRecordInput, assign_chapter_content,
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
    let Some((chapter_id, status, title, summary)) = chapter::Entity::find_current()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number))
        .select_only()
        .column(chapter::Column::Id)
        .column(chapter::Column::Status)
        .column(chapter::Column::Title)
        .column(chapter::Column::Summary)
        .into_tuple::<(i64, String, Option<String>, Option<String>)>()
        .one(db)
        .await
        .map_err(ChapterGenerationLifecycleError::database)?
    else {
        return Ok(None);
    };

    if status == chapter::STATUS_GENERATING {
        return Err(ChapterGenerationLifecycleError::InvalidState {
            current: status,
            operation: format!("生成第{chapter_number}章"),
        });
    }
    let previous_status = status;

    chapter::ActiveModel {
        id: Set(chapter_id),
        status: Set(chapter::STATUS_GENERATING.to_string()),
        updated_at: Set(now),
        ..Default::default()
    }
    .update(db)
    .await
    .map_err(ChapterGenerationLifecycleError::database)?;
    Ok(Some(ChapterGenerationExistingRecord {
        id: chapter_id,
        previous_status,
        title,
        summary,
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
    let Some(status) = chapter::Entity::find_by_id(chapter_id)
        .select_only()
        .column(chapter::Column::Status)
        .into_tuple::<String>()
        .one(db)
        .await
        .map_err(ChapterGenerationLifecycleError::database)?
    else {
        return Ok(false);
    };

    if status != chapter::STATUS_GENERATING {
        return Ok(false);
    }

    chapter::ActiveModel {
        id: Set(chapter_id),
        status: Set(previous_status),
        updated_at: Set(now),
        ..Default::default()
    }
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
            let status = chapter::Entity::find_by_id(chapter_id)
                .select_only()
                .column(chapter::Column::Status)
                .into_tuple::<String>()
                .one(db)
                .await
                .map_err(ChapterGenerationLifecycleError::database)?
                .ok_or_else(|| {
                    ChapterGenerationLifecycleError::NotFound(format!("章节不存在: {chapter_id}"))
                })?;
            if status != chapter::STATUS_GENERATING {
                return Err(ChapterGenerationLifecycleError::InvalidState {
                    current: status,
                    operation: "完成章节生成".to_string(),
                });
            }

            let mut active = chapter::ActiveModel {
                id: Set(chapter_id),
                ..Default::default()
            };
            assign_chapter_content(
                &mut active,
                input.content,
                input.word_count,
                STATUS_DRAFT,
                Some(input.summary),
                input.now,
            );
            active.title = Set(Some(input.title));
            active.outline_id = Set(outline_id);
            active
                .update(db)
                .await
                .map_err(ChapterGenerationLifecycleError::database)?
        }
        None => {
            let mut active = chapter::ActiveModel {
                project_id: Set(input.project_id),
                chapter_number: Set(input.chapter_number),
                title: Set(Some(input.title)),
                outline_id: Set(outline_id),
                created_at: Set(input.now),
                ..Default::default()
            };
            assign_chapter_content(
                &mut active,
                input.content,
                input.word_count,
                STATUS_DRAFT,
                Some(input.summary),
                input.now,
            );
            active
                .insert(db)
                .await
                .map_err(ChapterGenerationLifecycleError::database)?
        }
    };

    delete_generation_run_targets_for(db, "chapter_content", &[updated_chapter.id]).await?;
    delete_generation_run_targets_for(db, "chapter_analysis", &[updated_chapter.id]).await?;
    Ok(updated_chapter)
}
