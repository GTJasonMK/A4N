use a4n_service_protocol::{CapabilityTaskEvent, SIDECAR_EVENT_QUEUE_CAPACITY, ServiceErrorCode};
use a4n_test_support::process::{
    FakeJsonProvider, HangingJsonProvider, HttpImageClient, IMAGE_PROVIDER_CAPABILITY_VERSION,
    IMAGE_SERVICE_NAME, ImageClient, JsonRequest, SIDECAR_PROCESS_TEST_TOKEN, ServiceStatus,
    assert_process_concurrency_saturation, assert_protected_route_rejects_invalid_auth,
    assert_route_rejects_malformed_json, assert_route_rejects_oversized_json,
    assert_route_rejects_unknown_payload_field, assert_route_rejects_version_mismatch,
    finish_sidecar, provider_request_context, sidecar_base_url, sidecar_manager, start_sidecar,
    trusted_remote_request_context, unused_local_addr,
};
use tokio::time::{Duration, timeout};

const PNG_1X1_B64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

#[tokio::test]
async fn image_service_process_calls_local_fake_provider_and_stops() {
    let addr = unused_local_addr();
    let (manager, _temp_dir) = sidecar_manager(
        "image",
        env!("CARGO_BIN_EXE_a4n-image-service"),
        addr,
        "A4N_IMAGE_SERVICE_ADDR",
        [],
    );
    let process = start_sidecar(&manager, IMAGE_SERVICE_NAME).await;
    assert_process_concurrency_saturation(addr, IMAGE_SERVICE_NAME, "/image/models").await;
    let base_url = sidecar_base_url(addr);
    assert_protected_route_rejects_invalid_auth(&base_url, "/image/generate/events").await;
    assert_protected_route_rejects_invalid_auth(&base_url, "/image/request_preview").await;
    assert_route_rejects_malformed_json(&base_url, "/image/generate/events").await;
    assert_route_rejects_oversized_json(&base_url, "/image/models", 8 * 1024 * 1024).await;
    assert_route_rejects_oversized_json(&base_url, "/image/generate/events", 8 * 1024 * 1024).await;
    let client = HttpImageClient::new_with_auth_token(base_url.clone(), SIDECAR_PROCESS_TEST_TOKEN);
    let cancel_client = HttpImageClient::new_with_auth_token(base_url, SIDECAR_PROCESS_TEST_TOKEN);
    let contract_request = provider_request_context(
        "image-process-contract",
        "image-process-contract-trace",
        IMAGE_PROVIDER_CAPABILITY_VERSION,
        serde_json::json!({
            "backend": "openai-compatible-image",
            "base_url": "http://127.0.0.1:1",
            "api_key": "process-provider-secret",
            "request": {"prompt": "contract test", "width": 1, "height": 1, "model": "fake-image", "reference_images": []}
        }),
    );
    assert_route_rejects_unknown_payload_field(
        &sidecar_base_url(addr),
        "/image/request_preview",
        &contract_request,
    )
    .await;
    let (hanging_provider, provider_request_started, provider_body_dropped) =
        HangingJsonProvider::start().await;
    let cancelled_request = provider_request_context(
        "image-process-cancel",
        "image-process-cancel-trace",
        IMAGE_PROVIDER_CAPABILITY_VERSION,
        serde_json::json!({
            "backend": "openai-compatible-image",
            "base_url": hanging_provider.base_url(),
            "api_key": "process-provider-secret",
            "request": {"prompt": "cancel process test", "width": 1, "height": 1, "model": "fake-image", "reference_images": []}
        }),
    );
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    let cancel_task = tokio::spawn(async move {
        cancel_client
            .generate_events(cancelled_request, event_tx)
            .await
    });
    timeout(Duration::from_secs(5), event_rx.recv())
        .await
        .expect("image sidecar should emit initial progress before timeout")
        .expect("image sidecar should emit initial progress");
    timeout(Duration::from_secs(5), provider_request_started)
        .await
        .expect("image provider request should start before timeout")
        .expect("image provider request start signal should be delivered");
    drop(event_rx);
    let error = timeout(Duration::from_secs(5), cancel_task)
        .await
        .expect("cancelled image event request should finish before timeout")
        .expect("cancelled image event request task should join")
        .expect_err("closed image event receiver should cancel the request");
    assert_eq!(error.code, ServiceErrorCode::Cancelled);
    timeout(Duration::from_secs(5), provider_body_dropped)
        .await
        .expect("cancelled image request should release provider response body")
        .expect("image provider body drop signal should be delivered");
    hanging_provider.stop().await;

    let (idle_provider, idle_request_started, idle_body_dropped) =
        HangingJsonProvider::start().await;
    let idle_client =
        HttpImageClient::new_with_auth_token(sidecar_base_url(addr), SIDECAR_PROCESS_TEST_TOKEN)
            .with_test_timeouts(
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_millis(100),
                Duration::from_secs(1),
            );
    let idle_request = provider_request_context(
        "image-process-stream-idle",
        "image-process-stream-idle-trace",
        IMAGE_PROVIDER_CAPABILITY_VERSION,
        serde_json::json!({
            "backend": "openai-compatible-image",
            "base_url": idle_provider.base_url(),
            "api_key": "process-provider-secret",
            "request": {"prompt": "stream idle process test", "width": 1, "height": 1, "model": "fake-image", "reference_images": []}
        }),
    );
    let (idle_tx, mut idle_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    let idle_error = timeout(
        Duration::from_secs(5),
        idle_client.generate_events(idle_request, idle_tx),
    )
    .await
    .expect("image stream idle timeout should be observed promptly")
    .expect_err("a stalled image event stream must hit the idle deadline");
    assert_eq!(idle_error.code, ServiceErrorCode::ServiceUnavailable);
    assert!(idle_error.message.contains("event stream idle timeout"));
    assert!(!idle_error.message.contains("process-provider-secret"));
    assert!(!idle_error.message.contains(&idle_provider.base_url()));
    timeout(Duration::from_secs(5), idle_request_started)
        .await
        .expect("image idle provider request should start")
        .expect("image idle provider request signal should be delivered");
    timeout(Duration::from_secs(5), idle_rx.recv())
        .await
        .expect("image sidecar should emit progress before becoming idle")
        .expect("image sidecar should emit progress before becoming idle");
    timeout(Duration::from_secs(5), idle_body_dropped)
        .await
        .expect("image stream idle timeout should release provider response body")
        .expect("image idle provider body drop signal should be delivered");
    idle_provider.stop().await;

    let provider = FakeJsonProvider::start(serde_json::json!({
        "data": [{"b64_json": PNG_1X1_B64, "format": "png"}]
    }))
    .await;

    let remote_secret_events = image_generation_events(
        &client,
        trusted_remote_request_context(
            "image-process-remote-secret",
            "image-process-remote-secret-trace",
            IMAGE_PROVIDER_CAPABILITY_VERSION,
            serde_json::json!({
                "backend": "openai-compatible-image",
                "base_url": "http://127.0.0.1:1",
                "api_key": "process-provider-secret",
                "request": {"prompt": "remote secret must stop", "width": 1, "height": 1, "model": "fake-image", "reference_images": []}
            }),
        ),
    )
    .await;
    let remote_secret_error = remote_secret_events
        .last()
        .expect("remote image secret terminal event")
        .error
        .as_ref()
        .expect("remote image secret rejection error");
    assert_eq!(remote_secret_error.code, ServiceErrorCode::Unauthorized);
    assert!(
        !remote_secret_error
            .message
            .contains("process-provider-secret")
    );

    let remote_provider = FakeJsonProvider::start(serde_json::json!({
        "images": [PNG_1X1_B64]
    }))
    .await;
    let remote_content_events = image_generation_events(
        &client,
        trusted_remote_request_context(
            "image-process-remote-content",
            "image-process-remote-content-trace",
            IMAGE_PROVIDER_CAPABILITY_VERSION,
            serde_json::json!({
                "backend": "stable-diffusion",
                "base_url": remote_provider.base_url(),
                "api_key": "",
                "request": {"prompt": "remote image content", "width": 1, "height": 1, "model": "", "reference_images": []}
            }),
        ),
    )
    .await;
    assert_eq!(
        remote_content_events
            .last()
            .expect("remote image content terminal event")
            .payload
            .as_ref()
            .expect("remote image payload")["format"],
        "png"
    );
    remote_provider.stop().await;

    let preview_request = provider_request_context(
        "image-process-preview",
        "image-process-preview-trace",
        IMAGE_PROVIDER_CAPABILITY_VERSION,
        serde_json::json!({
            "backend": "openai-compatible-image",
            "base_url": provider.base_url(),
            "api_key": "process-provider-secret",
            "request": {"prompt": "preview test", "width": 1, "height": 1, "model": "fake-image", "reference_images": []}
        }),
    );
    let backpressure_client =
        HttpImageClient::new_with_auth_token(sidecar_base_url(addr), SIDECAR_PROCESS_TEST_TOKEN);
    let (backpressure_tx, mut backpressure_rx) = tokio::sync::mpsc::channel(1);
    let backpressure_request = preview_request.clone();
    let backpressure = tokio::spawn(async move {
        backpressure_client
            .generate_events(backpressure_request, backpressure_tx)
            .await
    });
    timeout(Duration::from_secs(5), async {
        while backpressure_rx.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("image stream should fill the bounded event queue");
    assert!(
        !backpressure.is_finished(),
        "a full image event queue must apply backpressure"
    );
    loop {
        let event = backpressure_rx.recv().await.expect("image terminal event");
        if event.event_kind.is_terminal() {
            assert_eq!(
                event.event_kind,
                a4n_service_protocol::CapabilityTaskEventKind::Completed
            );
            break;
        }
    }
    backpressure
        .await
        .expect("image backpressure task should join")
        .expect("image stream should finish after queue capacity is released");
    let preview = client
        .preview_request(preview_request.clone())
        .await
        .expect("image preview should cross the independent process");
    assert_eq!(preview.status, ServiceStatus::Ok);
    assert!(preview.payload.expect("image preview payload").is_object());
    let failed_provider_addr = unused_local_addr();
    let failed_events_request = provider_request_context(
        "image-process-provider-failure",
        "image-process-provider-failure-trace",
        IMAGE_PROVIDER_CAPABILITY_VERSION,
        serde_json::json!({
            "backend": "openai-compatible-image",
            "base_url": sidecar_base_url(failed_provider_addr),
            "api_key": "process-provider-secret",
            "request": {"prompt": "failure test", "width": 1, "height": 1, "model": "fake-image", "reference_images": []}
        }),
    );
    let failed_events = image_generation_events(&client, failed_events_request).await;
    let failed_event = failed_events.last().expect("image failed terminal event");
    assert_eq!(
        failed_event.event_kind,
        a4n_service_protocol::CapabilityTaskEventKind::Failed
    );
    let failed_error = failed_event.error.as_ref().expect("image provider error");
    assert_eq!(failed_error.code, ServiceErrorCode::ProviderError);
    assert!(!failed_error.message.contains("process-provider-secret"));
    assert!(
        !failed_error
            .message
            .contains(sidecar_base_url(failed_provider_addr).as_str())
    );

    let request = provider_request_context(
        "image-process-generate",
        "image-process-trace",
        IMAGE_PROVIDER_CAPABILITY_VERSION,
        serde_json::json!({
            "backend": "openai-compatible-image",
            "base_url": provider.base_url(),
            "api_key": "process-provider-secret",
            "request": {"prompt": "process test", "width": 1, "height": 1, "model": "fake-image", "reference_images": []}
        }),
    );
    assert_route_rejects_version_mismatch(
        &sidecar_base_url(addr),
        "/image/request_preview",
        &request,
    )
    .await;
    let events = image_generation_events(&client, request).await;
    let completed = events.last().expect("image completed event");
    assert_eq!(
        completed.event_kind,
        a4n_service_protocol::CapabilityTaskEventKind::Completed
    );
    assert_eq!(
        completed.payload.as_ref().expect("image payload")["format"],
        "png"
    );
    finish_sidecar(process, &manager, IMAGE_SERVICE_NAME, addr).await;
    provider.stop().await;
}

async fn image_generation_events(
    client: &HttpImageClient,
    request: JsonRequest,
) -> Vec<CapabilityTaskEvent> {
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    client
        .generate_events(request, event_tx)
        .await
        .expect("image generate events should cross the independent process");
    let mut events = Vec::new();
    while let Some(event) = event_rx.recv().await {
        events.push(event);
    }
    events
}
