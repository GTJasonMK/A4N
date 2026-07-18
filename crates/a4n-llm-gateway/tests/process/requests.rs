use a4n_capabilities::{HttpLlmClient, LlmClient};
use a4n_llm::provider::ChatRequest;
use a4n_service_protocol::{
    LLM_SERVICE_NAME, ServiceErrorCode, ServiceStatus, ServiceTaskEvent, ServiceTaskEventKind,
};
use serde_json::{json, to_string};

use super::support::{
    FakeProviderServer, assert_gateway_health, assert_port_released, collect_stream_events,
    gateway_manager, gateway_request, gateway_request_for_model, unused_local_addr,
    wait_until_unavailable,
};

#[tokio::test]
async fn typed_chat_and_stream_cross_independent_gateway_process() {
    let provider = FakeProviderServer::start().await;
    let provider_base_url = provider.base_url();
    let addr = unused_local_addr();
    let manager = gateway_manager(addr);
    let process = manager
        .start_sidecar(LLM_SERVICE_NAME)
        .await
        .expect("manager should start the independent gateway binary");
    let client = HttpLlmClient::new(format!("http://{addr}"));

    let chat_response = client
        .chat(gateway_request(
            "process-chat-request",
            "process-chat-trace",
            &provider_base_url,
            ChatRequest::system_user("system", "chat ping", None, Some(16), None),
        ))
        .await
        .expect("typed chat should cross the independent gateway process");

    let events = collect_stream_events(
        &client,
        gateway_request(
            "process-stream-request",
            "process-stream-trace",
            &provider_base_url,
            ChatRequest::system_user("system", "stream ping", None, Some(16), None)
                .with_stream_usage(),
        ),
    )
    .await;

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

    assert_eq!(chat_response.status, ServiceStatus::Ok);
    assert_eq!(
        chat_response.trace_id.as_deref(),
        Some("process-chat-trace")
    );
    assert_eq!(
        chat_response
            .payload
            .as_ref()
            .map(|payload| &payload["message"]["content"]),
        Some(&json!("process chat ok"))
    );
    assert_eq!(
        chat_response
            .payload
            .as_ref()
            .map(|payload| &payload["usage"]["total_tokens"]),
        Some(&json!(5))
    );

    assert_eq!(events.len(), 3);
    assert_eq!(events[0].task_id, "process-stream-request");
    assert_eq!(events[0].event_kind, ServiceTaskEventKind::Chunk);
    assert_eq!(events[0].chunk_delta.as_deref(), Some("pro"));
    assert_eq!(events[1].event_kind, ServiceTaskEventKind::Chunk);
    assert_eq!(events[1].chunk_delta.as_deref(), Some("cess"));
    assert_eq!(events[1].finish_reason.as_deref(), Some("stop"));
    assert_eq!(events[2].event_kind, ServiceTaskEventKind::Completed);
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
    assert!(provider_requests[0].body.get("stream").is_none());
    assert_eq!(provider_requests[1].body["stream"], json!(true));
    assert_eq!(
        provider_requests[1].body["stream_options"]["include_usage"],
        json!(true)
    );
    assert!(provider_requests.iter().all(|request| {
        request.authorization.as_deref() == Some("Bearer process-provider-secret")
    }));

    assert_output_redacted(
        &format!(
            "{} {}",
            to_string(&chat_response).expect("chat response should serialize"),
            to_string(&events).expect("stream events should serialize")
        ),
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
    let client = HttpLlmClient::new(format!("http://{addr}"));

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
    let recovery_response = client
        .chat(gateway_request(
            "process-recovery-request",
            "process-recovery-trace",
            &provider_base_url,
            ChatRequest::system_user("system", "recovery ping", None, Some(16), None),
        ))
        .await
        .expect("gateway should accept a successful request after provider failure");
    assert_eq!(recovery_response.status, ServiceStatus::Ok);
    assert_eq!(
        recovery_response
            .payload
            .as_ref()
            .map(|payload| &payload["message"]["content"]),
        Some(&json!("process chat ok"))
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
    assert!(provider_requests[1].body.get("stream").is_none());
    assert!(provider_requests.iter().all(|request| {
        request.authorization.as_deref() == Some("Bearer process-provider-secret")
    }));

    assert_output_redacted(
        &to_string(&recovery_response).expect("recovery response should serialize"),
        &provider_base_url,
    );
}

fn assert_one_failed_terminal(events: &[ServiceTaskEvent]) {
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_kind, ServiceTaskEventKind::Failed);
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
            .all(|event| event.event_kind != ServiceTaskEventKind::Completed)
    );
}

fn assert_output_redacted(output: &str, provider_base_url: &str) {
    assert!(!output.contains("process-provider-secret"));
    assert!(!output.contains("process-upstream-secret"));
    assert!(!output.contains("process-provider.invalid"));
    assert!(!output.contains(provider_base_url));
}
