use crate::protocol::service_error;
use a4n_service_protocol::{
    PROTOCOL_VERSION, RAG_INDEX_CAPABILITY_VERSION as RAG_CAPABILITY_VERSION, RAG_SERVICE_NAME,
    ServiceErrorCode, ServiceRequest, ServiceResponse, ServiceStatus,
};

#[test]
fn rejects_incompatible_capability_version() {
    let mut request = service_request(serde_json::json!({}));
    request.capability_version = "rag-index-v0".to_string();

    let error = request
        .validate_known_service(RAG_SERVICE_NAME)
        .expect_err("version mismatch");

    assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
    assert!(error.message.contains("rag capability version"));
}

#[test]
fn rejects_incompatible_protocol_version_with_trace() {
    let mut request = service_request(serde_json::json!({}));
    request.schema_version.protocol = PROTOCOL_VERSION + 1;

    let response: ServiceResponse<()> = request.error_response(
        request
            .validate_known_service(RAG_SERVICE_NAME)
            .unwrap_err(),
    );

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

    let response: ServiceResponse<()> = request.error_response(
        request
            .validate_known_service(RAG_SERVICE_NAME)
            .unwrap_err(),
    );

    assert_eq!(response.status, ServiceStatus::Error);
    assert_eq!(response.trace_id.as_deref(), Some("trace-1"));
    let error = response.error.expect("secret policy error");
    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("不接收 secret forwarding policy"));
}

#[test]
fn rag_service_error_redacts_provider_secrets() {
    let error = service_error(a4n_core::error::AppError::VectorStoreError(
        "Authorization: Bearer private-token".to_string(),
    ));
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
