use super::{SourceSyncConflictSummary, SourceSyncPlanSummary, SourceSyncRunSummary};
use a4n_core::models::source_sync::{source_sync_conflict, source_sync_run};
use a4n_orchestrator::source_sync::SourceSyncPlan;

pub(super) fn source_sync_plan_summary(plan: SourceSyncPlan) -> SourceSyncPlanSummary {
    SourceSyncPlanSummary {
        run: source_sync_run_summary(&plan.run),
        conflicts: plan
            .conflicts
            .iter()
            .map(source_sync_conflict_summary)
            .collect(),
    }
}

fn source_sync_conflict_summary(
    conflict: &source_sync_conflict::Model,
) -> SourceSyncConflictSummary {
    SourceSyncConflictSummary {
        id: conflict.id,
        sync_run_id: conflict.sync_run_id,
        derived_project_id: conflict.derived_project_id,
        root_project_id: conflict.root_project_id,
        from_snapshot_id: conflict.from_snapshot_id,
        to_snapshot_id: conflict.to_snapshot_id,
        conflict_scope: conflict.conflict_scope.clone(),
        target_kind: conflict.target_kind.clone(),
        target_id: conflict.target_id,
        source_change_kind: conflict.source_change_kind.clone(),
        local_state_kind: conflict.local_state_kind.clone(),
        severity: conflict.severity.clone(),
        status: conflict.status.clone(),
        resolution_strategy: conflict.resolution_strategy.clone(),
        source_summary_json: conflict.source_summary_json.clone(),
        local_summary_json: conflict.local_summary_json.clone(),
        affected_refs_json: conflict.affected_refs_json.clone(),
        decision_json: conflict.decision_json.clone(),
        created_at: conflict
            .created_at
            .format("%Y-%m-%dT%H:%M:%S%.f")
            .to_string(),
        resolved_at: conflict
            .resolved_at
            .map(|value| value.format("%Y-%m-%dT%H:%M:%S%.f").to_string()),
    }
}

pub(super) fn source_sync_run_summary(run: &source_sync_run::Model) -> SourceSyncRunSummary {
    SourceSyncRunSummary {
        id: run.id,
        derived_project_id: run.derived_project_id,
        root_project_id: run.root_project_id,
        from_snapshot_id: run.from_snapshot_id,
        to_snapshot_id: run.to_snapshot_id,
        mode: run.mode.clone(),
        status: run.status.clone(),
        summary_json: run.summary_json.clone(),
        error_message: run.error_message.clone(),
        created_at: run.created_at.to_string(),
        updated_at: run.updated_at.to_string(),
    }
}
