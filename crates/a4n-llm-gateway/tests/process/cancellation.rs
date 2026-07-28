use std::time::Duration;

use a4n_capabilities::{EVENT_QUEUE_CAPACITY, LlmClient};
use a4n_llm::provider::ChatRequest;
use a4n_service_protocol::{CapabilityTaskEventKind, LLM_SERVICE_NAME, ServiceErrorCode};
use serde_json::json;
use tokio::time::timeout;

use super::{
    hanging_provider::HangingProviderServer,
    support::{
        FakeProviderServer, assert_gateway_health, assert_port_released, collect_stream_events,
        gateway_client, gateway_client_with_test_timeouts, gateway_manager, gateway_request,
        gateway_request_for_model, unused_local_addr, wait_until_unavailable,
    },
};

#[tokio::test]
async fn closed_receiver_cancels_stream_across_independent_gateway_process() {
    let (hanging_provider, provider_body_dropped) = HangingProviderServer::start().await;
    let hanging_provider_base_url = hanging_provider.base_url();
    let addr = unused_local_addr();
    let manager = gateway_manager(addr);
    let process = manager
        .start_sidecar(LLM_SERVICE_NAME)
        .await
        .expect("manager should start the independent gateway binary");
    let client = gateway_client(addr);
    let (event_tx, event_rx) = tokio::sync::mpsc::channel(EVENT_QUEUE_CAPACITY);
    drop(event_rx);

    let error = timeout(
        Duration::from_secs(5),
        client.stream(
            gateway_request_for_model(
                "process-cancel-request",
                "process-cancel-trace",
                &hanging_provider_base_url,
                "process-cancel-model",
                ChatRequest::system_user("system", "cancel ping", None, Some(16), None),
            ),
            event_tx,
        ),
    )
    .await
    .expect("typed stream should observe the closed receiver before timeout")
    .expect_err("closed event receiver must cancel the typed stream");
    timeout(Duration::from_secs(5), provider_body_dropped)
        .await
        .expect("provider response body should be dropped before timeout")
        .expect("provider body drop signal should be delivered");

    assert_eq!(error.code, ServiceErrorCode::Cancelled);
    assert_eq!(error.retryable, Some(false));
    assert!(error.message.contains("event receiver 已关闭"));
    assert!(!error.message.contains("process-provider-secret"));
    assert!(!error.message.contains(&hanging_provider_base_url));

    let hanging_request = hanging_provider.request();
    assert_eq!(hanging_request.body["model"], json!("process-cancel-model"));
    assert_eq!(hanging_request.body["stream"], json!(true));
    assert_eq!(
        hanging_request.authorization.as_deref(),
        Some("Bearer process-provider-secret")
    );
    hanging_provider.stop().await;

    assert_gateway_health(&manager, addr).await;
    let recovery_provider = FakeProviderServer::start().await;
    let recovery_provider_base_url = recovery_provider.base_url();
    let recovery_events = collect_stream_events(
        &client,
        gateway_request(
            "process-cancel-recovery-request",
            "process-cancel-recovery-trace",
            &recovery_provider_base_url,
            ChatRequest::system_user("system", "recovery ping", None, Some(16), None),
        ),
    )
    .await;
    assert_eq!(
        recovery_events.last().map(|event| &event.event_kind),
        Some(&CapabilityTaskEventKind::Completed)
    );
    assert_gateway_health(&manager, addr).await;
    recovery_provider.stop().await;

    process
        .stop()
        .await
        .expect("independent gateway should stop after cancellation recovery");
    let unavailable = wait_until_unavailable(&manager).await;
    assert_eq!(
        unavailable.protocol_error_code(),
        ServiceErrorCode::ServiceUnavailable
    );
    assert_port_released(addr);
}

