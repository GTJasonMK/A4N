use super::context::{
    chapter_rag_cleanup_operation_trace_id, character_profile_delete_rag_operation_trace_id,
    character_profile_rollback_rag_operation_trace_id, rag_client_ingest_operation_trace_id,
    rag_ingest_operation_trace_id, scene_rag_sync_operation_trace_id, scope_rag_operation_trace,
    validate_rag_client_ingest_operation_trace_id, workflow_rag_operation_trace_id,
};
use super::events::handle_rag_ingest_event;
use super::request::rag_service_request;
use super::store::SidecarVectorStore;
use a4n_core::error::AppError;
use a4n_rag_core::vector_store::{EmbeddingRecord, VectorStore};
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, SecretForwardingPolicy, ServiceErrorCode,
};
use a4n_test_support::capability::RecordingRagClient;
use serde_json::json;
use std::sync::Arc;

use crate::rag_ingest::SceneRagSyncTraceSource;

#[test]
fn rag_service_request_extracts_nested_project_scope_without_changing_payload() {
    let payload = json!({
        "collection": "novel",
        "filters": {
            "equals": {
                "project_id": 42,
                "root_project_id": 3,
                "data_type": "chapter"
            }
        }
    });
    let request = rag_service_request("rag.query", payload.clone()).expect("rag service request");

    assert_eq!(request.project_id, Some(42));
    assert_eq!(request.root_project_id, Some(3));
    assert_eq!(request.payload, payload);
    assert_eq!(request.secret_policy, SecretForwardingPolicy::deny_all());
}

#[test]
fn rag_service_request_accepts_repeated_same_project_and_rejects_mixed_projects() {
    let request = rag_service_request(
        "rag.ingest",
        json!({
            "collection": "novel",
            "records": [
                {"metadata": {"project_id": 42, "root_project_id": 3}},
                {"metadata": {"project_id": "42", "root_project_id": "3"}}
            ]
        }),
    )
    .expect("same project batch");
    assert_eq!(request.project_id, Some(42));
    assert_eq!(request.root_project_id, Some(3));

    let error = rag_service_request(
        "rag.ingest",
        json!({
            "collection": "novel",
            "records": [
                {"metadata": {"project_id": 42}},
                {"metadata": {"project_id": 43}}
            ]
        }),
    )
    .expect_err("mixed project batch must fail");
    assert!(error.contains("多个 project_id"));
    assert!(error.contains("42"));
    assert!(error.contains("43"));

    let error = rag_service_request(
        "rag.ingest",
        json!({
            "collection": "novel",
            "records": [
                {"metadata": {"project_id": 42, "root_project_id": 3}},
                {"metadata": {"project_id": 42, "root_project_id": 4}}
            ]
        }),
    )
    .expect_err("mixed root project batch must fail");
    assert!(error.contains("多个 root_project_id"));
    assert!(error.contains("3"));
    assert!(error.contains("4"));

    let error = rag_service_request(
        "rag.query",
        json!({
            "collection": "novel",
            "filters": {"equals": {"root_project_id": 3}}
        }),
    )
    .expect_err("root project without project scope must fail");
    assert!(error.contains("缺少 project_id"));
}

#[test]
fn rag_service_request_keeps_global_operations_without_project_scope() {
    let request = rag_service_request("rag.drop_collection", json!({ "collection": "novel" }))
        .expect("global rag service request");

    assert_eq!(request.project_id, None);
    assert_eq!(request.root_project_id, None);
    assert_eq!(request.secret_policy, SecretForwardingPolicy::deny_all());
}

#[test]
fn rag_ingest_operation_trace_prefers_valid_host_task_id() {
    assert_eq!(
        rag_ingest_operation_trace_id(42, Some("rag-task-77")).expect("task trace"),
        "task:rag-task-77"
    );
    assert!(rag_ingest_operation_trace_id(42, Some("bad task")).is_err());
    assert!(rag_ingest_operation_trace_id(0, None).is_err());
    assert!(
        rag_ingest_operation_trace_id(42, None)
            .expect("generated trace")
            .starts_with("rag_ingest:42:")
    );
}

#[test]
fn rag_client_ingest_trace_round_trips_project_scope() {
    let trace_id = rag_client_ingest_operation_trace_id(42).expect("client ingest trace");
    assert!(trace_id.starts_with("rag_client_ingest:42:"));
    assert_eq!(
        validate_rag_client_ingest_operation_trace_id(42, &trace_id)
            .expect("matching client ingest trace"),
        trace_id
    );
    assert!(validate_rag_client_ingest_operation_trace_id(43, &trace_id).is_err());
    assert!(
        validate_rag_client_ingest_operation_trace_id(42, "rag_client_ingest:42:bad-run").is_err()
    );
}

#[test]
fn scene_rag_sync_trace_inherits_workflow_or_creates_independent_run() {
    assert_eq!(
        scene_rag_sync_operation_trace_id(42, SceneRagSyncTraceSource::WorkflowRun(77))
            .expect("workflow scene sync trace"),
        "workflow_run:77"
    );
    assert!(
        scene_rag_sync_operation_trace_id(42, SceneRagSyncTraceSource::Independent)
            .expect("independent scene sync trace")
            .starts_with("scene_rag_sync:42:")
    );
    assert!(
        scene_rag_sync_operation_trace_id(42, SceneRagSyncTraceSource::WorkflowRun(0)).is_err()
    );
    assert!(scene_rag_sync_operation_trace_id(0, SceneRagSyncTraceSource::Independent).is_err());
    assert!(
        scene_rag_sync_operation_trace_id(42, SceneRagSyncTraceSource::CurrentOperation).is_err()
    );
    assert!(
        chapter_rag_cleanup_operation_trace_id(42)
            .expect("chapter cleanup trace")
            .starts_with("chapter_rag_cleanup:42:")
    );
    assert!(chapter_rag_cleanup_operation_trace_id(0).is_err());
}

