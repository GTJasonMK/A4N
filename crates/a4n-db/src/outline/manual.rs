use a4n_core::models::chapter::{chapter_outline, part_outline};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};

use super::chapters::{clear_chapter_outline_links, sync_chapter_outline_part_assignments};
use super::{
    ManualChapterOutlineDraft, ManualPartOutlineDraft, OutlineDbError, part_id_for_chapter_number,
    validate_part_outline_layout,
};
use crate::project::{clear_chapter_outline_provenance, link_chapter_to_outline_if_needed};

pub async fn save_manual_chapter_outline_transaction(
    db: &DatabaseConnection,
    project_id: i64,
    draft: ManualChapterOutlineDraft,
    now: NaiveDateTime,
) -> Result<chapter_outline::Model, OutlineDbError> {
    let txn = db.begin().await?;
    let project = crate::project::project_for_update(&txn, project_id)
        .await?
        .ok_or_else(|| OutlineDbError::InvalidArgument(format!("项目不存在: {project_id}")))?;
    if let Some(operation) =
        crate::project::blocked_project_write_operation(&project.lifecycle_gate, "保存章节大纲")
    {
        return Err(OutlineDbError::InvalidState {
            current: project.lifecycle_gate,
            operation,
        });
    }
    crate::project::lock_project_chapter_rows(&txn, project_id).await?;

    let mut outline_query = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .order_by_asc(chapter_outline::Column::Id);
    if txn.get_database_backend() != DbBackend::Sqlite {
        outline_query = outline_query.lock_exclusive();
    }
    let outlines = outline_query.all(&txn).await?;
    let existing = match draft.outline_id {
        Some(outline_id) => Some(
            outlines
                .iter()
                .find(|outline| outline.id == outline_id)
                .cloned()
                .ok_or_else(|| {
                    OutlineDbError::InvalidArgument(format!(
                        "章节大纲 {outline_id} 不属于项目 {project_id}"
                    ))
                })?,
        ),
        None => None,
    };
    if let Some(conflict) = outlines.iter().find(|outline| {
        outline.chapter_number == draft.chapter_number
            && existing
                .as_ref()
                .is_none_or(|current| current.id != outline.id)
    }) {
        return Err(OutlineDbError::InvalidArgument(format!(
            "第 {} 章已有章节大纲 {}，请编辑现有记录",
            draft.chapter_number, conflict.id
        )));
    }

    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(&txn)
        .await?;
    let part_id = part_id_for_chapter_number(&parts, draft.chapter_number)?;

    let outline = match existing {
        Some(existing) => {
            if existing.chapter_number != draft.chapter_number {
                clear_chapter_outline_links(&txn, &[existing.id]).await?;
            }
            let outline_id = existing.id;
            let mut active: chapter_outline::ActiveModel = existing.into();
            active.part_id = Set(part_id);
            active.chapter_number = Set(draft.chapter_number);
            active.title = Set(draft.title);
            active.summary = Set(draft.summary);
            active.status = Set(chapter_outline::STATUS_DRAFT.to_string());
            active.updated_at = Set(now);
            clear_chapter_outline_provenance(&txn, outline_id).await?;
            active.update(&txn).await?
        }
        None => {
            chapter_outline::ActiveModel {
                project_id: Set(project_id),
                part_id: Set(part_id),
                chapter_number: Set(draft.chapter_number),
                title: Set(draft.title),
                summary: Set(draft.summary),
                beats: Set(None),
                key_scenes: Set(None),
                pacing_score: Set(None),
                foreshadowing_setup: Set(None),
                foreshadowing_payoff: Set(None),
                characters_involved: Set(None),
                word_count_target: Set(None),
                status: Set(chapter_outline::STATUS_DRAFT.to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&txn)
            .await?
        }
    };
    link_chapter_to_outline_if_needed(&txn, project_id, draft.chapter_number, outline.id).await?;
    txn.commit().await?;
    Ok(outline)
}

pub async fn save_manual_part_outline_transaction(
    db: &DatabaseConnection,
    project_id: i64,
    draft: ManualPartOutlineDraft,
    now: NaiveDateTime,
) -> Result<part_outline::Model, OutlineDbError> {
    let txn = db.begin().await?;
    let project = crate::project::project_for_update(&txn, project_id)
        .await?
        .ok_or_else(|| OutlineDbError::InvalidArgument(format!("项目不存在: {project_id}")))?;
    if let Some(operation) =
        crate::project::blocked_project_write_operation(&project.lifecycle_gate, "保存分部大纲")
    {
        return Err(OutlineDbError::InvalidState {
            current: project.lifecycle_gate,
            operation,
        });
    }
    crate::project::lock_project_chapter_rows(&txn, project_id).await?;

    let mut part_query = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id);
    if txn.get_database_backend() != DbBackend::Sqlite {
        part_query = part_query.lock_exclusive();
    }
    let parts = part_query.all(&txn).await?;
    let existing = match draft.part_id {
        Some(part_id) => Some(
            parts
                .iter()
                .find(|part| part.id == part_id)
                .cloned()
                .ok_or_else(|| {
                    OutlineDbError::InvalidArgument(format!(
                        "分部大纲 {part_id} 不属于项目 {project_id}"
                    ))
                })?,
        ),
        None => None,
    };
    if let Some(conflict) = parts.iter().find(|part| {
        part.part_number == draft.part_number
            && existing
                .as_ref()
                .is_none_or(|current| current.id != part.id)
    }) {
        return Err(OutlineDbError::InvalidArgument(format!(
            "第 {} 部已有分部大纲 {}，请编辑现有记录",
            draft.part_number, conflict.id
        )));
    }
    let prospective_ranges = parts
        .iter()
        .filter(|part| {
            existing
                .as_ref()
                .is_none_or(|current| current.id != part.id)
        })
        .map(|part| (part.part_number, part.start_chapter, part.end_chapter))
        .chain(std::iter::once((
            draft.part_number,
            draft.start_chapter,
            draft.end_chapter,
        )))
        .collect::<Vec<_>>();
    validate_part_outline_layout(&prospective_ranges)?;

    let part = match existing {
        Some(existing) => {
            let mut active: part_outline::ActiveModel = existing.into();
            active.part_number = Set(draft.part_number);
            active.title = Set(draft.title);
            active.start_chapter = Set(draft.start_chapter);
            active.end_chapter = Set(draft.end_chapter);
            active.summary = Set(draft.summary);
            active.theme = Set(draft.theme);
            active.updated_at = Set(now);
            active.update(&txn).await?
        }
        None => {
            part_outline::ActiveModel {
                project_id: Set(project_id),
                part_number: Set(draft.part_number),
                title: Set(draft.title),
                start_chapter: Set(draft.start_chapter),
                end_chapter: Set(draft.end_chapter),
                summary: Set(draft.summary),
                theme: Set(draft.theme),
                key_events: Set(None),
                character_focus: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&txn)
            .await?
        }
    };

    let current_parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id)
        .all(&txn)
        .await?;
    sync_chapter_outline_part_assignments(&txn, project_id, &current_parts, now).await?;
    txn.commit().await?;
    Ok(part)
}
