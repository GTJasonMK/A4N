use a4n_core::models::{
    project::{novel_project, project_derivation, project_source_snapshot},
    source_sync::source_sync_run,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::source_sync::snapshots::{
    calculate_source_sync_version_hash, parse_source_sync_chapter_scope,
};
use crate::source_sync::{
    SOURCE_SYNC_DETACHED, SYNC_RUN_STATUS_APPLYING, SYNC_RUN_STATUS_PLANNED, SourceSyncDbError,
};

pub async fn load_source_sync_apply_run_record<C>(
    db: &C,
    sync_run_id: i64,
) -> Result<source_sync_run::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let run = source_sync_run::Entity::find_by_id(sync_run_id)
        .one(db)
        .await?
        .ok_or_else(|| SourceSyncDbError::NotFound(format!("源同步运行不存在: {sync_run_id}")))?;
    if run.status != SYNC_RUN_STATUS_PLANNED && run.status != SYNC_RUN_STATUS_APPLYING {
        return Err(SourceSyncDbError::InvalidState {
            current: run.status,
            operation: "apply_source_sync_decisions".to_string(),
        });
    }
    let derived_project = novel_project::Entity::find_by_id(run.derived_project_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            SourceSyncDbError::NotFound(format!("派生项目不存在: {}", run.derived_project_id))
        })?;
    let expected_project_snapshot_id = if run.status == SYNC_RUN_STATUS_APPLYING {
        run.to_snapshot_id
    } else {
        run.from_snapshot_id
    };
    if derived_project.root_project_id != Some(run.root_project_id)
        || derived_project.source_snapshot_id != Some(expected_project_snapshot_id)
    {
        return Err(SourceSyncDbError::InvalidState {
            current: "source_sync_run_expired".to_string(),
            operation: "apply_source_sync_decisions".to_string(),
        });
    }
    if derived_project.source_sync_status == SOURCE_SYNC_DETACHED {
        return Err(SourceSyncDbError::InvalidState {
            current: SOURCE_SYNC_DETACHED.to_string(),
            operation: "apply_source_sync_decisions".to_string(),
        });
    }
    let derivation = project_derivation::Entity::find()
        .filter(project_derivation::Column::DerivedProjectId.eq(run.derived_project_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            SourceSyncDbError::InvalidArgument(format!(
                "派生项目缺少派生关系: {}",
                run.derived_project_id
            ))
        })?;
    let scope = parse_source_sync_chapter_scope(&derivation.chapter_scope_json)?;
    let current_hash = calculate_source_sync_version_hash(db, run.root_project_id, scope).await?;
    let target_snapshot = project_source_snapshot::Entity::find_by_id(run.to_snapshot_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            SourceSyncDbError::NotFound(format!("目标源快照不存在: {}", run.to_snapshot_id))
        })?;
    if current_hash != target_snapshot.source_version_hash {
        return Err(SourceSyncDbError::InvalidState {
            current: "root_changed_after_dry_run".to_string(),
            operation: "apply_source_sync_decisions".to_string(),
        });
    }
    Ok(run)
}
