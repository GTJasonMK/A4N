use super::{SourceSyncConflictRecordInput, insert_source_sync_conflict_record};
use crate::source_sync::SourceSyncDbError;
use a4n_core::models::source_sync::source_sync_run;
use a4n_domain::source_sync::{
    SourceSyncChangeKind, SourceSyncConflictSeverity, SourceSyncLocalStateKind,
};
use chrono::Utc;
use sea_orm::DatabaseConnection;
use serde_json::json;

fn source_sync_run_model() -> source_sync_run::Model {
    let now = Utc::now().naive_utc();
    source_sync_run::Model {
        id: 1,
        derived_project_id: 2,
        root_project_id: 3,
        from_snapshot_id: 4,
        to_snapshot_id: 5,
        mode: "dry_run".to_string(),
        status: "planned".to_string(),
        summary_json: json!({}),
        error_message: None,
        created_at: now,
        updated_at: now,
    }
}

fn conflict_input() -> SourceSyncConflictRecordInput {
    SourceSyncConflictRecordInput {
        conflict_scope: "generation_spec".to_string(),
        target_kind: "segment".to_string(),
        target_id: 6,
        source_change_kind: SourceSyncChangeKind::Updated,
        local_state_kind: SourceSyncLocalStateKind::Draft,
        severity: SourceSyncConflictSeverity::Blocking,
        source_summary_json: json!({}),
        local_summary_json: json!({}),
        affected_refs_json: json!({}),
    }
}

#[tokio::test]
async fn conflict_insert_rejects_invalid_run_before_database_access() {
    let mut run = source_sync_run_model();
    run.id = 0;

    let error = insert_source_sync_conflict_record(
        &DatabaseConnection::Disconnected,
        &run,
        conflict_input(),
    )
    .await
    .expect_err("invalid sync run id must fail before database access");

    assert!(matches!(
        error,
        SourceSyncDbError::InvalidArgument(message) if message == "sync_run_id 必须为正数"
    ));
}
