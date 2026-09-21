use a4n_db_models::models::chapter::chapter_outline;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseConnection, Set, TransactionTrait};

use super::super::types::OutlineDbError;
use a4n_domain::chapter::ChapterOutlineSyncDraft;

pub(super) fn chapter_outline_active_model(
    project_id: i64,
    part_id: Option<i64>,
    draft: &ChapterOutlineSyncDraft,
    now: NaiveDateTime,
) -> chapter_outline::ActiveModel {
    chapter_outline::ActiveModel {
        project_id: Set(project_id),
        part_id: Set(part_id),
        chapter_number: Set(draft.chapter_number),
        title: Set(Some(draft.title.clone())),
        summary: Set(Some(draft.summary.clone())),
        beats: Set(draft.beats.clone()),
        key_scenes: Set(draft.key_scenes.clone()),
        pacing_score: Set(draft.pacing_score),
        foreshadowing_setup: Set(draft.foreshadowing_setup.clone()),
        foreshadowing_payoff: Set(draft.foreshadowing_payoff.clone()),
        characters_involved: Set(draft.characters_involved.clone()),
        word_count_target: Set(draft.word_count_target),
        status: Set("draft".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

/// 插入章节大纲记录。
pub async fn insert_chapter_outline_record<C>(
    db: &C,
    project_id: i64,
    part_id: Option<i64>,
    draft: &ChapterOutlineSyncDraft,
    now: NaiveDateTime,
) -> Result<chapter_outline::Model, OutlineDbError>
where
    C: ConnectionTrait,
{
    Ok(
        chapter_outline_active_model(project_id, part_id, draft, now)
            .insert(db)
            .await?,
    )
}

/// 更新既有章节大纲记录。
pub async fn update_chapter_outline_record<C>(
    db: &C,
    existing: chapter_outline::Model,
    part_id: Option<i64>,
    draft: &ChapterOutlineSyncDraft,
    now: NaiveDateTime,
) -> Result<chapter_outline::Model, OutlineDbError>
where
    C: ConnectionTrait,
{
    crate::project::clear_chapter_outline_provenance(db, existing.id).await?;
    let updated_at = std::cmp::max(now, existing.updated_at + chrono::Duration::microseconds(1));
    let mut active: chapter_outline::ActiveModel = existing.into();
    active.part_id = Set(part_id);
    active.title = Set(Some(draft.title.clone()));
    active.summary = Set(Some(draft.summary.clone()));
    active.beats = Set(draft.beats.clone());
    active.key_scenes = Set(draft.key_scenes.clone());
    active.pacing_score = Set(draft.pacing_score);
    active.foreshadowing_setup = Set(draft.foreshadowing_setup.clone());
    active.foreshadowing_payoff = Set(draft.foreshadowing_payoff.clone());
    active.characters_involved = Set(draft.characters_involved.clone());
    active.word_count_target = Set(draft.word_count_target);
    active.status = Set("draft".to_string());
    active.updated_at = Set(updated_at);
    Ok(active.update(db).await?)
}

pub async fn update_chapter_outline_record_transaction(
    db: &DatabaseConnection,
    existing: chapter_outline::Model,
    part_id: Option<i64>,
    draft: &ChapterOutlineSyncDraft,
    now: NaiveDateTime,
) -> Result<chapter_outline::Model, OutlineDbError> {
    let txn = db.begin().await?;
    super::super::lock_outline_project_for_write(&txn, existing.project_id, "更新章节大纲").await?;
    let updated = update_chapter_outline_record(&txn, existing, part_id, draft, now).await?;
    txn.commit().await?;
    Ok(updated)
}
