use super::*;
use a4n_capabilities::JsonResponse;
use a4n_service_protocol::{ServiceResponse, ServiceStatus};
use a4n_test_support::capability::RecordingVideoClient;
use serde_json::json;
use std::sync::Arc;

fn fixed_time() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

fn video_config() -> video_gen_config::Model {
    let now = fixed_time();
    video_gen_config::Model {
        id: 1,
        name: "test-video".to_string(),
        backend: "openai-compatible-video".to_string(),
        base_url: Some("https://example.invalid/v1".to_string()),
        api_key_encrypted: Some("test-key".to_string()),
        model: "video-model".to_string(),
        max_concurrent: 1,
        default_width: 1280,
        default_height: 720,
        default_duration_secs: 5,
        min_duration_secs: 5,
        max_duration_secs: 10,
        duration_mode: "range".to_string(),
        supported_durations_json: None,
        default_fps: 24,
        is_default: true,
        created_at: now,
        updated_at: now,
    }
}

fn video_request() -> VideoGenerationRequest {
    VideoGenerationRequest {
        prompt: "test video".to_string(),
        negative_prompt: None,
        width: 1280,
        height: 720,
        duration_secs: 5,
        fps: 24,
        model: "video-model".to_string(),
        media: a4n_video::VideoMediaInput::None,
    }
}

#[tokio::test]
async fn video_sidecar_missing_capability_fails_without_embedded_fallback() {
    let error = generate_video(
        RuntimeProfile::Sidecar,
        &A4nCapabilities::empty(),
        &video_config(),
        video_request(),
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .expect_err("missing video sidecar capability must fail");

    assert!(error.contains("video sidecar capability 未配置"));
    assert!(error.contains("sidecar profile"));
}

#[tokio::test]
async fn video_poll_sidecar_missing_capability_fails_without_embedded_fallback() {
    let error = poll_video(
        RuntimeProfile::Sidecar,
        &A4nCapabilities::empty(),
        &video_config(),
        "job-1",
        Some(42),
        None,
        None,
    )
    .await
    .expect_err("missing video sidecar capability must fail");

    assert!(error.contains("video sidecar capability 未配置"));
    assert!(error.contains("sidecar profile"));
}

#[tokio::test]
async fn video_probe_sidecar_missing_capability_fails_without_embedded_fallback() {
    let error = probe_video_config(
        RuntimeProfile::Sidecar,
        &A4nCapabilities::empty(),
        &video_config(),
    )
    .await
    .expect_err("missing video sidecar capability must fail");

    assert!(error.contains("video sidecar capability 未配置"));
    assert!(error.contains("sidecar profile"));
}

#[tokio::test]
async fn video_provider_preview_uses_typed_capability_request() {
    let client = Arc::new(
        RecordingVideoClient::new().with_preview_response(successful_video_preview_response()),
    );
    let capabilities = A4nCapabilities::empty().with_video(client.clone());

    let preview = preview_video_provider_request(
        &capabilities,
        &video_config(),
        video_request(),
        Some(42),
        Some(3),
    )
    .await
    .expect("video provider preview");

    assert_eq!(client.preview_call_count(), 1);
    assert_eq!(client.generate_call_count(), 0);
    assert_eq!(client.probe_call_count(), 0);
    assert_eq!(client.poll_call_count(), 0);
    assert_eq!(preview["request"]["model"], "video-model");
    let request = client
        .last_preview_request()
        .expect("recorded preview request");
    assert_eq!(request.project_id, Some(42));
    assert_eq!(request.root_project_id, Some(3));
    assert!(request.secret_policy.is_local_user_secret_only());
    assert_eq!(request.payload["backend"], "openai-compatible-video");
    assert_eq!(request.payload["api_key"], "test-key");
    assert_eq!(request.payload["request"]["prompt"], "test video");
}

#[tokio::test]
async fn video_generation_maps_workflow_run_to_service_trace() {
    let client = Arc::new(
        RecordingVideoClient::new().with_generate_events(successful_video_generation_events()),
    );
    let capabilities = A4nCapabilities::empty().with_video(client.clone());

    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();
    let output = generate_video(
        RuntimeProfile::Sidecar,
        &capabilities,
        &video_config(),
        video_request(),
        Some("a".repeat(64)),
        Some(42),
        Some(3),
        Some(77),
        Some(progress_tx),
    )
    .await
    .expect("video generation");

    assert_eq!(output.bytes, b"video");
    assert_eq!(client.generate_call_count(), 1);
    assert_eq!(
        progress_rx.try_recv().expect("video progress message"),
        "video sidecar 正在提交 provider 请求"
    );
    let request = client
        .last_generate_request()
        .expect("recorded video generation request");
    assert_eq!(request.project_id, Some(42));
    assert_eq!(request.root_project_id, Some(3));
    assert_eq!(request.trace_context.trace_id, "workflow_run:77");
    assert_eq!(
        request.trace_context.parent_span_id.as_deref(),
        Some("video.generate")
    );
    assert_eq!(request.payload["project_id"], 42);
    assert_eq!(
        request.payload["provider_idempotency_key_sha256"],
        "a".repeat(64)
    );
}

#[test]
fn video_generation_terminal_events_preserve_failure_and_cancel_semantics() {
    let failed = CapabilityTaskEvent {
        task_id: "task-1".to_string(),
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
            message: "provider failed".to_string(),
            retryable: Some(false),
        }),
        payload: None,
    };
    let error = handle_video_generation_event(failed, None)
        .expect_err("provider failure must terminate generation");
    assert!(error.contains("provider failed"));

    let cancelled = CapabilityTaskEvent {
        task_id: "task-1".to_string(),
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
    let error = handle_video_generation_event(cancelled, None)
        .expect_err("cancel ack must terminate generation");
    assert_eq!(error, AppError::Cancelled.to_string());
}

