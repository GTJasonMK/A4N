use a4n_core::models::chapter::chapter;
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};
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
    if chapters.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = chapters
        .iter()
        .map(|chapter| {
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
            Ok::<_, ProjectDuplicateError>(chapter::ActiveModel {
                project_id: Set(target_project_id),
                outline_id: Set(outline_id),
                chapter_number: Set(chapter.chapter_number),
                title: Set(chapter.title.clone()),
                content: Set(chapter.content.clone()),
                word_count: Set(chapter.word_count),
                status: Set(chapter.status.clone()),
                summary: Set(chapter.summary.clone()),
                style_tags: Set(chapter.style_tags.clone()),
                analysis_data: Set(chapter.analysis_data.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, ProjectDuplicateError>>()?;
    let inserted = chapter::Entity::insert_many(rows)
        .exec_with_returning_many(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if inserted.len() != chapters.len() {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的章节数量与源记录不一致".to_string(),
            operation: "复制章节".to_string(),
        });
    }
    let mut id_map = HashMap::with_capacity(chapters.len());
    for (chapter, copied) in chapters.iter().zip(inserted) {
        id_map.insert(chapter.id, copied.id);
    }
    Ok(id_map)
}
