use a4n_core::models::chapter::chapter;
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::collections::HashMap;

use super::super::ProjectDuplicateError;

pub(super) async fn copy_project_chapters<C>(
    db: &C,
    source_project_id: i64,
    target_project_id: i64,
    outline_id_map: &HashMap<i64, i64>,
    now: NaiveDateTime,
) -> Result<HashMap<i64, i64>, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(source_project_id))
        .order_by_asc(chapter::Column::ChapterNumber)
        .order_by_asc(chapter::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let mut id_map = HashMap::new();
    for chapter in chapters {
        let outline_id = chapter
            .outline_id
            .map(|old_outline_id| {
                outline_id_map.get(&old_outline_id).copied().ok_or_else(|| {
                    ProjectDuplicateError::InvalidState {
                        current: format!(
                            "第{}章引用缺失的章节大纲 outline_id={old_outline_id}",
                            chapter.chapter_number
                        ),
                        operation: "复制章节引用".to_string(),
                    }
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
        .map_err(ProjectDuplicateError::Database)?;
        id_map.insert(chapter.id, inserted.id);
    }
    Ok(id_map)
}
