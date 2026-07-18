use a4n_core::models::chapter::chapter_outline;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use super::super::types::{ChapterOutlineSyncDraft, OutlineDbError};

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
    Ok(chapter_outline::ActiveModel {
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
    .insert(db)
    .await?)
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
    active.updated_at = Set(now);
    Ok(active.update(db).await?)
}
