use a4n_db_models::models::chapter::part_outline;
use a4n_domain::chapter::PartOutlineRecord;
use chrono::NaiveDateTime;
use sea_orm::ConnectionTrait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use super::super::chapters::sync_chapter_outline_part_assignments;
use super::super::record_mapping::part_outline_model_from_record;
use super::super::{OutlineDbError, PartOutlineSyncDraft};
use super::records::update_part_outline_record;
use super::sync::sync_part_outline_records;

pub async fn sync_part_outline_records_and_assign_chapters_in<C>(
    db: &C,
    project_id: i64,
    drafts: Vec<PartOutlineSyncDraft>,
    now: NaiveDateTime,
) -> Result<Vec<PartOutlineRecord>, OutlineDbError>
where
    C: ConnectionTrait,
{
    let parts = sync_part_outline_records(db, project_id, drafts, now).await?;
    let part_models = parts
        .iter()
        .cloned()
        .map(part_outline_model_from_record)
        .collect::<Vec<_>>();
    sync_chapter_outline_part_assignments(db, project_id, &part_models, now).await?;
    Ok(parts)
}

/// 原子更新单个分部，并按当前全部分部重新绑定现有章纲。
pub async fn update_part_outline_and_rebind_chapters_transaction(
    db: &impl crate::DbExecutor,
    project_id: i64,
    existing: PartOutlineRecord,
    draft: PartOutlineSyncDraft,
    now: NaiveDateTime,
) -> Result<PartOutlineRecord, OutlineDbError> {
    validate_part_outline_project(&existing, project_id)?;
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
    existing: PartOutlineRecord,
    draft: PartOutlineSyncDraft,
    now: NaiveDateTime,
) -> Result<PartOutlineRecord, OutlineDbError>
where
    C: ConnectionTrait,
{
    validate_part_outline_project(&existing, project_id)?;

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

fn validate_part_outline_project(
    existing: &PartOutlineRecord,
    project_id: i64,
) -> Result<(), OutlineDbError> {
    if existing.project_id == project_id {
        return Ok(());
    }
    Err(OutlineDbError::InvalidArgument(format!(
        "分部大纲 {} 不属于项目 {project_id}",
        existing.id
    )))
}
