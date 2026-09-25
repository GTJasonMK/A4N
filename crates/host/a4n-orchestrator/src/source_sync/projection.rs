use a4n_api_contract::source_sync::{
    SourceSyncConflictSummary, SourceSyncPlanSummary, SourceSyncRunSummary,
};
use a4n_db::source_sync::SourceSyncPlanRecord;
use a4n_db_models::models::source_sync::{source_sync_conflict, source_sync_run};
use a4n_domain::source_sync::{
    SourceSyncChangeKind, SourceSyncConflictSeverity, SourceSyncConflictStatus,
    SourceSyncLocalStateKind, SourceSyncResolutionStrategy, SourceSyncRunMode, SourceSyncRunStatus,
};
use a4n_error::AppError;
use serde_json::{Map, Value};

pub(crate) fn source_sync_plan_summary(
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
        created_at: conflict.created_at.and_utc().to_rfc3339(),
        resolved_at: conflict
            .resolved_at
            .map(|value| value.and_utc().to_rfc3339()),
    })
}

fn source_sync_run_summary(run: source_sync_run::Model) -> Result<SourceSyncRunSummary, AppError> {
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
        created_at: run.created_at.and_utc().to_rfc3339(),
        updated_at: run.updated_at.and_utc().to_rfc3339(),
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

#[cfg(test)]
mod tests {
    use super::source_sync_run_summary;
    use a4n_db_models::models::source_sync::source_sync_run;
    use a4n_domain::source_sync::{SourceSyncRunMode, SourceSyncRunStatus};
    use chrono::NaiveDate;
    use serde_json::json;

    #[test]
    fn source_sync_run_summary_preserves_public_contract_fields() {
        let created_at = NaiveDate::from_ymd_opt(2026, 6, 1)
            .unwrap()
            .and_hms_opt(2, 3, 4)
            .unwrap();
        let updated_at = NaiveDate::from_ymd_opt(2026, 6, 2)
            .unwrap()
            .and_hms_opt(3, 4, 5)
            .unwrap();
        let run = source_sync_run::Model {
            id: 1,
            derived_project_id: 2,
            root_project_id: 3,
            from_snapshot_id: 4,
            to_snapshot_id: 5,
            mode: "dry_run".to_string(),
            status: "planned".to_string(),
            summary_json: json!({"conflicts": 6}),
            error_message: Some("err".to_string()),
            created_at,
            updated_at,
        };

        let summary = source_sync_run_summary(run).expect("valid source sync run should serialize");

        assert_eq!(summary.id, 1);
        assert_eq!(summary.derived_project_id, 2);
        assert_eq!(summary.root_project_id, 3);
        assert_eq!(summary.from_snapshot_id, 4);
        assert_eq!(summary.to_snapshot_id, 5);
        assert_eq!(summary.mode, SourceSyncRunMode::DryRun);
        assert_eq!(summary.status, SourceSyncRunStatus::Planned);
        assert_eq!(summary.summary_json.get("conflicts"), Some(&json!(6)));
        assert_eq!(summary.error_message.as_deref(), Some("err"));
        assert_eq!(summary.created_at, "2026-06-01T02:03:04+00:00");
        assert_eq!(summary.updated_at, "2026-06-02T03:04:05+00:00");
    }
}
