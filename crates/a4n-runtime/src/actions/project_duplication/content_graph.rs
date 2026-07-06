use std::collections::HashMap;

use a4n_core::models::chapter::{chapter, chapter_outline, part_outline};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

pub(super) async fn copy_part_outlines<C>(
    db: &C,
    source_project_id: i64,
    target_project_id: i64,
    now: chrono::NaiveDateTime,
) -> Result<HashMap<i64, i64>, String>
where
    C: ConnectionTrait,
{
    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(source_project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
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
        .map_err(|e| e.to_string())?;
        id_map.insert(part.id, inserted.id);
    }
    Ok(id_map)
}

pub(super) async fn copy_chapter_outlines<C>(
    db: &C,
    source_project_id: i64,
    target_project_id: i64,
    part_id_map: &HashMap<i64, i64>,
    now: chrono::NaiveDateTime,
) -> Result<HashMap<i64, i64>, String>
where
    C: ConnectionTrait,
{
    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(source_project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .order_by_asc(chapter_outline::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let mut id_map = HashMap::new();
    for outline in outlines {
        let part_id = outline
            .part_id
            .map(|old_part_id| {
                part_id_map.get(&old_part_id).copied().ok_or_else(|| {
                    format!(
                        "第{}章大纲引用的分部大纲不存在: part_id={old_part_id}",
                        outline.chapter_number
                    )
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
        .map_err(|e| e.to_string())?;
        id_map.insert(outline.id, inserted.id);
    }
    Ok(id_map)
}

pub(super) async fn copy_chapters<C>(
    db: &C,
    source_project_id: i64,
    target_project_id: i64,
    outline_id_map: &HashMap<i64, i64>,
    now: chrono::NaiveDateTime,
) -> Result<HashMap<i64, i64>, String>
where
    C: ConnectionTrait,
{
    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(source_project_id))
        .order_by_asc(chapter::Column::ChapterNumber)
        .order_by_asc(chapter::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let mut id_map = HashMap::new();
    for chapter in chapters {
        let outline_id = chapter
            .outline_id
            .map(|old_outline_id| {
                outline_id_map.get(&old_outline_id).copied().ok_or_else(|| {
                    format!(
                        "第{}章引用的章节大纲不存在: outline_id={old_outline_id}",
                        chapter.chapter_number
                    )
                })
            })
            .transpose()?;
        let inserted = chapter::ActiveModel {
            project_id: Set(target_project_id),
            outline_id: Set(outline_id),
            chapter_number: Set(chapter.chapter_number),
            title: Set(chapter.title),
            content: Set(chapter.content),
            word_count: Set(chapter.word_count),
            status: Set(chapter.status),
            summary: Set(chapter.summary),
            style_tags: Set(chapter.style_tags),
            analysis_data: Set(chapter.analysis_data),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| e.to_string())?;
        id_map.insert(chapter.id, inserted.id);
    }
    Ok(id_map)
}