#[tokio::test]
async fn provider_idle_timeout_emits_failed_terminal_event_and_recovers() {
    let (hanging_provider, provider_body_dropped) = HangingProviderServer::start().await;
    let hanging_provider_base_url = hanging_provider.base_url();
    let addr = unused_local_addr();
    let manager = gateway_manager(addr);
    let process = manager
        .start_sidecar(LLM_SERVICE_NAME)
        .await
        .expect("manager should start the independent gateway binary");
    let client = gateway_client(addr);
    let mut request = gateway_request_for_model(
        "process-timeout-request",
        "process-timeout-trace",
        &hanging_provider_base_url,
        "process-timeout-model",
        ChatRequest::system_user("system", "timeout ping", None, Some(16), None),
    );
    request.payload["provider"]["timeout_secs"] = json!(1);

    let events = timeout(
        Duration::from_secs(5),
        collect_stream_events(&client, request),
    )
    .await
    .expect("provider idle timeout should produce a terminal event promptly");
    timeout(Duration::from_secs(5), provider_body_dropped)
        .await
        .expect("provider response body should be dropped after idle timeout")
        .expect("provider body drop signal should be delivered");

    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0].event_kind,
        a4n_service_protocol::CapabilityTaskEventKind::Chunk
    );
    assert_eq!(
        events[1].event_kind,
        a4n_service_protocol::CapabilityTaskEventKind::Failed
    );
    assert_eq!(
        events[1].error.as_ref().expect("failed event error").code,
        ServiceErrorCode::ServiceUnavailable
    );
    assert!(
        !events[1]
            .error
            .as_ref()
            .expect("failed event error")
            .message
            .contains(&hanging_provider_base_url)
    );

    hanging_provider.stop().await;
    assert_gateway_health(&manager, addr).await;
    let recovery_provider = FakeProviderServer::start().await;
    let recovery_events = collect_stream_events(
        &client,
        gateway_request(
            "process-timeout-recovery-request",
            "process-timeout-recovery-trace",
            &recovery_provider.base_url(),
            ChatRequest::system_user("system", "recovery ping", None, Some(16), None),
        ),
    )
    .await;
    assert_eq!(
        recovery_events.last().map(|event| &event.event_kind),
        Some(&CapabilityTaskEventKind::Completed)
    );
    recovery_provider.stop().await;

    process
        .stop()
        .await
        .expect("independent gateway should stop after idle timeout recovery");
    let unavailable = wait_until_unavailable(&manager).await;
    assert_eq!(
        unavailable.protocol_error_code(),
        ServiceErrorCode::ServiceUnavailable
    );
    assert_port_released(addr);
}

#[tokio::test]
async fn stream_idle_timeout_releases_provider_body_across_independent_process() {
    let (hanging_provider, provider_body_dropped) = HangingProviderServer::start().await;
    let hanging_provider_base_url = hanging_provider.base_url();
    let addr = unused_local_addr();
    let manager = gateway_manager(addr);
    let process = manager
        .start_sidecar(LLM_SERVICE_NAME)
        .await
        .expect("manager should start the independent gateway binary");
    let client = gateway_client_with_test_timeouts(addr);
    let mut request = gateway_request_for_model(
        "process-stream-idle-request",
        "process-stream-idle-trace",
        &hanging_provider_base_url,
        "process-stream-idle-model",
        ChatRequest::system_user("system", "stream idle ping", None, Some(16), None),
    );
    request.payload["provider"]["timeout_secs"] = json!(120);
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(EVENT_QUEUE_CAPACITY);

    let error = timeout(Duration::from_secs(5), client.stream(request, event_tx))
        .await
        .expect("stream idle timeout should be observed promptly")
        .expect_err("a stalled sidecar stream must return an idle timeout");

    assert_eq!(error.code, ServiceErrorCode::ServiceUnavailable);
    assert!(error.message.contains("event stream idle timeout"));
    assert_eq!(
        event_rx
            .recv()
            .await
            .expect("the provider's first chunk should reach the client")
            .event_kind,
        a4n_service_protocol::CapabilityTaskEventKind::Chunk
    );
    timeout(Duration::from_secs(5), provider_body_dropped)
        .await
        .expect("provider response body should be dropped after stream idle timeout")
        .expect("provider body drop signal should be delivered");

    assert_gateway_health(&manager, addr).await;
    process
        .stop()
        .await
        .expect("independent gateway should stop after stream idle timeout");
    assert_port_released(addr);
    hanging_provider.stop().await;
}
