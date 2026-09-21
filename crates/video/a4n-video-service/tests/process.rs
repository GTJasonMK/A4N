use a4n_service_protocol::{CapabilityTaskEvent, SIDECAR_EVENT_QUEUE_CAPACITY, ServiceErrorCode};
use a4n_test_support::process::{
    FakeJsonProvider, HangingJsonProvider, HttpVideoClient, SIDECAR_PROCESS_TEST_TOKEN,
    ServiceStatus, VIDEO_PROVIDER_CAPABILITY_VERSION, VIDEO_SERVICE_NAME, VideoClient,
    VideoDeleteOutput, VideoGenerationRequest, VideoJobOutput, VideoMediaInput, VideoPollPayload,
    VideoPollRequest, VideoProviderPayload, VideoProviderRequest, VideoStatusOutput,
    assert_process_concurrency_saturation, assert_protected_route_rejects_invalid_auth,
    assert_route_rejects_malformed_json, assert_route_rejects_oversized_json,
    assert_route_rejects_unknown_payload_field, assert_route_rejects_version_mismatch,
    finish_sidecar, provider_request_context, sidecar_base_url, sidecar_manager, start_sidecar,
    trusted_remote_request_context, unused_local_addr,
};
use axum::http::StatusCode;
use tokio::time::{Duration, timeout};

