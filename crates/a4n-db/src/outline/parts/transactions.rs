use a4n_core::models::chapter::part_outline;
use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};

use super::super::chapters::{
    delete_chapter_outlines_in_part_range, sync_chapter_outline_part_assignments,
};
use super::super::{OutlineDbError, PartOutlineSyncDraft};
use super::records::update_part_outline_record;
use super::sync::sync_part_outline_records;

/// 原子同步全部分部大纲，并按同步后的分部范围重绑现有章节大纲。
pub async fn sync_part_outline_records_and_assign_chapters_transaction(
    db: &DatabaseConnection,
    project_id: i64,
    drafts: Vec<PartOutlineSyncDraft>,
    now: NaiveDateTime,
) -> Result<Vec<part_outline::Model>, OutlineDbError> {
    let txn = db.begin().await?;
    let parts = sync_part_outline_records(&txn, project_id, drafts, now).await?;
    sync_chapter_outline_part_assignments(&txn, project_id, &parts, now).await?;
    txn.commit().await?;
    Ok(parts)
}

/// 原子更新单个分部，删除其章节范围内的旧章纲，并按当前全部分部重新绑定剩余章纲。
pub async fn update_part_outline_and_rebind_chapters_transaction(
    db: &DatabaseConnection,
    project_id: i64,
    existing: part_outline::Model,
    draft: PartOutlineSyncDraft,
    now: NaiveDateTime,
) -> Result<part_outline::Model, OutlineDbError> {
    if existing.project_id != project_id {
        return Err(OutlineDbError::InvalidArgument(format!(
            "分部大纲 {} 不属于项目 {project_id}",
            existing.id
        )));
    }

    let txn = db.begin().await?;
    let updated = update_part_outline_record(&txn, existing, &draft, now).await?;
    delete_chapter_outlines_in_part_range(&txn, project_id, &updated).await?;
    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id)
        .all(&txn)
        .await?;
    sync_chapter_outline_part_assignments(&txn, project_id, &parts, now).await?;
    txn.commit().await?;
    Ok(updated)
}
