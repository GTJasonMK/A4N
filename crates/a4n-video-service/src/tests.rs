use crate::contracts::{VideoPollPayload, VideoProviderPayload};
use crate::protocol::{
    validate_video_poll_request, validate_video_provider_request,
};
use a4n_service_protocol::{
    PROTOCOL_VERSION, ServiceErrorCode, ServiceRequest, ServiceResponse, ServiceStatus,
    VIDEO_PROVIDER_CAPABILITY_VERSION as VIDEO_CAPABILITY_VERSION,
};
use a4n_video::VideoGenerationRequest;
use serde_json::json;

#[test]
fn rejects_incompatible_capability_version() {
    let mut request = provider_request(video_payload(""), false);
    request.capability_version = "video-provider-v0".to_string();

    let error = validate_video_provider_request(&request).expect_err("version mismatch");

    assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
    assert!(error.message.contains("video capability version"));
}

#[test]
fn rejects_incompatible_protocol_version_with_trace() {
    let mut request = provider_request(video_payload(""), false);
    request.schema_version.protocol = PROTOCOL_VERSION + 1;

    let response: ServiceResponse<()> =
        request.error_response(validate_video_provider_request(&request).unwrap_err());

    assert_eq!(response.status, ServiceStatus::Error);
    assert_eq!(response.trace_id.as_deref(), Some("trace-1"));
    let error = response.error.expect("version mismatch error");
    assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
    assert!(error.message.contains("unsupported protocol version"));
}

#[test]
fn rejects_generate_api_key_when_user_secret_forwarding_is_denied() {
    let request = provider_request(video_payload("sk-test"), false);

    let error = validate_video_provider_request(&request).expect_err("user secret denied");

    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("allow_user_secret"));
}

#[test]
fn rejects_poll_api_key_when_user_secret_forwarding_is_denied() {
    let request = poll_request(video_poll_payload("sk-test"), false);

    let error = validate_video_poll_request(&request).expect_err("user secret denied");

    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("allow_user_secret"));
}

#[test]
fn rejects_generate_api_key_when_secret_policy_is_trusted_remote() {
    let request = service_request_with_policy(
        video_payload("sk-test"),
        trusted_remote_user_secret_policy(),
    );

    let error = validate_video_provider_request(&request).expect_err("remote user secret denied");

    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("local_sidecar_only"));
}

#[test]
fn rejects_poll_api_key_when_secret_policy_is_trusted_remote() {
    let request = service_request_with_policy(
        video_poll_payload("sk-test"),
        trusted_remote_user_secret_policy(),
    );

    let error = validate_video_poll_request(&request).expect_err("remote user secret denied");

    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("local_sidecar_only"));
}

#[test]
fn allows_api_key_when_user_secret_forwarding_is_allowed() {
    let request = provider_request(video_payload("sk-test"), true);

    validate_video_provider_request(&request).unwrap();
}

#[test]
fn allows_blank_api_key_when_user_secret_forwarding_is_denied() {
    let request = provider_request(video_payload("  "), false);

    validate_video_provider_request(&request).unwrap();
}

#[test]
fn rejects_nonpositive_video_dimensions_at_service_boundary() {
    let mut payload = video_payload("");
    payload.request.width = 0;
    let request = provider_request(payload, false);

    let error = validate_video_provider_request(&request).expect_err("invalid dimensions");

    assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
    assert!(error.message.contains("宽度和高度必须为正整数"));
}

#[test]
fn validates_generate_project_scope_against_request_envelope() {
    let mut request = provider_request(video_payload(""), false);
    request.project_id = Some(42);
    request.payload.project_id = Some(42);

    validate_video_provider_request(&request).expect("matching project scope");

    request.payload.project_id = Some(41);
    let error = validate_video_provider_request(&request).expect_err("mismatched project scope");
    assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
    assert!(error.message.contains("generation payload project_id"));
}

fn provider_request(
    payload: VideoProviderPayload,
    allow_user_secret: bool,
) -> ServiceRequest<VideoProviderPayload> {
    service_request(payload, allow_user_secret)
}

fn poll_request(
    payload: VideoPollPayload,
    allow_user_secret: bool,
) -> ServiceRequest<VideoPollPayload> {
    service_request(payload, allow_user_secret)
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
        capability_version: VIDEO_CAPABILITY_VERSION.to_string(),
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

fn video_payload(api_key: &str) -> VideoProviderPayload {
    VideoProviderPayload {
        backend: "openai-compatible-video".to_string(),
        base_url: "https://provider.example/v1".to_string(),
        api_key: api_key.to_string(),
        provider_idempotency_key_sha256: None,
        project_id: None,
        request: VideoGenerationRequest {
            prompt: "test prompt".to_string(),
            negative_prompt: None,
            width: 720,
            height: 1280,
            duration_secs: 5,
            fps: 24,
            model: "test-model".to_string(),
            media: a4n_video::VideoMediaInput::None,
        },
    }
}

fn video_poll_payload(api_key: &str) -> VideoPollPayload {
    VideoPollPayload {
        backend: "openai-compatible-video".to_string(),
        base_url: "https://provider.example/v1".to_string(),
        api_key: api_key.to_string(),
        project_id: None,
        job_id: "job-1".to_string(),
    }
}
