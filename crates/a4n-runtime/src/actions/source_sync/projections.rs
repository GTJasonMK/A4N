use super::{SourceSyncConflictSummary, SourceSyncPlanSummary, SourceSyncRunSummary};
use a4n_core::error::AppError;
use a4n_core::models::source_sync::{source_sync_conflict, source_sync_run};
use a4n_db::source_sync::SourceSyncPlanRecord;
use a4n_domain::source_sync::{
    SourceSyncChangeKind, SourceSyncConflictSeverity, SourceSyncConflictStatus,
    SourceSyncLocalStateKind, SourceSyncResolutionStrategy, SourceSyncRunMode, SourceSyncRunStatus,
};
use serde_json::{Map, Value};

pub(super) fn source_sync_plan_summary(
    plan: SourceSyncPlanRecord,
) -> Result<SourceSyncPlanSummary, AppError> {
    Ok(SourceSyncPlanSummary {
        run: source_sync_run_summary(plan.run)?,
        conflicts: plan
            .conflicts
            .into_iter()
            .map(source_sync_conflict_summary)
            .collect::<Result<_, _>>()?,
    })
}

fn source_sync_conflict_summary(
    conflict: source_sync_conflict::Model,
) -> Result<SourceSyncConflictSummary, AppError> {
    let source_change_kind =
        SourceSyncChangeKind::parse(&conflict.source_change_kind).ok_or_else(|| {
            invalid_source_sync_value("source_change_kind", &conflict.source_change_kind)
        })?;
    let local_state_kind = SourceSyncLocalStateKind::parse(&conflict.local_state_kind)
        .ok_or_else(|| invalid_source_sync_value("local_state_kind", &conflict.local_state_kind))?;
    let severity = SourceSyncConflictSeverity::parse(&conflict.severity)
        .ok_or_else(|| invalid_source_sync_value("severity", &conflict.severity))?;
    let status = SourceSyncConflictStatus::parse(&conflict.status)
        .ok_or_else(|| invalid_source_sync_value("conflict_status", &conflict.status))?;
    let resolution_strategy = conflict
        .resolution_strategy
        .as_deref()
        .map(|value| {
            SourceSyncResolutionStrategy::parse(value)
                .ok_or_else(|| invalid_source_sync_value("resolution_strategy", value))
        })
        .transpose()?;
    Ok(SourceSyncConflictSummary {
        id: conflict.id,
        sync_run_id: conflict.sync_run_id,
        derived_project_id: conflict.derived_project_id,
        root_project_id: conflict.root_project_id,
        from_snapshot_id: conflict.from_snapshot_id,
        to_snapshot_id: conflict.to_snapshot_id,
        conflict_scope: conflict.conflict_scope,
        target_kind: conflict.target_kind,
        target_id: conflict.target_id,
        source_change_kind,
        local_state_kind,
        severity,
        status,
        resolution_strategy,
        source_summary_json: json_object(conflict.source_summary_json, "source_summary_json")?,
        local_summary_json: json_object(conflict.local_summary_json, "local_summary_json")?,
        affected_refs_json: json_object(conflict.affected_refs_json, "affected_refs_json")?,
        decision_json: conflict
            .decision_json
            .map(|value| json_object(value, "decision_json"))
            .transpose()?,
        created_at: conflict
            .created_at
            .format("%Y-%m-%dT%H:%M:%S%.f")
            .to_string(),
        resolved_at: conflict
            .resolved_at
            .map(|value| value.format("%Y-%m-%dT%H:%M:%S%.f").to_string()),
    })
}

pub(super) fn source_sync_run_summary(
    run: source_sync_run::Model,
) -> Result<SourceSyncRunSummary, AppError> {
    let mode = SourceSyncRunMode::parse(&run.mode)
        .ok_or_else(|| invalid_source_sync_value("run_mode", &run.mode))?;
    let status = SourceSyncRunStatus::parse(&run.status)
        .ok_or_else(|| invalid_source_sync_value("run_status", &run.status))?;
    Ok(SourceSyncRunSummary {
        id: run.id,
        derived_project_id: run.derived_project_id,
        root_project_id: run.root_project_id,
        from_snapshot_id: run.from_snapshot_id,
        to_snapshot_id: run.to_snapshot_id,
        mode,
        status,
        summary_json: json_object(run.summary_json, "summary_json")?,
        error_message: run.error_message,
        created_at: run.created_at.to_string(),
        updated_at: run.updated_at.to_string(),
    })
}

fn json_object(value: Value, field: &str) -> Result<Map<String, Value>, AppError> {
    match value {
        Value::Object(value) => Ok(value),
        _ => Err(invalid_source_sync_value(field, "non_object_json")),
    }
}

fn invalid_source_sync_value(field: &str, value: &str) -> AppError {
    AppError::InvalidState {
        current: format!("{field}={value}"),
        operation: "serialize_source_sync_plan".to_string(),
    }
}
