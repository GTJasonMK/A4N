use a4n_core::models::chapter::part_outline;
use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};

use super::super::chapters::sync_chapter_outline_part_assignments;
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
    super::super::lock_outline_project_for_write(&txn, project_id, "同步分部大纲").await?;
    let parts =
        sync_part_outline_records_and_assign_chapters_in(&txn, project_id, drafts, now).await?;
    txn.commit().await?;
    Ok(parts)
}

pub async fn sync_part_outline_records_and_assign_chapters_in<C>(
    db: &C,
    project_id: i64,
    drafts: Vec<PartOutlineSyncDraft>,
    now: NaiveDateTime,
) -> Result<Vec<part_outline::Model>, OutlineDbError>
where
    C: ConnectionTrait,
{
    let parts = sync_part_outline_records(db, project_id, drafts, now).await?;
    sync_chapter_outline_part_assignments(db, project_id, &parts, now).await?;
    Ok(parts)
}

/// 原子更新单个分部，并按当前全部分部重新绑定现有章纲。
pub async fn update_part_outline_and_rebind_chapters_transaction(
    db: &DatabaseConnection,
    project_id: i64,
    existing: part_outline::Model,
    draft: PartOutlineSyncDraft,
    now: NaiveDateTime,
) -> Result<part_outline::Model, OutlineDbError> {
    let txn = db.begin().await?;
    super::super::lock_outline_project_for_write(&txn, project_id, "更新分部大纲").await?;
    let updated =
        update_part_outline_and_rebind_chapters_in(&txn, project_id, existing, draft, now).await?;
    txn.commit().await?;
    Ok(updated)
}

pub async fn update_part_outline_and_rebind_chapters_in<C>(
    db: &C,
    project_id: i64,
    existing: part_outline::Model,
    draft: PartOutlineSyncDraft,
    now: NaiveDateTime,
) -> Result<part_outline::Model, OutlineDbError>
where
    C: ConnectionTrait,
{
    if existing.project_id != project_id {
        return Err(OutlineDbError::InvalidArgument(format!(
            "分部大纲 {} 不属于项目 {project_id}",
            existing.id
        )));
    }

    let updated = update_part_outline_record(db, existing, &draft, now).await?;
    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id)
        .all(db)
        .await?;
    sync_chapter_outline_part_assignments(db, project_id, &parts, now).await?;
    Ok(updated)
}