#[test]
fn workflow_rag_trace_requires_persisted_positive_run_id() {
    assert_eq!(
        workflow_rag_operation_trace_id(77).expect("workflow RAG trace"),
        "workflow_run:77"
    );
    assert!(workflow_rag_operation_trace_id(0).is_err());
    assert!(workflow_rag_operation_trace_id(-1).is_err());
}

#[test]
fn character_rag_mutation_traces_require_explicit_valid_scope() {
    assert!(
        character_profile_delete_rag_operation_trace_id(42, 7)
            .expect("character delete trace")
            .starts_with("character_profile_delete:42:7:")
    );
    assert!(
        character_profile_rollback_rag_operation_trace_id(42, 7, 3)
            .expect("character rollback trace")
            .starts_with("character_profile_rollback:42:7:3:")
    );
    assert!(character_profile_delete_rag_operation_trace_id(0, 7).is_err());
    assert!(character_profile_delete_rag_operation_trace_id(42, 0).is_err());
    assert!(character_profile_rollback_rag_operation_trace_id(42, 7, -1).is_err());
}

#[tokio::test]
async fn scene_rag_sync_can_inherit_active_chapter_cleanup_operation() {
    let trace_id = "chapter_rag_cleanup:42:77".to_string();
    let inherited = scope_rag_operation_trace(trace_id.clone(), async {
        scene_rag_sync_operation_trace_id(42, SceneRagSyncTraceSource::CurrentOperation)
            .expect("active chapter cleanup trace")
    })
    .await;
    assert_eq!(inherited, trace_id);
}

#[tokio::test]
async fn rag_ingest_uses_typed_events_and_preserves_project_scope() {
    let client =
        Arc::new(RecordingRagClient::new().with_ingest_events(successful_rag_ingest_events()));
    let store = SidecarVectorStore {
        client: client.clone(),
    };

    let written = scope_rag_operation_trace("task:rag-task-77".to_string(), async {
        store
            .add_embeddings(
                "novel",
                vec![EmbeddingRecord {
                    id: "chapter:42:1".to_string(),
                    embedding: vec![0.1, 0.2],
                    metadata: json!({
                        "project_id": 42,
                        "root_project_id": 3,
                        "data_type": "chapter"
                    }),
                    content: "chapter content".to_string(),
                }],
            )
            .await
    })
    .await
    .expect("rag ingest events");

    assert_eq!(written, 1);
    assert_eq!(client.ingest_events_call_count(), 1);
    let request = client
        .last_ingest_events_request()
        .expect("recorded request");
    assert_eq!(request.project_id, Some(42));
    assert_eq!(request.root_project_id, Some(3));
    assert_eq!(request.trace_context.trace_id, "task:rag-task-77");
    assert_eq!(
        request.trace_context.parent_span_id.as_deref(),
        Some("rag.ingest")
    );
    assert_eq!(request.secret_policy, SecretForwardingPolicy::deny_all());
    assert_eq!(request.payload["records"][0]["metadata"]["project_id"], 42);
}

fn successful_rag_ingest_events() -> Vec<CapabilityTaskEvent> {
    vec![
        CapabilityTaskEvent {
            task_id: "configured-task".to_string(),
            event_kind: CapabilityTaskEventKind::Progress,
            sequence: 1,
            chunk_index: None,
            chunk_delta: None,
            usage_delta: None,
            final_usage: None,
            finish_reason: None,
            is_truncated_finish_reason: None,
            checkpoint_key: None,
            artifact_refs: Vec::new(),
            error: None,
            payload: Some(json!({
                "current": 0,
                "total": 1,
                "message": "rag sidecar 正在写入向量索引"
            })),
        },
        CapabilityTaskEvent {
            task_id: "configured-task".to_string(),
            event_kind: CapabilityTaskEventKind::Completed,
            sequence: 2,
            chunk_index: None,
            chunk_delta: None,
            usage_delta: None,
            final_usage: None,
            finish_reason: None,
            is_truncated_finish_reason: None,
            checkpoint_key: None,
            artifact_refs: Vec::new(),
            error: None,
            payload: Some(json!({ "written": 1 })),
        },
    ]
}

#[test]
fn rag_ingest_terminal_events_preserve_failure_and_cancel_semantics() {
    let failed = CapabilityTaskEvent {
        task_id: "rag-1".to_string(),
        event_kind: CapabilityTaskEventKind::Failed,
        sequence: 1,
        chunk_index: None,
        chunk_delta: None,
        usage_delta: None,
        final_usage: None,
        finish_reason: None,
        is_truncated_finish_reason: None,
        checkpoint_key: None,
        artifact_refs: Vec::new(),
        error: Some(a4n_service_protocol::ServiceError {
            code: ServiceErrorCode::ProviderError,
            message: "vector index unavailable".to_string(),
            retryable: Some(false),
        }),
        payload: None,
    };
    assert!(
        handle_rag_ingest_event(failed)
            .expect_err("failed event")
            .to_string()
            .contains("vector index unavailable")
    );

    let cancel = CapabilityTaskEvent {
        task_id: "rag-1".to_string(),
        event_kind: CapabilityTaskEventKind::CancelAck,
        sequence: 2,
        chunk_index: None,
        chunk_delta: None,
        usage_delta: None,
        final_usage: None,
        finish_reason: None,
        is_truncated_finish_reason: None,
        checkpoint_key: None,
        artifact_refs: Vec::new(),
        error: None,
        payload: None,
    };
    assert!(matches!(
        handle_rag_ingest_event(cancel),
        Err(AppError::Cancelled)
    ));
}
