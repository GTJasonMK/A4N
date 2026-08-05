use super::events::handle_image_generation_event;
use super::operations::{generate_image, preview_image_provider_request, probe_image_config};
use crate::state::RuntimeProfile;
use a4n_capabilities::{A4nCapabilities, JsonResponse};
use a4n_core::error::AppError;
use a4n_core::models::ai::image_gen_config;
use a4n_image::ImageGenerationRequest;
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, ServiceErrorCode, ServiceResponse, ServiceStatus,
};
use a4n_test_support::capability::RecordingImageClient;
use serde_json::json;
use std::sync::Arc;

fn fixed_time() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

fn image_config() -> image_gen_config::Model {
    let now = fixed_time();
    image_gen_config::Model {
        id: 1,
        name: "test-image".to_string(),
        backend: "openai-compatible-image".to_string(),
        base_url: Some("https://example.invalid/v1".to_string()),
        api_key_encrypted: Some("test-key".to_string()),
        model: "image-model".to_string(),
        max_concurrent: 1,
        default_width: 1024,
        default_height: 1024,
        is_default: true,
        created_at: now,
        updated_at: now,
    }
}

fn image_request() -> ImageGenerationRequest {
    ImageGenerationRequest {
        prompt: "test image".to_string(),
        width: 1024,
        height: 1024,
        model: "image-model".to_string(),
        reference_images: Vec::new(),
    }
}

#[tokio::test]
async fn image_sidecar_missing_capability_fails_without_embedded_fallback() {
    let error = generate_image(
        RuntimeProfile::Sidecar,
        &A4nCapabilities::empty(),
        &image_config(),
        image_request(),
        None,
        None,
        None,
        None,
    )
    .await
    .expect_err("missing image sidecar capability must fail");

    assert!(error.contains("image sidecar capability 未配置"));
    assert!(error.contains("sidecar profile"));
}

#[tokio::test]
async fn image_generation_maps_workflow_run_to_service_trace() {
    let client = Arc::new(
        RecordingImageClient::new().with_generate_events(successful_image_generation_events()),
    );
    let capabilities = A4nCapabilities::empty().with_image(client.clone());

    let progress_messages = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let progress_record = progress_messages.clone();
    let progress = move |message: &str| {
        progress_record
            .lock()
            .expect("progress record lock")
            .push(message.to_string());
    };
    let output = generate_image(
        RuntimeProfile::Sidecar,
        &capabilities,
        &image_config(),
        image_request(),
        Some(42),
        Some(3),
        Some(77),
        Some(&progress),
    )
    .await
    .expect("image generation");

    assert_eq!(output.bytes, b"img");
    assert_eq!(client.generate_call_count(), 1);
    assert_eq!(
        progress_messages
            .lock()
            .expect("progress messages lock")
            .as_slice(),
        ["image sidecar 正在调用 provider"]
    );
    let request = client
        .last_generate_request()
        .expect("recorded image generation request");
    assert_eq!(request.project_id, Some(42));
    assert_eq!(request.root_project_id, Some(3));
    assert_eq!(request.trace_context.trace_id, "workflow_run:77");
    assert_eq!(
        request.trace_context.parent_span_id.as_deref(),
        Some("image.generate")
    );
    assert!(request.payload.get("project_id").is_none());
}

#[test]
fn image_generation_terminal_events_preserve_failure_and_cancel_semantics() {
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
    let error = handle_image_generation_event(failed, None)
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
    let error = handle_image_generation_event(cancelled, None)
        .expect_err("cancel ack must terminate generation");
    assert_eq!(error, AppError::Cancelled.to_string());
}

#[tokio::test]
async fn image_connectivity_uses_probe_without_generation() {
    let client = Arc::new(
        RecordingImageClient::new().with_probe_response(successful_image_probe_response()),
    );
    let capabilities = A4nCapabilities::empty().with_image(client.clone());

    probe_image_config(&capabilities, &image_config(), image_request())
        .await
        .expect("image probe");

    assert_eq!(client.probe_call_count(), 1);
    assert_eq!(client.generate_call_count(), 0);
    let request = client.last_probe_request().expect("recorded probe request");
    assert!(request.secret_policy.is_local_user_secret_only());
    assert_eq!(request.payload["backend"], "openai-compatible-image");
    assert_eq!(request.payload["api_key"], "test-key");
    assert_eq!(request.payload["request"]["prompt"], "test image");
}

#[tokio::test]
async fn image_probe_missing_capability_fails_without_generation_fallback() {
    let error = probe_image_config(&A4nCapabilities::empty(), &image_config(), image_request())
        .await
        .expect_err("missing image probe capability must fail");

    assert!(error.contains("image sidecar capability 未配置"));
}

#[tokio::test]
async fn image_provider_preview_uses_typed_capability_request() {
    let client = Arc::new(
        RecordingImageClient::new().with_preview_response(successful_image_preview_response()),
    );
    let capabilities = A4nCapabilities::empty().with_image(client.clone());

    let preview = preview_image_provider_request(
        &capabilities,
        &image_config(),
        image_request(),
        Some(42),
        Some(3),
    )
    .await
    .expect("image provider preview");

    assert_eq!(client.preview_call_count(), 1);
    assert_eq!(client.probe_call_count(), 0);
    assert_eq!(client.generate_call_count(), 0);
    assert_eq!(preview["request"]["model"], "image-model");
    let request = client
        .last_preview_request()
        .expect("recorded preview request");
    assert!(request.secret_policy.is_local_user_secret_only());
    assert_eq!(request.project_id, Some(42));
    assert_eq!(request.root_project_id, Some(3));
    assert_eq!(request.payload["backend"], "openai-compatible-image");
    assert!(request.payload.get("project_id").is_none());
    assert_eq!(request.payload["request"]["prompt"], "test image");
}

#[tokio::test]
async fn image_provider_preview_missing_capability_has_no_embedded_fallback() {
    let error = preview_image_provider_request(
        &A4nCapabilities::empty(),
        &image_config(),
        image_request(),
        Some(42),
        Some(3),
    )
    .await
    .expect_err("missing image preview capability must fail");

    assert!(error.contains("image sidecar capability 未配置"));
}

fn successful_image_generation_events() -> Vec<CapabilityTaskEvent> {
    vec![
        image_event(
            CapabilityTaskEventKind::Progress,
            1,
            json!({
                "current": 1,
                "total": 2,
                "message": "image sidecar 正在调用 provider"
            }),
        ),
        image_event(
            CapabilityTaskEventKind::Completed,
            2,
            json!({
                "bytes_base64": "aW1n",
                "format": "png",
                "model": "image-model"
            }),
        ),
    ]
}

fn successful_image_preview_response() -> JsonResponse {
    image_response(json!({
        "request": {"model": "image-model", "prompt": "test image"}
    }))
}

fn successful_image_probe_response() -> JsonResponse {
    image_response(json!({
        "ok": true,
        "request_preview": {"request": {"model": "image-model"}}
    }))
}

fn image_response(payload: serde_json::Value) -> JsonResponse {
    ServiceResponse {
        status: ServiceStatus::Ok,
        trace_id: Some("trace".to_string()),
        usage: None,
        artifact_refs: Vec::new(),
        payload: Some(payload),
        error: None,
    }
}

fn image_event(
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
