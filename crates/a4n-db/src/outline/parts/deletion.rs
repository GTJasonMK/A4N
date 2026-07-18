use a4n_core::models::chapter::{chapter_outline, part_outline};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};

use super::super::OutlineDbError;
use super::super::ranges::{chapter_outline_ids_in_part_ranges, latest_part_ids_to_delete};
use crate::generation::delete_generation_run_targets_for;

/// 删除项目最后 N 个分部大纲，并级联删除对应章节范围内的章节大纲与 generation run target。
pub async fn delete_latest_part_outline_records(
    db: &DatabaseConnection,
    project_id: i64,
    count: u64,
) -> Result<(), OutlineDbError> {
    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(db)
        .await?;
    let part_ids = latest_part_ids_to_delete(&parts, count)?;
    let parts_to_delete = parts
        .iter()
        .filter(|part| part_ids.contains(&part.id))
        .cloned()
        .collect::<Vec<_>>();

    let txn = db.begin().await?;
    let chapter_outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .all(&txn)
        .await?;
    let chapter_outline_ids =
        chapter_outline_ids_in_part_ranges(&chapter_outlines, &parts_to_delete);

    if !chapter_outline_ids.is_empty() {
        chapter_outline::Entity::delete_many()
            .filter(chapter_outline::Column::Id.is_in(chapter_outline_ids.iter().copied()))
            .exec(&txn)
            .await?;
        delete_generation_run_targets_for(&txn, "chapter_outline", &chapter_outline_ids).await?;
    }

    part_outline::Entity::delete_many()
        .filter(part_outline::Column::Id.is_in(part_ids.iter().copied()))
        .exec(&txn)
        .await?;
    delete_generation_run_targets_for(&txn, "part_outline", &part_ids).await?;
    txn.commit().await?;
    Ok(())
}
