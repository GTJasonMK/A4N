use a4n_core::models::{
    project::{novel_project, project_derivation},
    source_sync::{source_sync_conflict, source_sync_run},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, Set,
    sea_query::Expr,
};

use super::contracts::FinalizeSourceSyncApplyRunInput;
use crate::source_sync::{
    CONFLICT_STATUS_PENDING, SOURCE_SYNC_CONFLICT, SOURCE_SYNC_CURRENT, SOURCE_SYNC_DETACHED,
    SOURCE_SYNC_ROOT_UPDATED, SYNC_RUN_MODE_APPLY, SYNC_RUN_STATUS_APPLYING,
    SYNC_RUN_STATUS_COMPLETED, SourceSyncDbError,
};

pub async fn finalize_source_sync_apply_run_record<C>(
    db: &C,
    input: FinalizeSourceSyncApplyRunInput,
) -> Result<source_sync_run::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let run = input.run;
    let pending_blocking = source_sync_conflict::Entity::find()
        .filter(source_sync_conflict::Column::SyncRunId.eq(run.id))
        .filter(source_sync_conflict::Column::Status.eq(CONFLICT_STATUS_PENDING))
        .filter(source_sync_conflict::Column::Severity.eq("blocking"))
        .count(db)
        .await?;
    let pending_any = source_sync_conflict::Entity::find()
        .filter(source_sync_conflict::Column::SyncRunId.eq(run.id))
        .filter(source_sync_conflict::Column::Status.eq(CONFLICT_STATUS_PENDING))
        .count(db)
        .await?;
    let new_status = if input.detach_requested {
        SOURCE_SYNC_DETACHED
    } else if pending_blocking > 0 {
        SOURCE_SYNC_CONFLICT
    } else if pending_any > 0 {
        SOURCE_SYNC_ROOT_UPDATED
    } else {
        SOURCE_SYNC_CURRENT
    };

    let mut project_update = novel_project::Entity::update_many();
    project_update = project_update
        .col_expr(
            novel_project::Column::SourceSyncStatus,
            Expr::value(new_status),
        )
        .col_expr(
            novel_project::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(novel_project::Column::Id.eq(run.derived_project_id));
    if !input.detach_requested {
        project_update = project_update.col_expr(
            novel_project::Column::SourceSnapshotId,
            Expr::value(run.to_snapshot_id),
        );
    }
    project_update.exec(db).await?;

    if !input.detach_requested {
        project_derivation::Entity::update_many()
            .col_expr(
                project_derivation::Column::SourceSnapshotId,
                Expr::value(run.to_snapshot_id),
            )
            .filter(project_derivation::Column::DerivedProjectId.eq(run.derived_project_id))
            .exec(db)
            .await?;
    }

    let new_run_status = if pending_any > 0 && !input.detach_requested {
        SYNC_RUN_STATUS_APPLYING
    } else {
        SYNC_RUN_STATUS_COMPLETED
    };
    let mut active: source_sync_run::ActiveModel = run.into();
    active.mode = Set(SYNC_RUN_MODE_APPLY.to_string());
    active.status = Set(new_run_status.to_string());
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(SourceSyncDbError::Database)
}