#[tokio::test]
async fn video_service_process_calls_local_fake_provider_and_stops() {
    let addr = unused_local_addr();
    let (manager, _temp_dir) = sidecar_manager(
        "video",
        env!("CARGO_BIN_EXE_a4n-video-service"),
        addr,
        "A4N_VIDEO_SERVICE_ADDR",
        [],
    );
    let process = start_sidecar(&manager, VIDEO_SERVICE_NAME).await;
    assert_process_concurrency_saturation(addr, VIDEO_SERVICE_NAME, "/video/models").await;
    let base_url = sidecar_base_url(addr);
    assert_protected_route_rejects_invalid_auth(&base_url, "/video/generate/events").await;
    assert_protected_route_rejects_invalid_auth(&base_url, "/video/poll").await;
    assert_protected_route_rejects_invalid_auth(&base_url, "/video/status").await;
    assert_protected_route_rejects_invalid_auth(&base_url, "/video/delete").await;
    assert_protected_route_rejects_invalid_auth(&base_url, "/video/request_preview").await;
    assert_route_rejects_malformed_json(&base_url, "/video/generate/events").await;
    assert_route_rejects_oversized_json(&base_url, "/video/models", 8 * 1024 * 1024).await;
    assert_route_rejects_oversized_json(&base_url, "/video/generate/events", 8 * 1024 * 1024).await;
    let client = HttpVideoClient::new_with_auth_token(base_url.clone(), SIDECAR_PROCESS_TEST_TOKEN);
    let cancel_client = HttpVideoClient::new_with_auth_token(base_url, SIDECAR_PROCESS_TEST_TOKEN);
    let contract_request = video_generation_request(
        "video-process-contract",
        "video-process-contract-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        "openai-compatible-video",
        "http://127.0.0.1:1".to_string(),
        "process-provider-secret",
        "contract test",
    );
    assert_route_rejects_unknown_payload_field(
        &sidecar_base_url(addr),
        "/video/request_preview",
        &contract_request,
    )
    .await;
    let (hanging_provider, provider_request_started, provider_body_dropped) =
        HangingJsonProvider::start().await;
    let cancelled_request = video_generation_request(
        "video-process-cancel",
        "video-process-cancel-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        "openai-compatible-video",
        hanging_provider.base_url(),
        "process-provider-secret",
        "cancel process test",
    );
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    let cancel_task = tokio::spawn(async move {
        cancel_client
            .generate_events(cancelled_request, event_tx)
            .await
    });
    timeout(Duration::from_secs(5), event_rx.recv())
        .await
        .expect("video sidecar should emit initial progress before timeout")
        .expect("video sidecar should emit initial progress");
    timeout(Duration::from_secs(5), provider_request_started)
        .await
        .expect("video provider request should start before timeout")
        .expect("video provider request start signal should be delivered");
    drop(event_rx);
    let error = timeout(Duration::from_secs(5), cancel_task)
        .await
        .expect("cancelled video event request should finish before timeout")
        .expect("cancelled video event request task should join")
        .expect_err("closed video event receiver should cancel the request");
    assert_eq!(error.code, ServiceErrorCode::Cancelled);
    timeout(Duration::from_secs(5), provider_body_dropped)
        .await
        .expect("cancelled video request should release provider response body")
        .expect("video provider body drop signal should be delivered");
    hanging_provider.stop().await;

    let (idle_provider, idle_request_started, idle_body_dropped) =
        HangingJsonProvider::start().await;
    let idle_client =
        HttpVideoClient::new_with_auth_token(sidecar_base_url(addr), SIDECAR_PROCESS_TEST_TOKEN)
            .with_test_timeouts(
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_millis(100),
                Duration::from_secs(1),
            );
    let idle_request = video_generation_request(
        "video-process-stream-idle",
        "video-process-stream-idle-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        "openai-compatible-video",
        idle_provider.base_url(),
        "process-provider-secret",
        "stream idle process test",
    );
    let (idle_tx, mut idle_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    let idle_error = timeout(
        Duration::from_secs(5),
        idle_client.generate_events(idle_request, idle_tx),
    )
    .await
    .expect("video stream idle timeout should be observed promptly")
    .expect_err("a stalled video event stream must hit the idle deadline");
    assert_eq!(idle_error.code, ServiceErrorCode::ServiceUnavailable);
    assert!(idle_error.message.contains("event stream idle timeout"));
    assert!(!idle_error.message.contains("process-provider-secret"));
    assert!(!idle_error.message.contains(&idle_provider.base_url()));
    timeout(Duration::from_secs(5), idle_request_started)
        .await
        .expect("video idle provider request should start")
        .expect("video idle provider request signal should be delivered");
    timeout(Duration::from_secs(5), idle_rx.recv())
        .await
        .expect("video sidecar should emit progress before becoming idle")
        .expect("video sidecar should emit progress before becoming idle");
    timeout(Duration::from_secs(5), idle_body_dropped)
        .await
        .expect("video stream idle timeout should release provider response body")
        .expect("video idle provider body drop signal should be delivered");
    idle_provider.stop().await;

    let preview_provider = FakeJsonProvider::start(serde_json::json!({"id": "preview-job"})).await;
    let preview_request = video_generation_request(
        "video-process-preview",
        "video-process-preview-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        "openai-compatible-video",
        preview_provider.base_url(),
        "process-provider-secret",
        "preview test",
    );
    let backpressure_client =
        HttpVideoClient::new_with_auth_token(sidecar_base_url(addr), SIDECAR_PROCESS_TEST_TOKEN);
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
    .expect("video stream should fill the bounded event queue");
    assert!(
        !backpressure.is_finished(),
        "a full video event queue must apply backpressure"
    );
    loop {
        let event = backpressure_rx.recv().await.expect("video terminal event");
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
        .expect("video backpressure task should join")
        .expect("video stream should finish after queue capacity is released");
    let preview = client
        .preview_request(preview_request.clone())
        .await
        .expect("video preview should cross the independent process");
    assert_eq!(preview.status, ServiceStatus::Ok);
    assert!(preview.payload.expect("video preview payload").is_object());
    preview_provider.stop().await;

    let remote_secret_events = video_generation_events(
        &client,
        trusted_remote_request_context(
            "video-process-remote-secret",
            "video-process-remote-secret-trace",
            VIDEO_PROVIDER_CAPABILITY_VERSION,
            VideoProviderPayload {
                backend: "openai-compatible-video".to_string(),
                base_url: "http://127.0.0.1:1".to_string(),
                api_key: "process-provider-secret".to_string(),
                provider_idempotency_key_sha256: None,
                project_id: None,
                request: video_request("remote secret must stop"),
            },
        ),
    )
    .await;
    let remote_secret_error = remote_secret_events
        .last()
        .expect("remote video secret terminal event")
        .error
        .as_ref()
        .expect("remote video secret rejection error");
    assert_eq!(remote_secret_error.code, ServiceErrorCode::Unauthorized);
    assert!(
        !remote_secret_error
            .message
            .contains("process-provider-secret")
    );

    let remote_provider = FakeJsonProvider::start(serde_json::json!({
        "data": [{"b64_json": "bWVkaWE=", "format": "mp4"}]
    }))
    .await;
    let remote_content_events = video_generation_events(
        &client,
        trusted_remote_request_context(
            "video-process-remote-content",
            "video-process-remote-content-trace",
            VIDEO_PROVIDER_CAPABILITY_VERSION,
            VideoProviderPayload {
                backend: "openai-compatible-video".to_string(),
                base_url: remote_provider.base_url(),
                api_key: String::new(),
                provider_idempotency_key_sha256: None,
                project_id: None,
                request: video_request("remote video content"),
            },
        ),
    )
    .await;
    assert_eq!(
        remote_content_events
            .last()
            .expect("remote video content terminal event")
            .payload
            .as_ref()
            .expect("remote video payload")["format"],
        "mp4"
    );
    remote_provider.stop().await;

    let failed_provider_addr = unused_local_addr();
    let failed_events_request = video_generation_request(
        "video-process-provider-failure",
        "video-process-provider-failure-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        "openai-compatible-video",
        sidecar_base_url(failed_provider_addr),
        "process-provider-secret",
        "failure test",
    );
    let failed_events = video_generation_events(&client, failed_events_request).await;
    let failed_event = failed_events.last().expect("video failed terminal event");
    assert_eq!(
        failed_event.event_kind,
        a4n_service_protocol::CapabilityTaskEventKind::Failed
    );
    let failed_error = failed_event.error.as_ref().expect("video provider error");
    assert_eq!(failed_error.code, ServiceErrorCode::ProviderError);
    assert!(!failed_error.message.contains("process-provider-secret"));
    assert!(
        !failed_error
            .message
            .contains(sidecar_base_url(failed_provider_addr).as_str())
    );

    let bytes_provider = FakeJsonProvider::start(serde_json::json!({
        "data": [{"b64_json": "bWVkaWE=", "format": "mp4"}]
    }))
    .await;
    let bytes_request = video_generation_request(
        "video-process-bytes",
        "video-process-bytes-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        "openai-compatible-video",
        bytes_provider.base_url(),
        "process-provider-secret",
        "bytes test",
    );
    let bytes_events = video_generation_events(&client, bytes_request).await;
    assert_eq!(
        bytes_events
            .last()
            .expect("video bytes terminal event")
            .payload
            .as_ref()
            .expect("video bytes payload")["format"],
        "mp4"
    );
    bytes_provider.stop().await;

    let provider = FakeJsonProvider::start(serde_json::json!({"id": "process-video-job"})).await;
    let request = video_generation_request(
        "video-process-generate",
        "video-process-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        "openai-compatible-video",
        provider.base_url(),
        "process-provider-secret",
        "process test",
    );
    assert_route_rejects_version_mismatch(
        &sidecar_base_url(addr),
        "/video/request_preview",
        &request,
    )
    .await;
    let events = video_generation_events(&client, request).await;
    let completed = events.last().expect("video completed event");
    assert_eq!(
        completed.event_kind,
        a4n_service_protocol::CapabilityTaskEventKind::Completed
    );
    assert_eq!(
        completed.payload.as_ref().expect("video payload")["provider_job_id"],
        "process-video-job"
    );
    provider.stop().await;

    let pending_provider = FakeJsonProvider::start(serde_json::json!({"status": "pending"})).await;
    let pending = client
        .poll(video_poll_request(
            "video-process-poll-pending",
            "video-process-poll-pending-trace",
            VIDEO_PROVIDER_CAPABILITY_VERSION,
            "openai-compatible-video",
            pending_provider.base_url(),
            "process-provider-secret",
            "process-video-job",
        ))
        .await
        .expect("video pending poll should cross the independent process");
    assert_eq!(pending.status, ServiceStatus::Ok);
    assert!(matches!(
        pending.payload.as_ref(),
        Some(VideoJobOutput::Pending)
    ));
    pending_provider.stop().await;

    let failed_poll_provider = FakeJsonProvider::start(serde_json::json!({
        "status": "failed",
        "error_message": "token=process-upstream-secret base_url=https://process-provider.invalid/v1"
    }))
    .await;
    let failed_poll = client
        .poll(video_poll_request(
            "video-process-poll-failed",
            "video-process-poll-failed-trace",
            VIDEO_PROVIDER_CAPABILITY_VERSION,
            "openai-compatible-video",
            failed_poll_provider.base_url(),
            "process-provider-secret",
            "process-video-job",
        ))
        .await
        .expect("video failed poll should cross the independent process");
    assert_eq!(failed_poll.status, ServiceStatus::Ok);
    let Some(VideoJobOutput::Failed { message }) = failed_poll.payload.as_ref() else {
        panic!("failed video poll should return failed output");
    };
    assert!(!message.contains("process-upstream-secret"));
    assert!(!message.contains("process-provider.invalid"));
    failed_poll_provider.stop().await;

    let completed_provider = FakeJsonProvider::start(serde_json::json!({
        "status": "completed",
        "data": [{"b64_json": "bWVkaWE=", "format": "mp4"}]
    }))
    .await;
    let completed = client
        .poll(video_poll_request(
            "video-process-poll-completed",
            "video-process-poll-completed-trace",
            VIDEO_PROVIDER_CAPABILITY_VERSION,
            "openai-compatible-video",
            completed_provider.base_url(),
            "process-provider-secret",
            "process-video-job",
        ))
        .await
        .expect("video completed poll should cross the independent process");
    assert_eq!(completed.status, ServiceStatus::Ok);
    let Some(VideoJobOutput::Completed { result }) = completed.payload else {
        panic!("completed video poll should return completed output");
    };
    assert_eq!(result.format, "mp4");
    completed_provider.stop().await;

    let status_provider = FakeJsonProvider::start(serde_json::json!({
        "status": "completed",
        "data": [{"url": "http://127.0.0.1:1/must-not-download", "format": "mp4"}]
    }))
    .await;
    let status = client
        .status(video_poll_request(
            "video-process-status-completed",
            "video-process-status-completed-trace",
            VIDEO_PROVIDER_CAPABILITY_VERSION,
            "openai-compatible-video",
            status_provider.base_url(),
            "process-provider-secret",
            "process-video-job",
        ))
        .await
        .expect("video status should not download completed artifact");
    assert_eq!(status.status, ServiceStatus::Ok);
    assert!(matches!(
        status.payload.as_ref(),
        Some(VideoStatusOutput::Completed)
    ));
    status_provider.stop().await;

    let missing_provider = FakeJsonProvider::start_with_status(
        StatusCode::NOT_FOUND,
        serde_json::json!({"error": "gone"}),
    )
    .await;
    let missing = client
        .status(video_poll_request(
            "video-process-status-not-found",
            "video-process-status-not-found-trace",
            VIDEO_PROVIDER_CAPABILITY_VERSION,
            "openai-compatible-video",
            missing_provider.base_url(),
            "process-provider-secret",
            "missing-video-job",
        ))
        .await
        .expect("404 status should be idempotent");
    assert_eq!(missing.status, ServiceStatus::Ok);
    assert!(matches!(
        missing.payload.as_ref(),
        Some(VideoStatusOutput::NotFound)
    ));
    missing_provider.stop().await;

    let delete_provider = FakeJsonProvider::start(serde_json::json!({})).await;
    let delete_request = video_poll_request(
        "video-process-delete",
        "video-process-delete-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        "cliproxy-openai-video",
        format!("{}/openai/v1", delete_provider.base_url()),
        "process-provider-secret",
        "process-video-job",
    );
    assert_route_rejects_unknown_payload_field(
        &sidecar_base_url(addr),
        "/video/delete",
        &delete_request,
    )
    .await;
    assert_route_rejects_version_mismatch(
        &sidecar_base_url(addr),
        "/video/delete",
        &delete_request,
    )
    .await;
    let deleted = client
        .delete(delete_request)
        .await
        .expect("video delete should cross the independent process");
    assert_eq!(deleted.status, ServiceStatus::Ok);
    assert!(matches!(
        deleted.payload.as_ref(),
        Some(VideoDeleteOutput::Deleted)
    ));
    delete_provider.stop().await;

    finish_sidecar(process, &manager, VIDEO_SERVICE_NAME, addr).await;
}

async fn video_generation_events(
    client: &HttpVideoClient,
    request: VideoProviderRequest,
) -> Vec<CapabilityTaskEvent> {
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    client
        .generate_events(request, event_tx)
        .await
        .expect("video generate events should cross the independent process");
    let mut events = Vec::new();
    while let Some(event) = event_rx.recv().await {
        events.push(event);
    }
    events
}

fn video_generation_request(
    request_id: &str,
    trace_id: &str,
    capability_version: &str,
    backend: &str,
    base_url: String,
    api_key: &str,
    prompt: &str,
) -> VideoProviderRequest {
    provider_request_context(
        request_id,
        trace_id,
        capability_version,
        VideoProviderPayload {
            backend: backend.to_string(),
            base_url,
            api_key: api_key.to_string(),
            provider_idempotency_key_sha256: None,
            project_id: None,
            request: video_request(prompt),
        },
    )
}

fn video_request(prompt: &str) -> VideoGenerationRequest {
    VideoGenerationRequest {
        prompt: prompt.to_string(),
        negative_prompt: None,
        width: 16,
        height: 16,
        duration_secs: 1,
        fps: 1,
        model: "fake-video".to_string(),
        media: VideoMediaInput::None,
    }
}

fn video_poll_request(
    request_id: &str,
    trace_id: &str,
    capability_version: &str,
    backend: &str,
    base_url: String,
    api_key: &str,
    job_id: &str,
) -> VideoPollRequest {
    provider_request_context(
        request_id,
        trace_id,
        capability_version,
        VideoPollPayload {
            backend: backend.to_string(),
            base_url,
            api_key: api_key.to_string(),
            project_id: None,
            job_id: job_id.to_string(),
        },
    )
}
