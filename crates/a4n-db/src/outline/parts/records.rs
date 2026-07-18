use a4n_core::models::chapter::part_outline;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use super::super::{OutlineDbError, PartOutlineSyncDraft};

/// 插入分部大纲记录。
pub async fn insert_part_outline_record<C>(
    db: &C,
    project_id: i64,
    draft: &PartOutlineSyncDraft,
    now: NaiveDateTime,
) -> Result<part_outline::Model, OutlineDbError>
where
    C: ConnectionTrait,
{
    Ok(part_outline::ActiveModel {
        project_id: Set(project_id),
        part_number: Set(draft.part_number),
        title: Set(draft.title.clone()),
        start_chapter: Set(draft.start_chapter),
        end_chapter: Set(draft.end_chapter),
        summary: Set(Some(draft.summary.clone())),
        theme: Set(Some(draft.theme.clone())),
        key_events: Set(Some(draft.key_events.clone())),
        character_focus: Set(Some(draft.character_focus.clone())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?)
}

/// 更新既有分部大纲记录。
pub async fn update_part_outline_record<C>(
    db: &C,
    existing: part_outline::Model,
    draft: &PartOutlineSyncDraft,
    now: NaiveDateTime,
) -> Result<part_outline::Model, OutlineDbError>
where
    C: ConnectionTrait,
{
    let mut active: part_outline::ActiveModel = existing.into();
    active.part_number = Set(draft.part_number);
    active.title = Set(draft.title.clone());
    active.start_chapter = Set(draft.start_chapter);
    active.end_chapter = Set(draft.end_chapter);
    active.summary = Set(Some(draft.summary.clone()));
    active.theme = Set(Some(draft.theme.clone()));
    active.key_events = Set(Some(draft.key_events.clone()));
    active.character_focus = Set(Some(draft.character_focus.clone()));
    active.updated_at = Set(now);
    Ok(active.update(db).await?)
}

pub(super) fn part_outline_matches(
    existing: &part_outline::Model,
    draft: &PartOutlineSyncDraft,
) -> bool {
    existing.part_number == draft.part_number
        && existing.start_chapter == draft.start_chapter
        && existing.end_chapter == draft.end_chapter
        && existing.title == draft.title
        && existing.summary.as_deref() == Some(draft.summary.as_str())
        && existing.theme.as_deref() == Some(draft.theme.as_str())
        && existing.key_events.as_ref() == Some(&draft.key_events)
        && existing.character_focus.as_ref() == Some(&draft.character_focus)
}
