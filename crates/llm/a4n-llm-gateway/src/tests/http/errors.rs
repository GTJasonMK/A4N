use std::sync::{Arc, Mutex};

use a4n_llm_provider_contract::ChatRequest;
use a4n_llm_sidecar_contract::{
    LlmChatRequest, LlmGatewayPayload as WireGatewayPayload, LlmGatewayProviderConfig,
};
use a4n_service_protocol::{CapabilityTaskEvent, CapabilityTaskEventKind, ServiceErrorCode};
use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use futures::stream;
use serde_json::{Value, json};

use super::{
    FakeProviderObservation, TestHttpServer, build_router, collect_stream_events, event_chunk,
    observe_fake_provider_request, service_request,
};

type LlmGatewayPayload = WireGatewayPayload<LlmChatRequest>;
fn malformed_fake_provider_router(observation: FakeProviderObservation) -> Router {
    Router::new()
        .route("/chat/completions", post(malformed_fake_chat_completions))
        .with_state(observation)
}

async fn malformed_fake_chat_completions(
    State(observation): State<FakeProviderObservation>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    observe_fake_provider_request(&observation, &headers, body);

    (
        [(header::CONTENT_TYPE, "text/event-stream")],
        concat!(
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"safe\"},\"finish_reason\":null}]}\n\n",
            "data: api_key=malformed-upstream-secret {\n\n"
        ),
    )
}

fn status_error_fake_provider_router(observation: FakeProviderObservation) -> Router {
    Router::new()
        .route("/chat/completions", post(status_error_chat_completions))
        .with_state(observation)
}

async fn status_error_chat_completions(
    State(observation): State<FakeProviderObservation>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let model = body["model"]
        .as_str()
        .expect("status error test request should include a model")
        .to_string();
    observe_fake_provider_request(&observation, &headers, body);

    let (status, response_body) = match model.as_str() {
        "status-401" => (
            StatusCode::UNAUTHORIZED,
            r#"{"error":{"message":"authorization=Bearer upstream-auth-secret base_url=https://auth-provider.example/v1"}}"#,
        ),
        "status-402" => (
            StatusCode::PAYMENT_REQUIRED,
            r#"{"error":{"message":"credential=upstream-balance-secret base_url=https://balance-provider.example/v1"}}"#,
        ),
        "status-429" => (
            StatusCode::TOO_MANY_REQUESTS,
            r#"{"error":{"message":"api_key=upstream-rate-secret base_url=https://rate-provider.example/v1"}}"#,
        ),
        "status-500" => (
            StatusCode::INTERNAL_SERVER_ERROR,
            r#"{"error":{"message":"token=upstream-server-secret base_url=https://server-provider.example/v1"}}"#,
        ),
        other => panic!("unexpected status test model: {other}"),
    };

    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        response_body,
    )
        .into_response()
}

fn broken_body_fake_provider_router(observation: FakeProviderObservation) -> Router {
    Router::new()
        .route("/chat/completions", post(broken_body_chat_completions))
        .with_state(observation)
}

async fn broken_body_chat_completions(
    State(observation): State<FakeProviderObservation>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let model = body["model"]
        .as_str()
        .expect("broken body test request should include a model")
        .to_string();
    observe_fake_provider_request(&observation, &headers, body);

    let prefix = match model.as_str() {
        "body-error-after-done" => concat!(
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"complete\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n"
        ),
        "body-error-after-length-finish" => concat!(
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"truncated\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"length\"}]}\n\n"
        ),
        "body-error-after-partial-content" => {
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"partial\"},\"finish_reason\":null}]}\n\n"
        }
        "body-error-after-reasoning" => {
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"thinking\"},\"finish_reason\":null}]}\n\n"
        }
        other => panic!("unexpected broken body test model: {other}"),
    };
    let body_stream = stream::unfold(
        (0_u8, Bytes::from_static(prefix.as_bytes())),
        |(state, prefix)| async move {
            match state {
                0 => Some((Ok::<Bytes, std::io::Error>(prefix.clone()), (1, prefix))),
                1 => {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    Some((
                        Err(std::io::Error::new(
                            std::io::ErrorKind::ConnectionReset,
                            "broken provider response body",
                        )),
                        (2, prefix),
                    ))
                }
                _ => None,
            }
        },
    );

    (
        [(header::CONTENT_TYPE, "text/event-stream")],
        Body::from_stream(body_stream),
    )
        .into_response()
}

