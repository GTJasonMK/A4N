use crate::protocol::validate_service_request;
use a4n_service_protocol::{
    PROMPT_RENDER_CAPABILITY_VERSION as PROMPT_CAPABILITY_VERSION, PROTOCOL_VERSION,
    ServiceErrorCode, ServiceRequest, ServiceResponse, ServiceStatus,
};

#[test]
fn rejects_incompatible_capability_version() {
    let mut request = service_request(serde_json::json!({}));
    request.capability_version = "prompt-render-v0".to_string();

    let error = validate_service_request(&request).expect_err("version mismatch");

    assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
    assert!(error.message.contains("prompt capability version"));
}

#[test]
fn rejects_incompatible_protocol_version_with_trace() {
    let mut request = service_request(serde_json::json!({}));
    request.schema_version.protocol = PROTOCOL_VERSION + 1;

    let response: ServiceResponse<()> =
        request.error_response(validate_service_request(&request).unwrap_err());

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
        request.error_response(validate_service_request(&request).unwrap_err());

    assert_eq!(response.status, ServiceStatus::Error);
    assert_eq!(response.trace_id.as_deref(), Some("trace-1"));
    let error = response.error.expect("secret policy error");
    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("不接收 secret forwarding policy"));
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
        capability_version: PROMPT_CAPABILITY_VERSION.to_string(),
        payload,
    }
}
