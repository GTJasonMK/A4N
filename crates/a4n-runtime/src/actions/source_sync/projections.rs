use super::{SourceSyncPlanSummary, SourceSyncRunSummary};
use a4n_core::models::source_sync::source_sync_run;
use a4n_orchestrator::source_sync::SourceSyncPlan;

pub(super) fn source_sync_plan_summary(plan: SourceSyncPlan) -> SourceSyncPlanSummary {
    SourceSyncPlanSummary {
        run: source_sync_run_summary(&plan.run),
        conflicts: plan.conflicts,
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
