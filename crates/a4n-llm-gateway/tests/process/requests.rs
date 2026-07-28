use a4n_capabilities::{CapabilityClient, LlmClient};
use a4n_llm::provider::ChatRequest;
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, LLM_SERVICE_NAME, PROTOCOL_VERSION,
    ServiceErrorCode, ServiceStatus, sidecar_resource_policy,
};
use a4n_test_support::process::{
    assert_protected_route_rejects_invalid_auth, assert_route_rejects_malformed_json,
    assert_route_rejects_oversized_json, assert_route_rejects_unknown_payload_field,
    hold_partial_requests,
};
use serde_json::{json, to_string};
use std::time::Duration;
use tokio::time::timeout;

use super::support::{
    FakeProviderServer, assert_gateway_health, assert_port_released, collect_stream_events,
    gateway_client, gateway_manager, gateway_models_request, gateway_probe_request,
    gateway_remote_request_for_model, gateway_request, gateway_request_for_model,
    unused_local_addr, wait_until_unavailable,
};

#[tokio::test]
async fn bounded_concurrency_backpressures_saturated_gateway() {
    let addr = unused_local_addr();
    let manager = gateway_manager(addr);
    let process = manager
        .start_sidecar(LLM_SERVICE_NAME)
        .await
        .expect("manager should start the independent gateway binary");
    let policy = sidecar_resource_policy(LLM_SERVICE_NAME).expect("llm resource policy");
    let held = hold_partial_requests(addr, "/llm/stream", policy.max_concurrent_requests).await;

    let saturated = timeout(Duration::from_millis(250), gateway_client(addr).health()).await;
    assert!(
        saturated.is_err(),
        "health should wait while all sidecar concurrency permits are held"
    );

    drop(held);
    let health = timeout(Duration::from_secs(2), gateway_client(addr).health())
        .await
        .expect("health should recover after partial requests close")
        .expect("health request should succeed after backpressure clears");
    assert_eq!(health.service, LLM_SERVICE_NAME);

    process
        .stop()
        .await
        .expect("independent gateway should stop after concurrency test");
}

#[tokio::test]
async fn stream_response_headers_have_a_connect_deadline_when_gateway_is_saturated() {
    let addr = unused_local_addr();
    let manager = gateway_manager(addr);
    let process = manager
        .start_sidecar(LLM_SERVICE_NAME)
        .await
        .expect("manager should start the independent gateway binary");
    let policy = sidecar_resource_policy(LLM_SERVICE_NAME).expect("llm resource policy");
    let held = hold_partial_requests(addr, "/llm/stream", policy.max_concurrent_requests).await;
    let (events, _receiver) = tokio::sync::mpsc::channel(a4n_capabilities::EVENT_QUEUE_CAPACITY);

    let error = gateway_client(addr)
        .stream(
            gateway_request_for_model(
                "process-connect-timeout-request",
                "process-connect-timeout-trace",
                "http://127.0.0.1:1",
                "process-connect-timeout-model",
                ChatRequest::system_user("system", "connect timeout", None, Some(16), None),
            ),
            events,
        )
        .await
        .expect_err("a saturated gateway must hit the stream response-header deadline");

    assert_eq!(error.code, ServiceErrorCode::ServiceUnavailable);
    assert!(error.message.contains("event stream connect timeout"));
    drop(held);
    assert_gateway_health(&manager, addr).await;
    process
        .stop()
        .await
        .expect("independent gateway should stop after connect timeout test");
    assert_port_released(addr);
}

#[tokio::test]
async fn event_queue_backpressure_waits_for_capacity_across_independent_gateway_process() {
    let provider = FakeProviderServer::start().await;
    let provider_base_url = provider.base_url();
    let addr = unused_local_addr();
    let manager = gateway_manager(addr);
    let process = manager
        .start_sidecar(LLM_SERVICE_NAME)
        .await
        .expect("manager should start the independent gateway binary");
    let client = gateway_client(addr);
    let (event_tx, mut event_rx) =
        tokio::sync::mpsc::channel(a4n_capabilities::EVENT_QUEUE_CAPACITY);

    let stream = tokio::spawn(async move {
        client
            .stream(
                gateway_request_for_model(
                    "process-backpressure-request",
                    "process-backpressure-trace",
                    &provider_base_url,
                    "process-burst-model",
                    ChatRequest::system_user("system", "backpressure ping", None, Some(16), None),
                ),
                event_tx,
            )
            .await
    });
    tokio::task::yield_now().await;
    assert!(
        !stream.is_finished(),
        "a full event queue must apply backpressure"
    );

    let mut chunk_count = 0;
    loop {
        let event = event_rx.recv().await.expect("stream terminal event");
        if event.event_kind == CapabilityTaskEventKind::Chunk {
            chunk_count += 1;
        }
        if event.event_kind.is_terminal() {
            assert_eq!(event.event_kind, CapabilityTaskEventKind::Completed);
            break;
        }
    }
    stream
        .await
        .expect("stream task should join")
        .expect("stream should finish after queue capacity is released");
    assert!(chunk_count > a4n_capabilities::EVENT_QUEUE_CAPACITY);

    assert_gateway_health(&manager, addr).await;
    process
        .stop()
        .await
        .expect("independent gateway should stop after backpressure test");
    assert_port_released(addr);
    provider.stop().await;
}

