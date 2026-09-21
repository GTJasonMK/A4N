use a4n_db_models::models::chapter::{chapter_outline, part_outline};
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};
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
    if parts.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = parts
        .iter()
        .map(|part| part_outline::ActiveModel {
            project_id: Set(target_project_id),
            part_number: Set(part.part_number),
            title: Set(part.title.clone()),
            start_chapter: Set(part.start_chapter),
            end_chapter: Set(part.end_chapter),
            summary: Set(part.summary.clone()),
            theme: Set(part.theme.clone()),
            key_events: Set(part.key_events.clone()),
            character_focus: Set(part.character_focus.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let inserted = part_outline::Entity::insert_many(rows)
        .exec_with_returning_many(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if inserted.len() != parts.len() {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的分部大纲数量与源记录不一致".to_string(),
            operation: "复制分部大纲".to_string(),
        });
    }
    let mut id_map = HashMap::with_capacity(parts.len());
    for (part, copied) in parts.iter().zip(inserted) {
        id_map.insert(part.id, copied.id);
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
    if outlines.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = outlines
        .iter()
        .map(|outline| {
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
            Ok::<_, ProjectDuplicateError>(chapter_outline::ActiveModel {
                project_id: Set(target_project_id),
                part_id: Set(part_id),
                chapter_number: Set(outline.chapter_number),
                title: Set(outline.title.clone()),
                summary: Set(outline.summary.clone()),
                beats: Set(outline.beats.clone()),
                key_scenes: Set(outline.key_scenes.clone()),
                pacing_score: Set(outline.pacing_score),
                foreshadowing_setup: Set(outline.foreshadowing_setup.clone()),
                foreshadowing_payoff: Set(outline.foreshadowing_payoff.clone()),
                characters_involved: Set(outline.characters_involved.clone()),
                word_count_target: Set(outline.word_count_target),
                status: Set(outline.status.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, ProjectDuplicateError>>()?;
    let inserted = chapter_outline::Entity::insert_many(rows)
        .exec_with_returning_many(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if inserted.len() != outlines.len() {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的章节大纲数量与源记录不一致".to_string(),
            operation: "复制章节大纲".to_string(),
        });
    }
    let mut id_map = HashMap::with_capacity(outlines.len());
    for (outline, copied) in outlines.iter().zip(inserted) {
        id_map.insert(outline.id, copied.id);
    }
    Ok(id_map)
}
