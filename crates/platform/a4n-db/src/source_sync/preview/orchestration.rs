use super::{
    anchor_conflicts::{insert_anchor_conflicts_for_run, insert_galgame_anchor_conflicts_for_run},
    chapter_conflicts::{
        insert_conflicts_for_changes, insert_conflicts_for_profile_changes,
        insert_galgame_conflicts_for_changes,
    },
};
use crate::{
    project::ProjectFamilyDbError,
    source_sync::{
        SOURCE_SYNC_DETACHED, SourceSyncDbError, SourceSyncPlanRecord,
        UpdateSourceSyncDryRunSummaryInput, diff_source_snapshots,
        insert_source_sync_dry_run_record, source_sync_chapter_scope_for_derived_project,
        source_sync_plan_record, update_source_sync_dry_run_summary,
    },
};
use a4n_db_models::models::source_sync::source_sync_run;
use a4n_domain::source_sync::CreateSourceSyncDryRunRecordInput;
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, TransactionTrait};

pub async fn create_source_sync_preview_records<T>(
    db: &T,
    derived_project_id: i64,
) -> Result<SourceSyncPlanRecord, SourceSyncDbError>
where
    T: TransactionTrait,
{
    let tx = db.begin().await?;
    let context = crate::project::derived_project_family_context(&tx, derived_project_id)
        .await
        .map_err(source_sync_project_family_error)?;
    if context.project.source_sync_status == SOURCE_SYNC_DETACHED {
        return Err(SourceSyncDbError::InvalidState {
            current: SOURCE_SYNC_DETACHED.to_string(),
            operation: "preview_source_sync".to_string(),
        });
    }
    let applying_run = source_sync_run::Entity::find()
        .filter(source_sync_run::Column::DerivedProjectId.eq(derived_project_id))
        .filter(source_sync_run::Column::RootProjectId.eq(context.root_project.id))
        .filter(source_sync_run::Column::ToSnapshotId.eq(context.source_snapshot.id))
        .filter(source_sync_run::Column::Status.eq(crate::source_sync::SYNC_RUN_STATUS_APPLYING))
        .order_by_desc(source_sync_run::Column::Id)
        .one(&tx)
        .await?;
    if let Some(run) = applying_run {
        let plan = source_sync_plan_record(&tx, run).await?;
        tx.commit().await?;
        return Ok(plan);
    }
    let scope = source_sync_chapter_scope_for_derived_project(&tx, derived_project_id).await?;
    let to_snapshot = crate::project::create_source_snapshot_in_record(
        &tx,
        context.root_project.id,
        scope,
        Utc::now().naive_utc(),
    )
    .await?;
    let diff = diff_source_snapshots(&tx, context.source_snapshot.id, to_snapshot.id).await?;
    let chapter_change_count = diff.chapter_changes.len();
    let profile_change_count = diff.profile_changes.len();
    let run = insert_source_sync_dry_run_record(
        &tx,
        CreateSourceSyncDryRunRecordInput {
            derived_project_id: context.project.id,
            root_project_id: context.root_project.id,
            from_snapshot_id: context.source_snapshot.id,
            to_snapshot_id: to_snapshot.id,
            chapter_change_count,
            profile_change_count,
        },
    )
    .await?;
    let mut conflicts = Vec::new();
    let mut galgame_scene_conflict_count = 0usize;
    let chapter_conflicts = insert_conflicts_for_changes(&tx, &run, &diff.chapter_changes).await?;
    let galgame_conflicts =
        insert_galgame_conflicts_for_changes(&tx, &run, &diff.chapter_changes).await?;
    for ((_, regular_conflicts), galgame_conflicts) in diff
        .chapter_changes
        .iter()
        .zip(chapter_conflicts)
        .zip(galgame_conflicts)
    {
        conflicts.extend(regular_conflicts);
        galgame_scene_conflict_count += galgame_conflicts.len();
        conflicts.extend(galgame_conflicts);
    }
    let profile_conflicts =
        insert_conflicts_for_profile_changes(&tx, &run, &diff.profile_changes).await?;
    for profile_conflicts in profile_conflicts {
        conflicts.extend(profile_conflicts);
    }
    let anchor_conflicts = insert_anchor_conflicts_for_run(&tx, &run).await?;
    let anchor_conflict_count = anchor_conflicts.len();
    conflicts.extend(anchor_conflicts);
    let galgame_anchor_conflicts = insert_galgame_anchor_conflicts_for_run(&tx, &run).await?;
    let galgame_anchor_conflict_count = galgame_anchor_conflicts.len();
    conflicts.extend(galgame_anchor_conflicts);
    let run = update_source_sync_dry_run_summary(
        &tx,
        UpdateSourceSyncDryRunSummaryInput {
            run,
            chapter_change_count,
            profile_change_count,
            anchor_conflict_count,
            galgame_scene_conflict_count,
            galgame_anchor_conflict_count,
            conflict_count: conflicts.len(),
        },
    )
    .await?;
    let regeneration_targets = crate::source_sync::galgame::galgame_scene_regeneration_targets(
        &tx,
        run.derived_project_id,
        &conflicts,
    )
    .await?;
    tx.commit().await?;
    Ok(SourceSyncPlanRecord {
        run,
        conflicts,
        regeneration_targets,
    })
}

fn source_sync_project_family_error(error: ProjectFamilyDbError) -> SourceSyncDbError {
    match error {
        ProjectFamilyDbError::InvalidArgument(message) => {
            SourceSyncDbError::InvalidArgument(message)
        }
        ProjectFamilyDbError::NotFound(message) => SourceSyncDbError::NotFound(message),
        ProjectFamilyDbError::Database(error) => SourceSyncDbError::Database(error),
    }
}