#[tokio::test]
async fn remote_egress_rejects_provider_secret_and_allows_content_without_it() {
    let provider = FakeProviderServer::start().await;
    let provider_base_url = provider.base_url();
    let addr = unused_local_addr();
    let manager = gateway_manager(addr);
    let process = manager
        .start_sidecar(LLM_SERVICE_NAME)
        .await
        .expect("manager should start the independent gateway binary");
    let client = gateway_client(addr);

    let rejected = collect_stream_events(
        &client,
        gateway_remote_request_for_model(
            "process-remote-secret-request",
            "process-remote-secret-trace",
            &provider_base_url,
            "process-fake-model",
            Some("process-provider-secret"),
            ChatRequest::system_user("system", "remote secret must stop", None, Some(16), None),
        ),
    )
    .await;
    assert_one_failed_terminal(&rejected);
    let error = rejected[0]
        .error
        .as_ref()
        .expect("remote provider secret error");
    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(!error.message.contains("process-provider-secret"));
    assert!(!error.message.contains(&provider_base_url));

    let allowed = collect_stream_events(
        &client,
        gateway_remote_request_for_model(
            "process-remote-content-request",
            "process-remote-content-trace",
            &provider_base_url,
            "process-fake-model",
            None,
            ChatRequest::system_user("system", "remote content is explicit", None, Some(16), None),
        ),
    )
    .await;
    assert_eq!(
        allowed.last().map(|event| &event.event_kind),
        Some(&CapabilityTaskEventKind::Completed)
    );

    assert_gateway_health(&manager, addr).await;
    process
        .stop()
        .await
        .expect("independent gateway should stop after egress test");
    assert_port_released(addr);

    let provider_requests = provider.requests();
    provider.stop().await;
    assert_eq!(provider_requests.len(), 1);
    assert_eq!(
        provider_requests[0].body["model"],
        json!("process-fake-model")
    );
    assert_eq!(
        provider_requests[0].body["messages"][1]["content"],
        json!("remote content is explicit")
    );
    assert!(provider_requests[0].authorization.is_none());
}

#[tokio::test]
async fn typed_stream_crosses_independent_gateway_process() {
    let provider = FakeProviderServer::start().await;
    let provider_base_url = provider.base_url();
    let addr = unused_local_addr();
    let manager = gateway_manager(addr);
    let process = manager
        .start_sidecar(LLM_SERVICE_NAME)
        .await
        .expect("manager should start the independent gateway binary");
    let client = gateway_client(addr);
    let gateway_base_url = format!("http://{addr}");
    assert_protected_route_rejects_invalid_auth(&gateway_base_url, "/llm/stream").await;
    assert_route_rejects_malformed_json(&gateway_base_url, "/llm/stream").await;
    assert_route_rejects_oversized_json(&gateway_base_url, "/llm/stream", 8 * 1024 * 1024).await;

    let stream_request = gateway_request(
        "process-stream-request",
        "process-stream-trace",
        &provider_base_url,
        ChatRequest::system_user("system", "stream ping", None, Some(16), None).with_stream_usage(),
    );
    assert_route_rejects_unknown_payload_field(&gateway_base_url, "/llm/stream", &stream_request)
        .await;

    let mut incompatible_capability = stream_request.clone();
    incompatible_capability.capability_version = "process-incompatible-v0".to_string();
    let mut incompatible_protocol = stream_request.clone();
    incompatible_protocol.schema_version.protocol = PROTOCOL_VERSION + 1;
    for request in [incompatible_capability, incompatible_protocol] {
        let events = collect_stream_events(&client, request).await;
        assert_one_failed_terminal(&events);
        assert_eq!(
            events[0].error.as_ref().map(|error| error.code.clone()),
            Some(ServiceErrorCode::VersionMismatch)
        );
    }

    let probe_response = client
        .probe(gateway_probe_request(
            "process-probe-request",
            "process-probe-trace",
            &provider_base_url,
        ))
        .await
        .expect("typed probe should cross the independent gateway process");
    let models_response = client
        .discover_models(gateway_models_request(
            "process-models-request",
            "process-models-trace",
            &provider_base_url,
        ))
        .await
        .expect("typed models request should cross the independent gateway process");

    let events = collect_stream_events(&client, stream_request).await;

    process
        .stop()
        .await
        .expect("independent gateway process should stop after requests");
    let error = wait_until_unavailable(&manager).await;
    assert_eq!(
        error.protocol_error_code(),
        ServiceErrorCode::ServiceUnavailable
    );
    assert_port_released(addr);

    let provider_requests = provider.requests();
    provider.stop().await;

    assert_eq!(probe_response.status, ServiceStatus::Ok);
    assert_eq!(
        probe_response
            .payload
            .as_ref()
            .map(|payload| &payload["ok"]),
        Some(&json!(true))
    );
    assert_eq!(models_response.status, ServiceStatus::Ok);
    assert_eq!(
        models_response
            .payload
            .as_ref()
            .and_then(|payload| payload["models"].as_array())
            .map(Vec::len),
        Some(1)
    );

    assert_eq!(events.len(), 3);
    assert_eq!(events[0].task_id, "process-stream-request");
    assert_eq!(events[0].event_kind, CapabilityTaskEventKind::Chunk);
    assert_eq!(events[0].chunk_delta.as_deref(), Some("pro"));
    assert_eq!(events[1].event_kind, CapabilityTaskEventKind::Chunk);
    assert_eq!(events[1].chunk_delta.as_deref(), Some("cess"));
    assert_eq!(events[1].finish_reason.as_deref(), Some("stop"));
    assert_eq!(events[2].event_kind, CapabilityTaskEventKind::Completed);
    assert_eq!(
        events[2]
            .final_usage
            .as_ref()
            .map(|usage| &usage["total_tokens"]),
        Some(&json!(6))
    );
    assert_eq!(events[2].finish_reason.as_deref(), Some("stop"));
    assert_eq!(events[2].error, None);

    assert_eq!(provider_requests.len(), 2);
    assert_eq!(
        provider_requests[0].body["model"],
        json!("process-fake-model")
    );
    assert_eq!(provider_requests[0].body["stream"], json!(true));
    assert_eq!(provider_requests[1].body["stream"], json!(true));
    assert_eq!(
        provider_requests[1].body["stream_options"]["include_usage"],
        json!(true)
    );
    assert!(provider_requests.iter().all(|request| {
        request.authorization.as_deref() == Some("Bearer process-provider-secret")
    }));

    assert_output_redacted(
        &to_string(&events).expect("stream events should serialize"),
        &provider_base_url,
    );
}