#[tokio::test]
async fn malformed_provider_sse_emits_one_redacted_failed_terminal_event() {
    let provider_observation = Arc::new(Mutex::new(None));
    let provider =
        TestHttpServer::start(malformed_fake_provider_router(provider_observation.clone())).await;
    let provider_base_url = provider.base_url();
    let gateway = TestHttpServer::start(build_router()).await;
    let client = gateway.client();
    let request = service_request(LlmGatewayPayload {
        provider: LlmGatewayProviderConfig {
            provider: "openai".to_string(),
            base_url: provider_base_url.clone(),
            model: "fake-malformed-model".to_string(),
            api_key: Some("fake-malformed-provider-secret".to_string()),
            timeout_secs: 5,
            extra_body_json: None,
        },
        request: ChatRequest::system_user("system", "malformed ping", None, Some(16)).into(),
    });

    let events = collect_stream_events(&client, request).await;

    gateway.stop().await;
    provider.stop().await;

    let provider_request = provider_observation
        .lock()
        .expect("fake provider observation lock should not be poisoned")
        .clone()
        .expect("gateway should call the malformed fake provider");
    assert_eq!(provider_request.body["stream"], json!(true));
    assert_eq!(
        provider_request.authorization.as_deref(),
        Some("Bearer fake-malformed-provider-secret")
    );

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_kind, CapabilityTaskEventKind::Chunk);
    assert_eq!(events[0].sequence, 1);
    assert_eq!(event_chunk(&events[0]).content.as_deref(), Some("safe"));

    assert_eq!(events[1].event_kind, CapabilityTaskEventKind::Failed);
    assert_eq!(events[1].sequence, 2);
    assert_eq!(
        events[1].error.as_ref().map(|error| error.code.clone()),
        Some(ServiceErrorCode::Internal)
    );
    assert!(events[1].error.as_ref().is_some_and(|error| {
        error.message.contains("解析流式响应 JSON 失败")
            && error.message.contains("api_key=[REDACTED]")
            && error.retryable == Some(false)
    }));
    assert_one_failed_terminal(&events);
    let events_text = serde_json::to_string(&events).expect("stream events should serialize");
    assert!(!events_text.contains("malformed-upstream-secret"));
    assert!(!events_text.contains("fake-malformed-provider-secret"));
    assert!(!events_text.contains(&provider_base_url));
}

