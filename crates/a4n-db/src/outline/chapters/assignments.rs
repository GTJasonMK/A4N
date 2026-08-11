use a4n_core::models::chapter::{chapter_outline, part_outline};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use super::super::ranges::expected_part_id_for_existing_outline;
use super::super::types::OutlineDbError;

/// 按当前分部范围重绑章节大纲的 part_id。
pub async fn sync_chapter_outline_part_assignments<C>(
    db: &C,
    project_id: i64,
    parts: &[part_outline::Model],
    now: NaiveDateTime,
) -> Result<(), OutlineDbError>
where
    C: ConnectionTrait,
{
    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(db)
        .await?;

    for outline in outlines {
        let expected_part_id = expected_part_id_for_existing_outline(parts, &outline)?;
        if outline.part_id == expected_part_id {
            continue;
        }
        let updated_at = std::cmp::max(now, outline.updated_at + chrono::Duration::microseconds(1));
        let mut active: chapter_outline::ActiveModel = outline.into();
        active.part_id = Set(expected_part_id);
        active.updated_at = Set(updated_at);
        active.update(db).await?;
    }

    Ok(())
}
