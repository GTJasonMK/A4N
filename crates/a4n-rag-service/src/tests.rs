use crate::contracts::IngestOutput;
use crate::handlers::{completed_ingest_event, failed_ingest_event, progress_ingest_event};
use crate::protocol::{error_response, service_error, validate_service_request};
use a4n_service_protocol::{
    PROTOCOL_VERSION, RAG_INDEX_CAPABILITY_VERSION as RAG_CAPABILITY_VERSION, ServiceErrorCode,
    ServiceRequest, ServiceResponse, ServiceStatus, ServiceTaskEventKind,
};

#[test]
fn rejects_incompatible_capability_version() {
    let mut request = service_request(serde_json::json!({}));
    request.capability_version = "rag-index-v0".to_string();

    let error = validate_service_request(&request).expect_err("version mismatch");

    assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
    assert!(error.message.contains("rag capability version"));
}

#[test]
fn rejects_incompatible_protocol_version_with_trace() {
    let mut request = service_request(serde_json::json!({}));
    request.schema_version.protocol = PROTOCOL_VERSION + 1;

    let response: ServiceResponse<()> =
        error_response(&request, validate_service_request(&request).unwrap_err());

    assert_eq!(response.status, ServiceStatus::Error);
    assert_eq!(response.trace_id.as_deref(), Some("trace-1"));
    let error = response.error.expect("version mismatch error");
    assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
    assert!(error.message.contains("unsupported protocol version"));
}

#[test]
fn rejects_secret_forwarding_policy() {
    let mut request = service_request(serde_json::json!({}));
    request.secret_policy = a4n_service_protocol::SecretForwardingPolicy::local_user_secret();

    let response: ServiceResponse<()> =
        error_response(&request, validate_service_request(&request).unwrap_err());

    assert_eq!(response.status, ServiceStatus::Error);
    assert_eq!(response.trace_id.as_deref(), Some("trace-1"));
    let error = response.error.expect("secret policy error");
    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("不接收 secret forwarding policy"));
}

#[test]
fn rag_ingest_events_distinguish_progress_completion_and_failure() {
    let progress = progress_ingest_event("request-1".to_string(), 1, 3);
    assert_eq!(progress.event_kind, ServiceTaskEventKind::Progress);
    assert_eq!(progress.payload.as_ref().unwrap()["total"], 3);
    assert_eq!(
        progress.payload.as_ref().unwrap()["message"],
        "rag sidecar 正在写入向量索引"
    );

    let completed = completed_ingest_event("request-1".to_string(), 2, IngestOutput { written: 3 })
        .expect("completed event");
    assert_eq!(completed.event_kind, ServiceTaskEventKind::Completed);
    assert_eq!(completed.payload.as_ref().unwrap()["written"], 3);
    assert!(completed.artifact_refs.is_empty());

    let failed = failed_ingest_event(
        "request-1".to_string(),
        2,
        service_error(&a4n_core::error::AppError::VectorStoreError(
            "Authorization: Bearer private-token".to_string(),
        )),
    );
    assert_eq!(failed.event_kind, ServiceTaskEventKind::Failed);
    let error = failed.error.expect("failed event error");
    assert_eq!(error.code, ServiceErrorCode::ProviderError);
    assert!(!error.message.contains("private-token"));
}

fn service_request<T>(payload: T) -> ServiceRequest<T> {
    ServiceRequest {
        request_id: "request-1".to_string(),
        trace_context: a4n_service_protocol::TraceContext {
            trace_id: "trace-1".to_string(),
            parent_span_id: None,
        },
        schema_version: a4n_service_protocol::SchemaVersion::default(),
        user_id: None,
        project_id: None,
        root_project_id: None,
        runtime_config_snapshot: None,
        secret_policy: a4n_service_protocol::SecretForwardingPolicy::deny_all(),
        capability_version: RAG_CAPABILITY_VERSION.to_string(),
        payload,
    }
}
