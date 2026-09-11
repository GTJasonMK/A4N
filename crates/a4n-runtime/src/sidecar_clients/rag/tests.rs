use super::context::{
    chapter_rag_cleanup_operation_trace_id, character_profile_delete_rag_operation_trace_id,
    character_profile_rollback_rag_operation_trace_id, rag_client_ingest_operation_trace_id,
    rag_ingest_operation_trace_id, scene_rag_sync_operation_trace_id, scope_rag_operation_trace,
    validate_rag_client_ingest_operation_trace_id, workflow_rag_operation_trace_id,
};
use super::events::handle_rag_ingest_event;
use super::request::rag_service_request;
use super::store::SidecarVectorStore;
use a4n_capabilities::A4nCapabilities;
use a4n_error::AppError;
use a4n_rag_core::vector_store::{EmbeddingRecord, VectorStore};
use a4n_service_protocol::{CapabilityTaskEvent, SecretForwardingPolicy, ServiceErrorCode};
use a4n_test_support::capability::RecordingRagClient;
use serde_json::json;
use std::sync::Arc;

use crate::rag_ingest::SceneRagSyncTraceSource;

#[test]
fn sidecar_rag_requires_capability_without_disabled_fallback() {
    let Err(error) = super::store::create_sidecar_rag_store(&A4nCapabilities::default()) else {
        panic!("missing RAG sidecar capability must fail at startup");
    };

    assert!(error.to_string().contains("RAG sidecar capability 未配置"));
}

#[test]
fn rag_service_request_uses_numeric_collection_scope_without_changing_payload() {
    let payload = json!({
        "collection": "42",
        "filters": {
            "equals": {
                "data_type": "chapter"
            }
        }
    });
    let request = rag_service_request("rag.query", payload.clone()).expect("rag service request");

    assert_eq!(request.project_id, Some(42));
    assert_eq!(request.root_project_id, Some(42));
    assert_eq!(request.payload, payload);
    assert_eq!(request.secret_policy, SecretForwardingPolicy::deny_all());
}

#[test]
fn rag_service_request_rejects_non_project_collection() {
    let error = rag_service_request("rag.drop_collection", json!({ "collection": "novel" }))
        .expect_err("non-project collection must fail");

    assert!(error.contains("正整数项目 ID"));
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
                "42",
                vec![EmbeddingRecord {
                    id: "chapter:42:1".to_string(),
                    embedding: vec![0.1, 0.2],
                    metadata: json!({
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
    assert_eq!(request.root_project_id, Some(42));
    assert_eq!(request.trace_context.trace_id, "task:rag-task-77");
    assert_eq!(
        request.trace_context.parent_span_id.as_deref(),
        Some("rag.ingest")
    );
    assert_eq!(request.secret_policy, SecretForwardingPolicy::deny_all());
    assert!(
        request.payload["records"][0]["metadata"]
            .get("project_id")
            .is_none()
    );
}

fn successful_rag_ingest_events() -> Vec<CapabilityTaskEvent> {
    vec![
        CapabilityTaskEvent::progress_message(
            "configured-task".to_string(),
            1,
            "rag sidecar 正在写入向量索引",
        ),
        CapabilityTaskEvent::completed_or_failed(
            "configured-task".to_string(),
            2,
            1,
            "test rag output serialization failed",
        ),
    ]
}

#[test]
fn rag_ingest_terminal_events_preserve_failure_and_cancel_semantics() {
    let failed = CapabilityTaskEvent::failed(
        "rag-1".to_string(),
        1,
        a4n_service_protocol::ServiceError {
            code: ServiceErrorCode::ProviderError,
            message: "vector index unavailable".to_string(),
            retryable: Some(false),
        },
    );
    assert!(
        handle_rag_ingest_event(failed)
            .expect_err("failed event")
            .to_string()
            .contains("vector index unavailable")
    );

    let cancel = CapabilityTaskEvent::failed(
        "rag-1".to_string(),
        2,
        a4n_service_protocol::ServiceError {
            code: ServiceErrorCode::Cancelled,
            message: "cancelled".to_string(),
            retryable: Some(false),
        },
    );
    assert!(matches!(
        handle_rag_ingest_event(cancel),
        Err(AppError::Cancelled)
    ));
}
