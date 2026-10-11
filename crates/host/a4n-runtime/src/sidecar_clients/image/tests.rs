use super::operations::{generate_image, preview_image_provider_request};
use crate::state::RuntimeProfile;
use a4n_image_client::JsonResponse;
use a4n_image_contract::ImageGenerationRequest;
use a4n_provider_contract::ImageProviderConfig;
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, ServiceResponse, ServiceStatus,
};
use a4n_test_support::capability::RecordingImageClient;
use serde_json::json;
use std::sync::Arc;

fn image_config() -> ImageProviderConfig {
    ImageProviderConfig {
        id: 1,
        name: "test-image".to_string(),
        backend: "openai-compatible-image".to_string(),
        base_url: Some("https://example.invalid/v1".to_string()),
        api_key: Some("test-key".to_string()),
        model: "image-model".to_string(),
        max_concurrent: 1,
        supported_sizes_json: json!([{"width": 1024, "height": 1024}]),
        pricing: Default::default(),
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
        None,
        &image_config(),
        image_request(),
        crate::sidecar_clients::SidecarWorkflowScope {
            project_id: None,
            root_project_id: None,
            workflow_run_id: None,
        },
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
        Some(client.clone()),
        &image_config(),
        image_request(),
        crate::sidecar_clients::SidecarWorkflowScope {
            project_id: Some(42),
            root_project_id: Some(3),
            workflow_run_id: Some(77),
        },
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
    let payload = serde_json::to_value(&request.payload).expect("image payload serializes");
    assert!(payload.get("project_id").is_none());
}

#[tokio::test]
async fn image_provider_preview_uses_typed_capability_request() {
    let client = Arc::new(
        RecordingImageClient::new().with_preview_response(successful_image_preview_response()),
    );
    let preview = preview_image_provider_request(
        Some(client.clone()),
        &image_config(),
        image_request(),
        Some(42),
        Some(3),
    )
    .await
    .expect("image provider preview");

    assert_eq!(client.preview_call_count(), 1);
    assert_eq!(client.generate_call_count(), 0);
    assert_eq!(preview["model"], "image-model");
    let request = client
        .last_preview_request()
        .expect("recorded preview request");
    assert!(request.secret_policy.is_local_user_secret_only());
    assert_eq!(request.project_id, Some(42));
    assert_eq!(request.root_project_id, Some(3));
    let payload = serde_json::to_value(&request.payload).expect("image payload serializes");
    assert_eq!(payload["backend"], "openai-compatible-image");
    assert!(payload.get("project_id").is_none());
    assert_eq!(payload["request"]["prompt"], "test image");
}

#[tokio::test]
async fn image_provider_preview_missing_capability_has_no_embedded_fallback() {
    let error =
        preview_image_provider_request(None, &image_config(), image_request(), Some(42), Some(3))
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
    image_response(json!({"model": "image-model", "prompt": "test image"}))
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
        error: None,
        payload: Some(payload),
    }
}
