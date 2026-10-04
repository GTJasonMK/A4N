use super::*;
use a4n_domain::source_sync::SourceSyncConflictStatus;

pub(super) async fn insert_source_sync_rows(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    insert_mapped_import_rows!(
        txn,
        &package.source_sync_runs,
        run,
        source_sync_run::ActiveModel,
        maps.source_sync_run_ids,
        "导入源同步运行失败",
        |active| {
            active.derived_project_id = Set(map_required(
                &maps.project_ids,
                run.derived_project_id,
                "同步运行 derived_project_id",
            )?);
            active.root_project_id = Set(map_required(
                &maps.project_ids,
                run.root_project_id,
                "同步运行 root_project_id",
            )?);
            active.from_snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                run.from_snapshot_id,
                "同步运行 from_snapshot_id",
            )?);
            active.to_snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                run.to_snapshot_id,
                "同步运行 to_snapshot_id",
            )?);
        }
    );
    insert_mapped_import_rows!(
        txn,
        &package.source_sync_conflicts,
        conflict,
        source_sync_conflict::ActiveModel,
        maps.source_sync_conflict_ids,
        "导入源同步冲突失败",
        |active| {
            active.sync_run_id = Set(map_required(
                &maps.source_sync_run_ids,
                conflict.sync_run_id,
                "同步冲突 sync_run_id",
            )?);
            active.derived_project_id = Set(map_required(
                &maps.project_ids,
                conflict.derived_project_id,
                "同步冲突 derived_project_id",
            )?);
            active.root_project_id = Set(map_required(
                &maps.project_ids,
                conflict.root_project_id,
                "同步冲突 root_project_id",
            )?);
            active.from_snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                conflict.from_snapshot_id,
                "同步冲突 from_snapshot_id",
            )?);
            active.to_snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                conflict.to_snapshot_id,
                "同步冲突 to_snapshot_id",
            )?);
            if conflict.status != SourceSyncConflictStatus::Obsolete.as_str() {
                active.target_id = Set(remap_source_sync_target_id(conflict, maps)?);
                active.source_summary_json = Set(remap_conflict_source_summary(conflict, maps)?);
                active.local_summary_json = Set(remap_conflict_local_summary(conflict, maps)?);
                active.affected_refs_json = Set(remap_conflict_affected_refs(conflict, maps)?);
                active.decision_json = Set(remap_conflict_decision_json(conflict, maps)?);
            }
        }
    );
    let obsolete_conflict_ids = package
        .source_sync_conflicts
        .iter()
        .filter(|conflict| conflict.status == SourceSyncConflictStatus::Obsolete.as_str())
        .map(|conflict| conflict.id)
        .collect::<std::collections::HashSet<_>>();
    insert_import_rows!(
        txn,
        &package.source_sync_decisions,
        decision,
        source_sync_decision::ActiveModel,
        "导入源同步决策失败",
        |active| {
            active.conflict_id = Set(map_required(
                &maps.source_sync_conflict_ids,
                decision.conflict_id,
                "同步决策 conflict_id",
            )?);
            if !obsolete_conflict_ids.contains(&decision.conflict_id) {
                active.decision_payload_json = Set(remap_source_sync_decision_payload(
                    &decision.strategy,
                    &decision.decision_payload_json,
                    maps,
                    "同步决策 decision_payload_json",
                )?);
            }
        }
    );
    Ok(())
}