#[tokio::test]
async fn video_provider_preview_missing_capability_has_no_embedded_fallback() {
    let error = preview_video_provider_request(
        &A4nCapabilities::empty(),
        &video_config(),
        video_request(),
        Some(42),
        Some(3),
    )
    .await
    .expect_err("missing video preview capability must fail");

    assert!(error.contains("video sidecar capability 未配置"));
}

#[tokio::test]
async fn video_poll_carries_project_scope_in_context_and_payload() {
    let client =
        Arc::new(RecordingVideoClient::new().with_poll_response(successful_video_poll_response()));
    let capabilities = A4nCapabilities::empty().with_video(client.clone());

    let state = poll_video(
        RuntimeProfile::Sidecar,
        &capabilities,
        &video_config(),
        "job-42",
        Some(42),
        Some(3),
        Some(77),
    )
    .await
    .expect("video poll");

    assert!(matches!(state, VideoJobState::Pending));
    assert_eq!(client.poll_call_count(), 1);
    let request = client.last_poll_request().expect("recorded poll request");
    assert_eq!(request.project_id, Some(42));
    assert_eq!(request.root_project_id, Some(3));
    assert_eq!(request.trace_context.trace_id, "workflow_run:77");
    assert_eq!(
        request.trace_context.parent_span_id.as_deref(),
        Some("video.poll")
    );
    assert!(request.secret_policy.is_local_user_secret_only());
    assert_eq!(request.payload["project_id"], 42);
    assert_eq!(request.payload["job_id"], "job-42");
}

#[tokio::test]
async fn video_status_uses_status_only_capability_without_polling() {
    let client = Arc::new(
        RecordingVideoClient::new().with_status_response(video_response(json!({
            "state": "completed"
        }))),
    );
    let capabilities = A4nCapabilities::empty().with_video(client.clone());

    let status = video_job_status(
        RuntimeProfile::Sidecar,
        &capabilities,
        &video_config(),
        "job-42",
        Some(42),
        Some(3),
    )
    .await
    .expect("video status");

    assert_eq!(status, VideoJobStatus::Completed);
    assert_eq!(client.status_call_count(), 1);
    assert_eq!(client.poll_call_count(), 0);
    let request = client
        .last_status_request()
        .expect("recorded status request");
    assert_eq!(request.project_id, Some(42));
    assert_eq!(request.root_project_id, Some(3));
    assert_eq!(request.payload["job_id"], "job-42");
}

#[tokio::test]
async fn video_delete_carries_project_scope_and_maps_outcome() {
    let client = Arc::new(
        RecordingVideoClient::new().with_delete_response(video_response(json!({
            "outcome": "deleted"
        }))),
    );
    let capabilities = A4nCapabilities::empty().with_video(client.clone());

    let outcome = delete_video_job(
        RuntimeProfile::Sidecar,
        &capabilities,
        &video_config(),
        "job-42",
        Some(42),
        Some(3),
    )
    .await
    .expect("video delete");

    assert_eq!(outcome, VideoJobDeleteOutcome::Deleted);
    assert_eq!(client.delete_call_count(), 1);
    let request = client
        .last_delete_request()
        .expect("recorded delete request");
    assert_eq!(request.project_id, Some(42));
    assert_eq!(request.root_project_id, Some(3));
    assert_eq!(request.payload["job_id"], "job-42");
}

fn successful_video_generation_events() -> Vec<CapabilityTaskEvent> {
    vec![
        video_event(
            CapabilityTaskEventKind::Progress,
            1,
            json!({
                "current": 1,
                "total": 2,
                "message": "video sidecar 正在提交 provider 请求"
            }),
        ),
        video_event(
            CapabilityTaskEventKind::Completed,
            2,
            json!({
                "bytes_base64": "dmlkZW8=",
                "format": "mp4",
                "provider_job_id": null,
                "metadata": {}
            }),
        ),
    ]
}

fn successful_video_poll_response() -> JsonResponse {
    video_response(json!({"state": "pending"}))
}

fn successful_video_preview_response() -> JsonResponse {
    video_response(json!({
        "request": {"model": "video-model", "prompt": "test video"}
    }))
}

fn video_response(payload: serde_json::Value) -> JsonResponse {
    ServiceResponse {
        status: ServiceStatus::Ok,
        trace_id: Some("trace".to_string()),
        usage: None,
        artifact_refs: Vec::new(),
        payload: Some(payload),
        error: None,
    }
}

fn video_event(
    event_kind: CapabilityTaskEventKind,
    sequence: u64,
    payload: serde_json::Value,
) -> CapabilityTaskEvent {
    CapabilityTaskEvent {
        task_id: "configured-task".to_string(),
        event_kind,
        sequence,
        chunk_index: None,
        chunk_delta: None,
        usage_delta: None,
        final_usage: None,
        finish_reason: None,
        is_truncated_finish_reason: None,
        checkpoint_key: None,
        artifact_refs: Vec::new(),
        error: None,
        payload: Some(payload),
    }
}
