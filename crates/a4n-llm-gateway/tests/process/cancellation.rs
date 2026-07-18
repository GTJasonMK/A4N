use std::time::Duration;

use a4n_capabilities::{HttpLlmClient, LlmClient};
use a4n_llm::provider::ChatRequest;
use a4n_service_protocol::{LLM_SERVICE_NAME, ServiceErrorCode, ServiceStatus};
use serde_json::json;
use tokio::time::timeout;

use super::{
    hanging_provider::HangingProviderServer,
    support::{
        FakeProviderServer, assert_gateway_health, assert_port_released, gateway_manager,
        gateway_request, gateway_request_for_model, unused_local_addr, wait_until_unavailable,
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
    let client = HttpLlmClient::new(format!("http://{addr}"));
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
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
    let recovery_response = client
        .chat(gateway_request(
            "process-cancel-recovery-request",
            "process-cancel-recovery-trace",
            &recovery_provider_base_url,
            ChatRequest::system_user("system", "recovery ping", None, Some(16), None),
        ))
        .await
        .expect("gateway should accept a request after cross-process cancellation");
    assert_eq!(recovery_response.status, ServiceStatus::Ok);
    assert_eq!(
        recovery_response
            .payload
            .as_ref()
            .map(|payload| &payload["message"]["content"]),
        Some(&json!("process chat ok"))
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
