use a4n_image::ImageGenerationRequest;
use a4n_model_discovery::ModelDiscoveryRequest;
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind,
    IMAGE_PROVIDER_CAPABILITY_VERSION as IMAGE_CAPABILITY_VERSION, PROTOCOL_VERSION, ServiceError,
    ServiceErrorCode, ServiceRequest, ServiceResponse, ServiceStatus,
};

use crate::contracts::{ImageGenerationOutput, ImageProviderPayload};
use crate::handlers::events::{
    completed_generation_event, event_line, failed_generation_event, progress_generation_event,
};
use crate::protocol::{error_response, validate_image_request, validate_model_discovery_request};

#[test]
fn image_generation_events_keep_terminal_payload_and_error_contracts() {
    let progress = progress_generation_event("task-1".to_string(), 1);
    assert_eq!(progress.event_kind, CapabilityTaskEventKind::Progress);
    assert_eq!(
        progress.payload.as_ref().unwrap()["message"],
        "image sidecar 正在调用 provider"
    );
    let completed = completed_generation_event(
        "task-1".to_string(),
        2,
        ImageGenerationOutput {
            bytes_base64: "aW1n".to_string(),
            format: "png".to_string(),
            model: "image-model".to_string(),
        },
    )
    .expect("completed event");
    assert_eq!(completed.event_kind, CapabilityTaskEventKind::Completed);
    assert_eq!(completed.payload.as_ref().unwrap()["format"], "png");
    assert!(completed.error.is_none());
    let failed = failed_generation_event(
        "task-1".to_string(),
        2,
        ServiceError {
            code: ServiceErrorCode::ProviderError,
            message: "provider failed".to_string(),
            retryable: Some(false),
        },
    );
    assert_eq!(failed.event_kind, CapabilityTaskEventKind::Failed);
    assert_eq!(failed.error.unwrap().code, ServiceErrorCode::ProviderError);
    assert!(failed.payload.is_none());
}

#[test]
fn image_generation_event_line_is_newline_delimited_json() {
    let line = event_line(progress_generation_event("task-1".to_string(), 1));
    assert_eq!(line.last(), Some(&b'\n'));
    let event: CapabilityTaskEvent =
        serde_json::from_slice(&line[..line.len() - 1]).expect("event json");
    assert_eq!(event.task_id, "task-1");
    assert_eq!(event.event_kind, CapabilityTaskEventKind::Progress);
}

#[test]
fn rejects_incompatible_capability_version() {
    let mut request = service_request(image_payload(""), false);
    request.capability_version = "image-provider-v0".to_string();
    let error = validate_image_request(&request).expect_err("version mismatch");
    assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
    assert!(error.message.contains("image capability version"));
}

#[test]
fn rejects_incompatible_protocol_version_with_trace() {
    let mut request = service_request(image_payload(""), false);
    request.schema_version.protocol = PROTOCOL_VERSION + 1;
    let response: ServiceResponse<()> =
        error_response(&request, validate_image_request(&request).unwrap_err());
    assert_eq!(response.status, ServiceStatus::Error);
    assert_eq!(response.trace_id.as_deref(), Some("trace-1"));
    let error = response.error.expect("version mismatch error");
    assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
    assert!(error.message.contains("unsupported protocol version"));
}

#[test]
fn rejects_api_key_when_user_secret_forwarding_is_denied() {
    let error = validate_image_request(&service_request(image_payload("sk-test"), false))
        .expect_err("user secret denied");
    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("allow_user_secret"));
}

#[test]
fn rejects_api_key_when_secret_policy_is_trusted_remote() {
    let request = service_request_with_policy(
        image_payload("sk-test"),
        trusted_remote_user_secret_policy(),
    );
    let error = validate_image_request(&request).expect_err("remote user secret denied");
    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("local_sidecar_only"));
}

#[test]
fn rejects_model_discovery_api_key_when_user_secret_forwarding_is_denied() {
    let request = service_request(model_discovery_payload("sk-test"), false);
    let error =
        validate_model_discovery_request(&request).expect_err("model discovery user secret denied");
    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("allow_user_secret"));
}

#[test]
fn rejects_model_discovery_api_key_when_secret_policy_is_trusted_remote() {
    let request = service_request_with_policy(
        model_discovery_payload("sk-test"),
        trusted_remote_user_secret_policy(),
    );
    let error = validate_model_discovery_request(&request)
        .expect_err("model discovery remote user secret denied");
    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("local_sidecar_only"));
}

#[test]
fn allows_api_key_when_user_secret_forwarding_is_allowed() {
    validate_image_request(&service_request(image_payload("sk-test"), true)).unwrap();
}

#[test]
fn allows_blank_api_key_when_user_secret_forwarding_is_denied() {
    validate_image_request(&service_request(image_payload("  "), false)).unwrap();
}

fn service_request<T>(payload: T, allow_user_secret: bool) -> ServiceRequest<T> {
    service_request_with_policy(payload, local_secret_policy(allow_user_secret))
}

fn service_request_with_policy<T>(
    payload: T,
    secret_policy: a4n_service_protocol::SecretForwardingPolicy,
) -> ServiceRequest<T> {
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
        secret_policy,
        capability_version: IMAGE_CAPABILITY_VERSION.to_string(),
        payload,
    }
}

fn local_secret_policy(allow_user_secret: bool) -> a4n_service_protocol::SecretForwardingPolicy {
    a4n_service_protocol::SecretForwardingPolicy {
        scope: a4n_service_protocol::SecretMaterialScope::LocalSidecarOnly,
        allow_system_secret: false,
        allow_user_secret,
        allow_upload_token: false,
    }
}

fn trusted_remote_user_secret_policy() -> a4n_service_protocol::SecretForwardingPolicy {
    a4n_service_protocol::SecretForwardingPolicy {
        scope: a4n_service_protocol::SecretMaterialScope::TrustedRemote,
        allow_system_secret: false,
        allow_user_secret: true,
        allow_upload_token: false,
    }
}

fn image_payload(api_key: &str) -> ImageProviderPayload {
    ImageProviderPayload {
        backend: "openai-compatible-image".to_string(),
        base_url: "https://provider.example/v1".to_string(),
        api_key: api_key.to_string(),
        request: ImageGenerationRequest {
            prompt: "test prompt".to_string(),
            width: 512,
            height: 512,
            model: "test-model".to_string(),
            reference_images: Vec::new(),
        },
    }
}

fn model_discovery_payload(api_key: &str) -> ModelDiscoveryRequest {
    ModelDiscoveryRequest {
        provider: "openai".to_string(),
        base_url: "https://provider.example/v1".to_string(),
        api_key: Some(api_key.to_string()),
    }
}
