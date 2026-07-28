use a4n_service_protocol::{SIDECAR_EVENT_QUEUE_CAPACITY, ServiceErrorCode};
use a4n_test_support::process::{
    FakeJsonProvider, HangingJsonProvider, HttpVideoClient, SIDECAR_PROCESS_TEST_TOKEN,
    ServiceStatus, VIDEO_PROVIDER_CAPABILITY_VERSION, VIDEO_SERVICE_NAME, VideoClient,
    assert_event_queue_backpressure, assert_process_concurrency_saturation,
    assert_protected_route_rejects_invalid_auth, assert_route_rejects_malformed_json,
    assert_route_rejects_oversized_json, assert_route_rejects_unknown_payload_field,
    assert_route_rejects_version_mismatch, finish_sidecar, provider_request_context,
    sidecar_base_url, sidecar_manager, start_sidecar, trusted_remote_request_context,
    unused_local_addr,
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
    assert_protected_route_rejects_invalid_auth(&base_url, "/video/generate").await;
    assert_protected_route_rejects_invalid_auth(&base_url, "/video/poll").await;
    assert_protected_route_rejects_invalid_auth(&base_url, "/video/status").await;
    assert_protected_route_rejects_invalid_auth(&base_url, "/video/delete").await;
    assert_protected_route_rejects_invalid_auth(&base_url, "/video/request_preview").await;
    assert_protected_route_rejects_invalid_auth(&base_url, "/video/probe").await;
    assert_route_rejects_malformed_json(&base_url, "/video/generate").await;
    assert_route_rejects_oversized_json(&base_url, "/video/models", 8 * 1024 * 1024).await;
    assert_route_rejects_oversized_json(&base_url, "/video/generate/events", 8 * 1024 * 1024).await;
    assert_route_rejects_oversized_json(&base_url, "/video/generate", 64 * 1024 * 1024).await;
    let client = HttpVideoClient::new_with_auth_token(base_url.clone(), SIDECAR_PROCESS_TEST_TOKEN);
    let cancel_client = HttpVideoClient::new_with_auth_token(base_url, SIDECAR_PROCESS_TEST_TOKEN);
    let contract_request = provider_request_context(
        "video-process-contract",
        "video-process-contract-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        serde_json::json!({
            "backend": "openai-compatible-video",
            "base_url": "http://127.0.0.1:1",
            "api_key": "process-provider-secret",
            "request": {"prompt": "contract test", "negative_prompt": null, "width": 16, "height": 16, "duration_secs": 1, "fps": 1, "model": "fake-video", "first_frame_image_url": null, "last_frame_image_url": null, "reference_image_urls": [], "reference_media": null}
        }),
    );
    assert_route_rejects_unknown_payload_field(
        &sidecar_base_url(addr),
        "/video/generate",
        &contract_request,
    )
    .await;
    let (hanging_provider, provider_request_started, provider_body_dropped) =
        HangingJsonProvider::start().await;
    let cancelled_request = provider_request_context(
        "video-process-cancel",
        "video-process-cancel-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        serde_json::json!({
            "backend": "openai-compatible-video",
            "base_url": hanging_provider.base_url(),
            "api_key": "process-provider-secret",
            "request": {"prompt": "cancel process test", "negative_prompt": null, "width": 16, "height": 16, "duration_secs": 1, "fps": 1, "model": "fake-video", "first_frame_image_url": null, "last_frame_image_url": null, "reference_image_urls": [], "reference_media": null}
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
    let idle_request = provider_request_context(
        "video-process-stream-idle",
        "video-process-stream-idle-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        serde_json::json!({
            "backend": "openai-compatible-video",
            "base_url": idle_provider.base_url(),
            "api_key": "process-provider-secret",
            "request": {"prompt": "stream idle process test", "negative_prompt": null, "width": 16, "height": 16, "duration_secs": 1, "fps": 1, "model": "fake-video", "first_frame_image_url": null, "last_frame_image_url": null, "reference_image_urls": [], "reference_media": null}
        }),
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
    let preview_request = provider_request_context(
        "video-process-preview",
        "video-process-preview-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        serde_json::json!({
            "backend": "openai-compatible-video",
            "base_url": preview_provider.base_url(),
            "api_key": "process-provider-secret",
            "request": {"prompt": "preview test", "negative_prompt": null, "width": 16, "height": 16, "duration_secs": 1, "fps": 1, "model": "fake-video", "first_frame_image_url": null, "last_frame_image_url": null, "reference_image_urls": [], "reference_media": null}
        }),
    );
    let (backpressure_tx, backpressure_rx) = tokio::sync::mpsc::channel(1);
    let backpressure = client
        .generate_events(preview_request.clone(), backpressure_tx)
        .await
        .expect_err("a full video event queue must fail the typed stream");
    assert_event_queue_backpressure(&backpressure);
    drop(backpressure_rx);
    let preview = client
        .preview_request(preview_request.clone())
        .await
        .expect("video preview should cross the independent process");
    assert_eq!(preview.status, ServiceStatus::Ok);
    assert!(preview.payload.expect("video preview payload")["request"].is_object());
    let probe = client
        .probe(preview_request)
        .await
        .expect("video probe should cross the independent process");
    assert_eq!(probe.status, ServiceStatus::Ok);
    assert_eq!(probe.payload.expect("video probe payload")["ok"], true);
    preview_provider.stop().await;

    let remote_secret = client
        .generate(trusted_remote_request_context(
            "video-process-remote-secret",
            "video-process-remote-secret-trace",
            VIDEO_PROVIDER_CAPABILITY_VERSION,
            serde_json::json!({
                "backend": "openai-compatible-video",
                "base_url": "http://127.0.0.1:1",
                "api_key": "process-provider-secret",
                "request": {"prompt": "remote secret must stop", "negative_prompt": null, "width": 16, "height": 16, "duration_secs": 1, "fps": 1, "model": "fake-video", "first_frame_image_url": null, "last_frame_image_url": null, "reference_image_urls": [], "reference_media": null}
            }),
        ))
        .await
        .expect("remote video secret rejection should remain a typed response");
    assert_eq!(remote_secret.status, ServiceStatus::Error);
    let remote_secret_error = remote_secret
        .error
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
    let remote_content = client
        .generate(trusted_remote_request_context(
            "video-process-remote-content",
            "video-process-remote-content-trace",
            VIDEO_PROVIDER_CAPABILITY_VERSION,
            serde_json::json!({
                "backend": "openai-compatible-video",
                "base_url": remote_provider.base_url(),
                "api_key": "",
                "request": {"prompt": "remote video content", "negative_prompt": null, "width": 16, "height": 16, "duration_secs": 1, "fps": 1, "model": "fake-video", "first_frame_image_url": null, "last_frame_image_url": null, "reference_image_urls": [], "reference_media": null}
            }),
        ))
        .await
        .expect("secret-free remote video content should remain available");
    assert_eq!(remote_content.status, ServiceStatus::Ok);
    assert_eq!(
        remote_content.payload.expect("remote video payload")["format"],
        "mp4"
    );
    remote_provider.stop().await;

    let failed_provider_addr = unused_local_addr();
    let failed_events_request = provider_request_context(
        "video-process-provider-failure",
        "video-process-provider-failure-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        serde_json::json!({
            "backend": "openai-compatible-video",
            "base_url": sidecar_base_url(failed_provider_addr),
            "api_key": "process-provider-secret",
            "request": {"prompt": "failure test", "negative_prompt": null, "width": 16, "height": 16, "duration_secs": 1, "fps": 1, "model": "fake-video", "first_frame_image_url": null, "last_frame_image_url": null, "reference_image_urls": [], "reference_media": null}
        }),
    );
    let (failed_tx, mut failed_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    client
        .generate_events(failed_events_request, failed_tx)
        .await
        .expect("video provider failure should end in a typed event");
    let mut failed_events = Vec::new();
    while let Some(event) = failed_rx.recv().await {
        failed_events.push(event);
    }
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
    let bytes_request = provider_request_context(
        "video-process-bytes",
        "video-process-bytes-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        serde_json::json!({
            "backend": "openai-compatible-video",
            "base_url": bytes_provider.base_url(),
            "api_key": "process-provider-secret",
            "request": {"prompt": "bytes test", "negative_prompt": null, "width": 16, "height": 16, "duration_secs": 1, "fps": 1, "model": "fake-video", "first_frame_image_url": null, "last_frame_image_url": null, "reference_image_urls": [], "reference_media": null}
        }),
    );
    let bytes_response = client
        .generate(bytes_request)
        .await
        .expect("video bytes response should cross the independent process");
    assert_eq!(bytes_response.status, ServiceStatus::Ok);
    assert_eq!(
        bytes_response.payload.expect("video bytes payload")["format"],
        "mp4"
    );
    bytes_provider.stop().await;

    let provider = FakeJsonProvider::start(serde_json::json!({"id": "process-video-job"})).await;
    let request = provider_request_context(
        "video-process-generate",
        "video-process-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        serde_json::json!({
            "backend": "openai-compatible-video",
            "base_url": provider.base_url(),
            "api_key": "process-provider-secret",
            "request": {"prompt": "process test", "negative_prompt": null, "width": 16, "height": 16, "duration_secs": 1, "fps": 1, "model": "fake-video", "first_frame_image_url": null, "last_frame_image_url": null, "reference_image_urls": [], "reference_media": null}
        }),
    );
    assert_route_rejects_version_mismatch(&sidecar_base_url(addr), "/video/generate", &request)
        .await;
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    client
        .generate_events(request.clone(), event_tx)
        .await
        .expect("video generate events should cross the independent process");
    let mut events = Vec::new();
    while let Some(event) = event_rx.recv().await {
        events.push(event);
    }
    assert_eq!(
        events.last().expect("video completed event").event_kind,
        a4n_service_protocol::CapabilityTaskEventKind::Completed
    );
    let response = client
        .generate(request)
        .await
        .expect("typed video request should cross the independent process");
    assert_eq!(response.status, ServiceStatus::Ok);
    assert_eq!(
        response.payload.expect("video payload")["provider_job_id"],
        "process-video-job"
    );
    provider.stop().await;

    let pending_provider = FakeJsonProvider::start(serde_json::json!({"status": "pending"})).await;
    let pending = client
        .poll(provider_request_context(
            "video-process-poll-pending",
            "video-process-poll-pending-trace",
            VIDEO_PROVIDER_CAPABILITY_VERSION,
            serde_json::json!({
                "backend": "openai-compatible-video",
                "base_url": pending_provider.base_url(),
                "api_key": "process-provider-secret",
                "job_id": "process-video-job"
            }),
        ))
        .await
        .expect("video pending poll should cross the independent process");
    assert_eq!(pending.status, ServiceStatus::Ok);
    assert_eq!(
        pending.payload.expect("pending video payload")["state"],
        "pending"
    );
    pending_provider.stop().await;

    let failed_poll_provider = FakeJsonProvider::start(serde_json::json!({
        "status": "failed",
        "error_message": "token=process-upstream-secret base_url=https://process-provider.invalid/v1"
    }))
    .await;
    let failed_poll = client
        .poll(provider_request_context(
            "video-process-poll-failed",
            "video-process-poll-failed-trace",
            VIDEO_PROVIDER_CAPABILITY_VERSION,
            serde_json::json!({
                "backend": "openai-compatible-video",
                "base_url": failed_poll_provider.base_url(),
                "api_key": "process-provider-secret",
                "job_id": "process-video-job"
            }),
        ))
        .await
        .expect("video failed poll should cross the independent process");
    assert_eq!(failed_poll.status, ServiceStatus::Ok);
    let failed_poll_text = serde_json::to_string(
        failed_poll
            .payload
            .as_ref()
            .expect("failed video poll payload"),
    )
    .expect("failed poll payload should serialize");
    assert_eq!(
        failed_poll.payload.expect("failed video payload")["state"],
        "failed"
    );
    assert!(!failed_poll_text.contains("process-upstream-secret"));
    assert!(!failed_poll_text.contains("process-provider.invalid"));
    failed_poll_provider.stop().await;

    let completed_provider = FakeJsonProvider::start(serde_json::json!({
        "status": "completed",
        "data": [{"b64_json": "bWVkaWE=", "format": "mp4"}]
    }))
    .await;
    let completed = client
        .poll(provider_request_context(
            "video-process-poll-completed",
            "video-process-poll-completed-trace",
            VIDEO_PROVIDER_CAPABILITY_VERSION,
            serde_json::json!({
                "backend": "openai-compatible-video",
                "base_url": completed_provider.base_url(),
                "api_key": "process-provider-secret",
                "job_id": "process-video-job"
            }),
        ))
        .await
        .expect("video completed poll should cross the independent process");
    assert_eq!(completed.status, ServiceStatus::Ok);
    let completed_payload = completed.payload.expect("completed video payload");
    assert_eq!(completed_payload["state"], "completed");
    assert_eq!(completed_payload["result"]["format"], "mp4");
    completed_provider.stop().await;

    let status_provider = FakeJsonProvider::start(serde_json::json!({
        "status": "completed",
        "data": [{"url": "http://127.0.0.1:1/must-not-download", "format": "mp4"}]
    }))
    .await;
    let status = client
        .status(provider_request_context(
            "video-process-status-completed",
            "video-process-status-completed-trace",
            VIDEO_PROVIDER_CAPABILITY_VERSION,
            serde_json::json!({
                "backend": "openai-compatible-video",
                "base_url": status_provider.base_url(),
                "api_key": "process-provider-secret",
                "job_id": "process-video-job"
            }),
        ))
        .await
        .expect("video status should not download completed artifact");
    assert_eq!(status.status, ServiceStatus::Ok);
    let status_payload = status.payload.expect("completed status payload");
    assert_eq!(status_payload["state"], "completed");
    assert!(status_payload.get("result").is_none());
    status_provider.stop().await;

    let missing_provider = FakeJsonProvider::start_with_status(
        StatusCode::NOT_FOUND,
        serde_json::json!({"error": "gone"}),
    )
    .await;
    let missing = client
        .status(provider_request_context(
            "video-process-status-not-found",
            "video-process-status-not-found-trace",
            VIDEO_PROVIDER_CAPABILITY_VERSION,
            serde_json::json!({
                "backend": "openai-compatible-video",
                "base_url": missing_provider.base_url(),
                "api_key": "process-provider-secret",
                "job_id": "missing-video-job"
            }),
        ))
        .await
        .expect("404 status should be idempotent");
    assert_eq!(missing.status, ServiceStatus::Ok);
    assert_eq!(
        missing.payload.expect("not found status payload")["state"],
        "not_found"
    );
    missing_provider.stop().await;

    let delete_provider = FakeJsonProvider::start(serde_json::json!({})).await;
    let delete_request = provider_request_context(
        "video-process-delete",
        "video-process-delete-trace",
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        serde_json::json!({
            "backend": "cliproxy-openai-video",
            "base_url": format!("{}/openai/v1", delete_provider.base_url()),
            "api_key": "process-provider-secret",
            "job_id": "process-video-job"
        }),
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
    assert_eq!(
        deleted.payload.expect("video delete payload")["outcome"],
        "deleted"
    );
    delete_provider.stop().await;

    finish_sidecar(process, &manager, VIDEO_SERVICE_NAME, addr).await;
}
