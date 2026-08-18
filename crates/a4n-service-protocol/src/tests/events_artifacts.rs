use serde_json::json;

use crate::{
    ArtifactLocation, ArtifactOwnership, ArtifactRef, CapabilityTaskEvent, CapabilityTaskEventKind,
    ServiceError, ServiceErrorCode, ServiceResponse, ServiceStatus, sanitize_capability_task_event,
    sanitize_service_response, validate_artifact_refs,
};

#[test]
fn capability_task_event_kind_marks_only_stream_terminal_events() {
    assert!(!CapabilityTaskEventKind::Progress.is_terminal());
    assert!(!CapabilityTaskEventKind::Chunk.is_terminal());
    assert!(CapabilityTaskEventKind::Completed.is_terminal());
    assert!(CapabilityTaskEventKind::Failed.is_terminal());
}

#[test]
fn validate_artifact_refs_rejects_remote_local_path() {
    let artifact = ArtifactRef {
        artifact_id: "artifact-1".to_string(),
        artifact_kind: "package".to_string(),
        owner_user_id: Some(10),
        project_id: Some(20),
        content_type: "application/zip".to_string(),
        sha256: "abc".to_string(),
        byte_size: 42,
        location: ArtifactLocation::LocalPath("/tmp/package.zip".to_string()),
        expires_at: None,
        ownership: ArtifactOwnership::Remote,
    };

    let err = validate_artifact_refs(&[artifact]).unwrap_err();

    assert!(err.to_string().contains("remote sidecar artifact"));
    assert!(err.to_string().contains("artifact-1"));
}

#[test]
fn validate_artifact_refs_allows_sidecar_local_path() {
    let artifact = ArtifactRef {
        artifact_id: "artifact-1".to_string(),
        artifact_kind: "package".to_string(),
        owner_user_id: Some(10),
        project_id: Some(20),
        content_type: "application/zip".to_string(),
        sha256: "abc".to_string(),
        byte_size: 42,
        location: ArtifactLocation::LocalPath("/tmp/package.zip".to_string()),
        expires_at: None,
        ownership: ArtifactOwnership::Sidecar,
    };

    validate_artifact_refs(&[artifact]).unwrap();
}

#[test]
fn sanitize_capability_task_event_redacts_error_message() {
    let mut event = CapabilityTaskEvent::failed(
        "task-1".to_string(),
        1,
        ServiceError {
            code: ServiceErrorCode::ProviderError,
            message: "provider failed: base_url=https://provider.example api_key=sk-test"
                .to_string(),
            retryable: Some(false),
        },
    );

    sanitize_capability_task_event(&mut event);

    let message = &event.error.unwrap().message;
    assert!(message.contains("base_url=[REDACTED]"));
    assert!(message.contains("api_key=[REDACTED]"));
    assert!(!message.contains("provider.example"));
    assert!(!message.contains("sk-test"));
}

#[test]
fn sanitize_service_response_rejects_remote_local_path() {
    let mut response = ServiceResponse::<serde_json::Value> {
        status: ServiceStatus::Ok,
        trace_id: Some("trace-1".to_string()),
        usage: None,
        artifact_refs: vec![ArtifactRef {
            artifact_id: "artifact-1".to_string(),
            artifact_kind: "package".to_string(),
            owner_user_id: None,
            project_id: None,
            content_type: "application/zip".to_string(),
            sha256: "abc".to_string(),
            byte_size: 10,
            location: ArtifactLocation::LocalPath("/tmp/package.zip".to_string()),
            expires_at: None,
            ownership: ArtifactOwnership::Remote,
        }],
        payload: Some(json!({"ok": true})),
        error: None,
    };

    let err = sanitize_service_response(&mut response).unwrap_err();

    assert!(err.to_string().contains("remote sidecar artifact"));
}

#[test]
fn sanitize_service_response_redacts_error_message() {
    let mut response = ServiceResponse::<serde_json::Value> {
        status: ServiceStatus::Error,
        trace_id: Some("trace-1".to_string()),
        usage: None,
        artifact_refs: Vec::new(),
        payload: None,
        error: Some(ServiceError {
            code: ServiceErrorCode::ProviderError,
            message: "provider failed: https://provider.example/v1 token=secret-token".to_string(),
            retryable: Some(false),
        }),
    };

    sanitize_service_response(&mut response).unwrap();

    let message = &response.error.unwrap().message;
    assert!(message.contains("[REDACTED_URL]"));
    assert!(message.contains("token=[REDACTED]"));
    assert!(!message.contains("provider.example"));
    assert!(!message.contains("secret-token"));
}