#[tokio::test]
async fn provider_http_status_errors_map_to_redacted_failed_terminal_events() {
    struct StatusCase {
        model: &'static str,
        status: u16,
        expected_code: ServiceErrorCode,
        retryable: bool,
        upstream_secret: &'static str,
        upstream_host: &'static str,
    }

    let cases = [
        StatusCase {
            model: "status-401",
            status: 401,
            expected_code: ServiceErrorCode::ConfigurationError,
            retryable: false,
            upstream_secret: "upstream-auth-secret",
            upstream_host: "auth-provider.example",
        },
        StatusCase {
            model: "status-402",
            status: 402,
            expected_code: ServiceErrorCode::ConfigurationError,
            retryable: false,
            upstream_secret: "upstream-balance-secret",
            upstream_host: "balance-provider.example",
        },
        StatusCase {
            model: "status-429",
            status: 429,
            expected_code: ServiceErrorCode::ProviderError,
            retryable: true,
            upstream_secret: "upstream-rate-secret",
            upstream_host: "rate-provider.example",
        },
        StatusCase {
            model: "status-500",
            status: 500,
            expected_code: ServiceErrorCode::ProviderError,
            retryable: true,
            upstream_secret: "upstream-server-secret",
            upstream_host: "server-provider.example",
        },
    ];
    let provider_observation = Arc::new(Mutex::new(None));
    let provider = TestHttpServer::start(status_error_fake_provider_router(
        provider_observation.clone(),
    ))
    .await;
    let provider_base_url = provider.base_url();
    let gateway = TestHttpServer::start(build_router()).await;
    let client = gateway.client();

    for case in cases {
        let request = service_request(LlmGatewayPayload {
            provider: LlmGatewayProviderConfig {
                provider: "openai".to_string(),
                base_url: provider_base_url.clone(),
                model: case.model.to_string(),
                api_key: Some("fake-status-provider-secret".to_string()),
                timeout_secs: 5,
                extra_body_json: None,
            },
            request: ChatRequest::system_user("system", "status ping", None, Some(16)).into(),
        });
        let events = collect_stream_events(&client, request).await;

        assert_eq!(events.len(), 1);
        assert_one_failed_terminal(&events);
        let error = events[0]
            .error
            .as_ref()
            .expect("Failed terminal event should carry an error");
        assert_eq!(error.code, case.expected_code);
        assert_eq!(error.retryable, Some(case.retryable));
        assert!(error.message.contains(&case.status.to_string()));
        assert!(error.message.contains("[REDACTED]"));

        let events_text = serde_json::to_string(&events).expect("stream events should serialize");
        assert!(!events_text.contains(case.upstream_secret));
        assert!(!events_text.contains(case.upstream_host));
        assert!(!events_text.contains("fake-status-provider-secret"));
        assert!(!events_text.contains(&provider_base_url));

        let provider_request = provider_observation
            .lock()
            .expect("fake provider observation lock should not be poisoned")
            .clone()
            .expect("gateway should call the status error fake provider");
        assert_eq!(provider_request.body["model"], json!(case.model));
        assert_eq!(provider_request.body["stream"], json!(true));
        assert_eq!(
            provider_request.authorization.as_deref(),
            Some("Bearer fake-status-provider-secret")
        );
    }

    gateway.stop().await;
    provider.stop().await;
}

#[tokio::test]
async fn provider_body_error_after_done_keeps_completed_result() {
    let provider_observation = Arc::new(Mutex::new(None));
    let provider = TestHttpServer::start(broken_body_fake_provider_router(
        provider_observation.clone(),
    ))
    .await;
    let gateway = TestHttpServer::start(build_router()).await;
    let client = gateway.client();
    let request = service_request(LlmGatewayPayload {
        provider: LlmGatewayProviderConfig {
            provider: "openai".to_string(),
            base_url: provider.base_url(),
            model: "body-error-after-done".to_string(),
            api_key: None,
            timeout_secs: 5,
            extra_body_json: None,
        },
        request: ChatRequest::system_user("system", "done ping", None, Some(16)).into(),
    });

    let events = collect_stream_events(&client, request).await;

    gateway.stop().await;
    provider.stop().await;

    assert_eq!(events.len(), 3);
    assert_eq!(events[0].event_kind, CapabilityTaskEventKind::Chunk);
    assert_eq!(event_chunk(&events[0]).content.as_deref(), Some("complete"));
    assert_eq!(events[1].event_kind, CapabilityTaskEventKind::Chunk);
    assert_eq!(
        event_chunk(&events[1]).finish_reason.as_deref(),
        Some("stop")
    );
    assert_eq!(events[2].event_kind, CapabilityTaskEventKind::Completed);
    assert_eq!(events[2].payload, None);
}

#[tokio::test]
async fn provider_body_error_after_finish_preserves_truncation_contract() {
    let provider_observation = Arc::new(Mutex::new(None));
    let provider = TestHttpServer::start(broken_body_fake_provider_router(
        provider_observation.clone(),
    ))
    .await;
    let gateway = TestHttpServer::start(build_router()).await;
    let client = gateway.client();
    let request = service_request(LlmGatewayPayload {
        provider: LlmGatewayProviderConfig {
            provider: "openai".to_string(),
            base_url: provider.base_url(),
            model: "body-error-after-length-finish".to_string(),
            api_key: None,
            timeout_secs: 5,
            extra_body_json: None,
        },
        request: ChatRequest::system_user("system", "length ping", None, Some(16)).into(),
    });

    let events = collect_stream_events(&client, request).await;

    gateway.stop().await;
    provider.stop().await;

    assert_eq!(events.len(), 3);
    assert_eq!(events[2].event_kind, CapabilityTaskEventKind::Completed);
    assert_eq!(
        event_chunk(&events[1]).finish_reason.as_deref(),
        Some("length")
    );
    assert_eq!(events[2].payload, None);
}