#[tokio::test]
async fn provider_failure_is_isolated_from_independent_gateway_process() {
    let provider = FakeProviderServer::start().await;
    let provider_base_url = provider.base_url();
    let addr = unused_local_addr();
    let manager = gateway_manager(addr);
    let process = manager
        .start_sidecar(LLM_SERVICE_NAME)
        .await
        .expect("manager should start the independent gateway binary");
    let client = gateway_client(addr);

    let events = collect_stream_events(
        &client,
        gateway_request_for_model(
            "process-error-request",
            "process-error-trace",
            &provider_base_url,
            "process-error-500",
            ChatRequest::system_user("system", "error ping", None, Some(16), None),
        ),
    )
    .await;

    assert_one_failed_terminal(&events);
    let failure = events[0]
        .error
        .as_ref()
        .expect("Failed terminal event should carry an error");
    assert_eq!(failure.code, ServiceErrorCode::ProviderError);
    assert_eq!(failure.retryable, Some(true));
    assert!(failure.message.contains("500"));
    assert!(failure.message.contains("[REDACTED]"));
    assert_eq!(events[0].task_id, "process-error-request");
    assert_output_redacted(
        &to_string(&events).expect("failed events should serialize"),
        &provider_base_url,
    );

    assert_gateway_health(&manager, addr).await;
    let recovery_events = collect_stream_events(
        &client,
        gateway_request(
            "process-recovery-request",
            "process-recovery-trace",
            &provider_base_url,
            ChatRequest::system_user("system", "recovery ping", None, Some(16), None),
        ),
    )
    .await;
    assert_eq!(
        recovery_events.last().map(|event| &event.event_kind),
        Some(&CapabilityTaskEventKind::Completed)
    );
    assert_gateway_health(&manager, addr).await;

    process
        .stop()
        .await
        .expect("independent gateway should stop after failure recovery");
    let error = wait_until_unavailable(&manager).await;
    assert_eq!(
        error.protocol_error_code(),
        ServiceErrorCode::ServiceUnavailable
    );
    assert_port_released(addr);

    let provider_requests = provider.requests();
    provider.stop().await;

    assert_eq!(provider_requests.len(), 2);
    assert_eq!(
        provider_requests[0].body["model"],
        json!("process-error-500")
    );
    assert_eq!(provider_requests[0].body["stream"], json!(true));
    assert_eq!(
        provider_requests[1].body["model"],
        json!("process-fake-model")
    );
    assert_eq!(provider_requests[1].body["stream"], json!(true));
    assert!(provider_requests.iter().all(|request| {
        request.authorization.as_deref() == Some("Bearer process-provider-secret")
    }));

    assert_output_redacted(
        &to_string(&recovery_events).expect("recovery events should serialize"),
        &provider_base_url,
    );
}

fn assert_one_failed_terminal(events: &[CapabilityTaskEvent]) {
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_kind, CapabilityTaskEventKind::Failed);
    assert_eq!(events[0].sequence, 1);
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_kind.is_terminal())
            .count(),
        1
    );
    assert!(
        events
            .iter()
            .all(|event| event.event_kind != CapabilityTaskEventKind::Completed)
    );
}

fn assert_output_redacted(output: &str, provider_base_url: &str) {
    assert!(!output.contains("process-provider-secret"));
    assert!(!output.contains("process-upstream-secret"));
    assert!(!output.contains("process-provider.invalid"));
    assert!(!output.contains(provider_base_url));
}
