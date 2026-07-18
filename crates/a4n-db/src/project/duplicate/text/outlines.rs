use a4n_core::models::chapter::{chapter_outline, part_outline};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::collections::HashMap;

use super::super::ProjectDuplicateError;

pub(super) async fn copy_project_part_outlines<C>(
    db: &C,
    source_project_id: i64,
    target_project_id: i64,
    now: NaiveDateTime,
) -> Result<HashMap<i64, i64>, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(source_project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let mut id_map = HashMap::new();
    for part in parts {
        let inserted = part_outline::ActiveModel {
            project_id: Set(target_project_id),
            part_number: Set(part.part_number),
            title: Set(part.title),
            start_chapter: Set(part.start_chapter),
            end_chapter: Set(part.end_chapter),
            summary: Set(part.summary),
            theme: Set(part.theme),
            key_events: Set(part.key_events),
            character_focus: Set(part.character_focus),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
        id_map.insert(part.id, inserted.id);
    }
    Ok(id_map)
}

pub(super) async fn copy_project_chapter_outlines<C>(
    db: &C,
    source_project_id: i64,
    target_project_id: i64,
    part_id_map: &HashMap<i64, i64>,
    now: NaiveDateTime,
) -> Result<HashMap<i64, i64>, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(source_project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .order_by_asc(chapter_outline::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let mut id_map = HashMap::new();
    for outline in outlines {
        let part_id = outline
            .part_id
            .map(|old_part_id| {
                part_id_map.get(&old_part_id).copied().ok_or_else(|| {
                    ProjectDuplicateError::InvalidState {
                        current: format!(
                            "第{}章大纲引用缺失的分部大纲 part_id={old_part_id}",
                            outline.chapter_number
                        ),
                        operation: "复制章节大纲引用".to_string(),
                    }
                })
            })
            .transpose()?;
        let inserted = chapter_outline::ActiveModel {
            project_id: Set(target_project_id),
            part_id: Set(part_id),
            chapter_number: Set(outline.chapter_number),
            title: Set(outline.title),
            summary: Set(outline.summary),
            beats: Set(outline.beats),
            key_scenes: Set(outline.key_scenes),
            pacing_score: Set(outline.pacing_score),
            foreshadowing_setup: Set(outline.foreshadowing_setup),
            foreshadowing_payoff: Set(outline.foreshadowing_payoff),
            characters_involved: Set(outline.characters_involved),
            word_count_target: Set(outline.word_count_target),
            status: Set(outline.status),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
        id_map.insert(outline.id, inserted.id);
    }
    Ok(id_map)
}
