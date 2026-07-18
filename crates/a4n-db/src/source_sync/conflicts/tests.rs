use super::{
    SourceSyncConflictRecordInput, SourceSyncDecisionRecordInput,
    insert_source_sync_conflict_record, insert_source_sync_decision_record,
};
use crate::source_sync::SourceSyncDbError;
use a4n_core::models::source_sync::source_sync_run;
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
        source_change_kind: "updated".to_string(),
        local_state_kind: "draft".to_string(),
        severity: "blocking".to_string(),
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

#[tokio::test]
async fn decision_insert_rejects_blank_actor_before_database_access() {
    let error = insert_source_sync_decision_record(
        &DatabaseConnection::Disconnected,
        SourceSyncDecisionRecordInput {
            conflict_id: 1,
            decided_by: "  ".to_string(),
            strategy: "keep_local".to_string(),
            decision_payload_json: json!({}),
        },
    )
    .await
    .expect_err("blank decided_by must fail before database access");

    assert!(matches!(
        error,
        SourceSyncDbError::InvalidArgument(message) if message == "decided_by 不能为空"
    ));
}