#[tokio::test]
async fn provider_body_error_before_terminal_marker_remains_failed() {
    for model in [
        "body-error-after-partial-content",
        "body-error-after-reasoning",
    ] {
        let provider_observation = Arc::new(Mutex::new(None));
        let provider = TestHttpServer::start(broken_body_fake_provider_router(
            provider_observation.clone(),
        ))
        .await;
        let gateway = TestHttpServer::start(build_router()).await;
        let client = gateway.client();
        let request = service_request(LlmGatewayPayload {
            provider: LlmGatewayProviderConfig {
                provider: "openai".to_string(),
                base_url: provider.base_url(),
                model: model.to_string(),
                api_key: None,
                timeout_secs: 5,
                extra_body_json: None,
            },
            request: ChatRequest::system_user("system", "partial ping", None, Some(16)).into(),
        });

        let events = collect_stream_events(&client, request).await;

        gateway.stop().await;
        provider.stop().await;

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_kind, CapabilityTaskEventKind::Chunk);
        assert_one_failed_terminal(&events);
        let error = events[1]
            .error
            .as_ref()
            .expect("body read failure should carry provider error");
        assert_eq!(error.code, ServiceErrorCode::ProviderError);
        assert_eq!(error.retryable, Some(true));
        assert!(error.message.contains("LLM 流式响应体解码失败"));
        assert!(error.message.contains(model));
        assert!(error.message.contains("received_bytes="));
        assert!(error.message.contains("parsed_chunks=1"));
        assert!(error.message.contains("request or response body error"));
        assert!(error.message.contains("unexpected EOF"));
    }
}

#[tokio::test]
async fn incomplete_or_interleaved_provider_output_cannot_complete_gateway_stream() {
    for (tail, expected_error) in [
        ("data: [DONE]\n\n", "finish_reason"),
        (
            "data: {\"choices\":[{\"index\":1,\"delta\":{\"content\":\"other\"},\"finish_reason\":\"stop\"}]}\n\n",
            "一个候选",
        ),
    ] {
        let response = format!(
            "data: {{\"choices\":[{{\"index\":0,\"delta\":{{\"content\":\"partial\"}},\"finish_reason\":null}}]}}\n\n{tail}"
        );
        let provider = TestHttpServer::start(Router::new().route(
            "/chat/completions",
            post(move || {
                let response = response.clone();
                async move { ([(header::CONTENT_TYPE, "text/event-stream")], response) }
            }),
        ))
        .await;
        let gateway = TestHttpServer::start(build_router()).await;
        let request = service_request(LlmGatewayPayload {
            provider: LlmGatewayProviderConfig {
                base_url: provider.base_url(),
                api_key: None,
                ..super::test_provider_config()
            },
            request: ChatRequest::system_user("system", "protocol ping", None, Some(16)).into(),
        });
        let events = collect_stream_events(&gateway.client(), request).await;
        gateway.stop().await;
        provider.stop().await;

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_kind, CapabilityTaskEventKind::Chunk);
        assert_one_failed_terminal(&events);
        assert!(
            events[1]
                .error
                .as_ref()
                .unwrap()
                .message
                .contains(expected_error)
        );
    }
}

fn assert_one_failed_terminal(events: &[CapabilityTaskEvent]) {
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_kind.is_terminal())
            .count(),
        1
    );
    assert_eq!(
        events.last().map(|event| &event.event_kind),
        Some(&CapabilityTaskEventKind::Failed)
    );
    assert!(
        events
            .iter()
            .all(|event| event.event_kind != CapabilityTaskEventKind::Completed)
    );
}
