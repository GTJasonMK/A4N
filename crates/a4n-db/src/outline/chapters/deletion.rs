use a4n_core::models::chapter::{chapter_outline, part_outline};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use super::super::ranges::chapter_outline_ids_in_part_range;
use super::super::types::OutlineDbError;
use crate::generation::delete_generation_run_targets_for;

/// 删除某分部章节范围内的章纲，并清理 generation run target。
pub async fn delete_chapter_outlines_in_part_range<C>(
    db: &C,
    project_id: i64,
    part: &part_outline::Model,
) -> Result<Vec<i64>, OutlineDbError>
where
    C: ConnectionTrait,
{
    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.gte(part.start_chapter))
        .filter(chapter_outline::Column::ChapterNumber.lte(part.end_chapter))
        .all(db)
        .await?;
    let outline_ids = chapter_outline_ids_in_part_range(&outlines, part);
    if outline_ids.is_empty() {
        return Ok(outline_ids);
    }

    chapter_outline::Entity::delete_many()
        .filter(chapter_outline::Column::Id.is_in(outline_ids.iter().copied()))
        .exec(db)
        .await?;
    delete_generation_run_targets_for(db, "chapter_outline", &outline_ids).await?;
    Ok(outline_ids)
}
