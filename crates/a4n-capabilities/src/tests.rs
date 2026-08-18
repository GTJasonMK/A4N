use super::contracts::*;
use super::embedding::*;
use super::llm::*;
use super::media::*;
use super::package_io::*;
use super::prompt::*;
use super::rag::*;
use super::transport::*;
use a4n_service_protocol::{
    CapabilityTaskEventKind, EMBEDDING_MODEL_DOWNLOAD_EVENTS_PATH, EMBEDDING_SERVICE_NAME,
    HealthStatus, IMAGE_SERVICE_NAME, IMPORT_SERVICE_NAME, LLM_SERVICE_NAME, PROMPT_SERVICE_NAME,
    PROTOCOL_VERSION, RAG_SERVICE_NAME, SIDECAR_EVENT_CONTENT_TYPE, SIDECAR_EVENT_QUEUE_CAPACITY,
    SIDECAR_HEALTH_PATH, SIDECAR_REQUEST_ID_HEADER, SIDECAR_TRACE_ID_HEADER, ServiceErrorCode,
    ServiceRequest, TraceContext, VIDEO_SERVICE_NAME,
    expected_capability_version_for_service,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{io::AsyncWriteExt, net::TcpListener};

#[test]
fn sidecar_transport_joins_paths_without_double_slashes() {
    let transport = HttpSidecarTransport::new("http://127.0.0.1:47641/", LLM_SERVICE_NAME);

    assert_eq!(
        transport.url(SIDECAR_HEALTH_PATH),
        "http://127.0.0.1:47641/healthz"
    );
}

#[test]
fn sidecar_transport_debug_never_exposes_url_or_token() {
    let transport = HttpSidecarTransport::new(
        "https://user:secret@sidecar.example.test",
        LLM_SERVICE_NAME,
    )
    .with_auth_token("manager-issued-secret-token");
    let debug = format!("{transport:?}");
    assert!(!debug.contains("sidecar.example.test"));
    assert!(!debug.contains("manager-issued-secret-token"));
    assert!(debug.contains("auth_token_configured: true"));
}

#[test]
fn sidecar_transport_propagates_request_and_trace_headers_without_body_duplication() {
    let transport =
        HttpSidecarTransport::new("http://127.0.0.1:47641", LLM_SERVICE_NAME)
            .with_auth_token("manager-token");
    let request = ServiceRequest {
        request_id: "request-42".to_string(),
        trace_context: TraceContext {
            trace_id: "workflow:42".to_string(),
            parent_span_id: None,
        },
        schema_version: Default::default(),
        user_id: None,
        project_id: None,
        root_project_id: None,
        runtime_config_snapshot: None,
        secret_policy: a4n_service_protocol::SecretForwardingPolicy::deny_all(),
        capability_version: "demo".to_string(),
        payload: serde_json::json!({"safe": true}),
    };
    let built = transport
        .authorize_request(transport.client.post(transport.url("/trace")), &request)
        .expect("request context IDs should be valid HTTP headers")
        .json(&request)
        .build()
        .unwrap();

    assert_eq!(
        built
            .headers()
            .get(a4n_service_protocol::SIDECAR_AUTH_HEADER)
            .and_then(|value| value.to_str().ok()),
        Some("manager-token")
    );

    assert_eq!(
        built
            .headers()
            .get(SIDECAR_REQUEST_ID_HEADER)
            .and_then(|value| value.to_str().ok()),
        Some("request-42")
    );
    assert_eq!(
        built
            .headers()
            .get(SIDECAR_TRACE_ID_HEADER)
            .and_then(|value| value.to_str().ok()),
        Some("workflow:42")
    );
}

#[test]
fn sidecar_transport_authorizes_health_requests() {
    let transport =
        HttpSidecarTransport::new("https://sidecar.example.test", LLM_SERVICE_NAME)
            .with_auth_token("manager-token");
    let request = transport
        .authorize(transport.client.get(transport.url(SIDECAR_HEALTH_PATH)))
        .build()
        .expect("health request should build");

    assert_eq!(
        request
            .headers()
            .get(a4n_service_protocol::SIDECAR_AUTH_HEADER)
            .and_then(|value| value.to_str().ok()),
        Some("manager-token")
    );
}

#[tokio::test]
async fn event_stream_response_headers_have_a_connect_deadline() {
    let (addr, server) =
        delayed_http_server(Duration::from_millis(100), Vec::new(), Duration::ZERO).await;
    let transport = HttpSidecarTransport::new(format!("http://{addr}"), LLM_SERVICE_NAME).with_test_timeouts(
        Duration::from_millis(20),
        Duration::from_secs(1),
        Duration::from_secs(1),
        Duration::from_secs(1),
    );
    let (events, _receiver) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);

    let error = transport
        .post_event_stream("/events", test_json_request(), events)
        .await
        .expect_err("delayed response headers must hit the connect deadline");

    assert_eq!(error.code, ServiceErrorCode::ServiceUnavailable);
    assert!(error.message.contains("event stream connect timeout"));
    server.abort();
}

#[tokio::test]
async fn non_streaming_responses_have_a_request_deadline() {
    let (addr, server) = delayed_http_server(
        Duration::from_millis(100),
        vec![r#"{"status":"ok"}"#],
        Duration::ZERO,
    )
    .await;
    let transport = HttpSidecarTransport::new(format!("http://{addr}"), LLM_SERVICE_NAME).with_test_timeouts(
        Duration::from_secs(1),
        Duration::from_millis(20),
        Duration::from_secs(1),
        Duration::from_secs(1),
    );

    let result: CapabilityResult<JsonResponse> = transport.post("/json", test_json_request()).await;
    let error = result.expect_err("a delayed JSON response must hit the request deadline");

    assert_eq!(error.code, ServiceErrorCode::ServiceUnavailable);
    assert!(error.message.contains("sidecar request timeout"));
    server.abort();
}

#[tokio::test]
async fn event_stream_chunks_have_an_idle_deadline() {
    let (addr, server) = delayed_http_server(
        Duration::ZERO,
        vec![
            r#"{"task_id":"task","event_kind":"chunk","sequence":1,"payload":{"text":"x"}}
"#,
            r#"{"task_id":"task","event_kind":"completed","sequence":2}
"#,
        ],
        Duration::from_millis(100),
    )
    .await;
    let transport = HttpSidecarTransport::new(format!("http://{addr}"), LLM_SERVICE_NAME).with_test_timeouts(
        Duration::from_secs(1),
        Duration::from_secs(1),
        Duration::from_millis(20),
        Duration::from_secs(1),
    );
    let (events, mut receiver) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);

    let error = transport
        .post_event_stream("/events", test_json_request(), events)
        .await
        .expect_err("a silent event stream must hit the idle deadline");

    assert_eq!(error.code, ServiceErrorCode::ServiceUnavailable);
    assert!(error.message.contains("event stream idle timeout"));
    assert_eq!(
        receiver
            .try_recv()
            .expect("the first event should arrive before the idle gap")
            .event_kind,
        a4n_service_protocol::CapabilityTaskEventKind::Chunk
    );
    server.abort();
}

#[test]
fn health_validation_rejects_protocol_mismatch() {
    let prompt_capability_version = capability_version_for(PROMPT_SERVICE_NAME);
    let err = validate_service_health(
        &health(
            PROMPT_SERVICE_NAME,
            PROTOCOL_VERSION + 1,
            prompt_capability_version,
        ),
        PROMPT_SERVICE_NAME,
        prompt_capability_version,
    )
    .unwrap_err();

    assert_eq!(err.code, ServiceErrorCode::VersionMismatch);
    assert!(err.message.contains("protocol version mismatch"));
    assert_eq!(err.retryable, Some(false));
}

#[test]
fn health_validation_rejects_expected_capability_mismatch() {
    let prompt_capability_version = capability_version_for(PROMPT_SERVICE_NAME);
    let err = validate_service_health(
        &health(PROMPT_SERVICE_NAME, PROTOCOL_VERSION, "prompt-render-v0"),
        PROMPT_SERVICE_NAME,
        prompt_capability_version,
    )
    .unwrap_err();

    assert_eq!(err.code, ServiceErrorCode::VersionMismatch);
    assert!(err.message.contains("capability version mismatch"));
    assert_eq!(err.retryable, Some(false));
}

#[test]
fn health_validation_rejects_expected_service_identity_mismatch() {
    let prompt_capability_version = capability_version_for(PROMPT_SERVICE_NAME);
    let err = validate_service_health(
        &health(
            LLM_SERVICE_NAME,
            PROTOCOL_VERSION,
            prompt_capability_version,
        ),
        PROMPT_SERVICE_NAME,
        prompt_capability_version,
    )
    .unwrap_err();

    assert_eq!(err.code, ServiceErrorCode::VersionMismatch);
    assert!(err.message.contains("service identity mismatch"));
    assert_eq!(err.retryable, Some(false));
}

#[test]
fn health_validation_rejects_unhealthy_status() {
    let prompt_capability_version = capability_version_for(PROMPT_SERVICE_NAME);
    let err = validate_service_health(
        &a4n_service_protocol::ServiceHealth {
            service: PROMPT_SERVICE_NAME.to_string(),
            status: HealthStatus::Unavailable,
            version: a4n_service_protocol::ServiceVersion {
                service_version: "test".to_string(),
                protocol_version: PROTOCOL_VERSION,
                capability_version: prompt_capability_version.to_string(),
            },
        },
        PROMPT_SERVICE_NAME,
        prompt_capability_version,
    )
    .unwrap_err();

    assert_eq!(err.code, ServiceErrorCode::ServiceUnavailable);
    assert!(err.message.contains("unhealthy"));
    assert_eq!(err.retryable, Some(true));
}

#[test]
fn import_export_typed_clients_are_trait_object_safe() {
    let import: Arc<dyn ImportClient> = Arc::new(HttpImportClient::new("http://127.0.0.1:1"));
    let export: Arc<dyn ExportClient> = Arc::new(HttpExportClient::new("http://127.0.0.1:1"));

    assert_import_client_object(import);
    assert_export_client_object(export);
}

#[test]
fn request_serialization_rejects_invalid_context_before_transport() {
    let mut request = typed_test_request(
        capability_version_for(IMPORT_SERVICE_NAME),
        serde_json::json!({}),
    );
    request.runtime_config_snapshot = Some(serde_json::json!({
        "api_key": "sk-unredacted"
    }));

    let error = serialize_request(&request)
        .expect_err("request serialization must reject unredacted context");
    assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
    assert!(error.message.contains("unredacted sensitive fields"));
    assert_eq!(error.retryable, Some(false));
    assert!(!error.message.contains("sk-unredacted"));
}

#[test]
fn embedding_model_operation_paths_are_explicit() {
    assert_eq!(
        embedding_model_operation_path(EmbeddingModelOperation::List),
        "/embedding/models"
    );
    assert_eq!(
        embedding_model_operation_path(EmbeddingModelOperation::DownloadInfo),
        "/embedding/model/download_info"
    );
    assert_eq!(
        embedding_model_operation_path(EmbeddingModelOperation::Delete),
        "/embedding/model/delete"
    );
    assert_eq!(
        EMBEDDING_MODEL_DOWNLOAD_EVENTS_PATH,
        "/embedding/model/download/events"
    );
}

#[tokio::test]
async fn drains_ndjson_events_across_chunks() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    let mut buffer =
        br#"{"task_id":"task","event_kind":"chunk","sequence":1,"payload":{"text":"he"}}"#.to_vec();

    let terminal = drain_event_lines(&mut buffer, &tx)
        .await
        .expect("partial line should stay buffered");
    assert!(!terminal);
    assert!(rx.try_recv().is_err());

    buffer.extend_from_slice(
        br#"
{"task_id":"task","event_kind":"completed","sequence":2}
"#,
    );
    let terminal = drain_event_lines(&mut buffer, &tx)
        .await
        .expect("complete lines should drain");
    assert!(terminal);

    let first = rx.try_recv().expect("chunk event");
    assert_eq!(
        first.event_kind,
        a4n_service_protocol::CapabilityTaskEventKind::Chunk
    );
    assert_eq!(
        first.payload.as_ref().and_then(|value| value["text"].as_str()),
        Some("he")
    );

    let second = rx.try_recv().expect("completed event");
    assert_eq!(
        second.event_kind,
        a4n_service_protocol::CapabilityTaskEventKind::Completed
    );
}

#[tokio::test]
async fn drain_event_lines_stops_after_terminal_event() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    let mut buffer = br#"{"task_id":"task","event_kind":"completed","sequence":1}
{"task_id":"task","event_kind":"chunk","sequence":2,"payload":{"text":"late"}}
"#
    .to_vec();

    let terminal = drain_event_lines(&mut buffer, &tx)
        .await
        .expect("terminal line should drain");
    assert!(terminal);
    assert!(buffer.is_empty());

    let event = rx.try_recv().expect("completed event");
    assert_eq!(
        event.event_kind,
        a4n_service_protocol::CapabilityTaskEventKind::Completed
    );
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn drain_event_lines_waits_for_queue_capacity() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let mut buffer = br#"{"task_id":"task","event_kind":"chunk","sequence":1,"payload":{"text":"a"}}
{"task_id":"task","event_kind":"completed","sequence":2}
"#
    .to_vec();

    let drain = tokio::spawn(async move { drain_event_lines(&mut buffer, &tx).await });
    tokio::task::yield_now().await;
    assert!(!drain.is_finished());

    let first = rx.recv().await.expect("first queued event");
    assert_eq!(first.event_kind, CapabilityTaskEventKind::Chunk);
    assert!(
        drain
            .await
            .expect("drain task")
            .expect("backpressure drain")
    );
    let terminal = rx
        .recv()
        .await
        .expect("terminal event after capacity frees");
    assert_eq!(terminal.event_kind, CapabilityTaskEventKind::Completed);
}

#[test]
fn parse_event_line_redacts_error_message() {
    let event = parse_event_line(
        br#"{"task_id":"task","event_kind":"failed","sequence":1,"error":{"code":"provider_error","message":"provider failed: base_url=https://provider.example api_key=sk-test","retryable":false}}"#,
    )
    .expect("event should parse");

    let message = event.error.expect("error").message;
    assert!(message.contains("base_url=[REDACTED]"));
    assert!(message.contains("api_key=[REDACTED]"));
    assert!(!message.contains("provider.example"));
    assert!(!message.contains("sk-test"));
}

async fn delayed_http_server(
    header_delay: Duration,
    chunks: Vec<&'static str>,
    inter_chunk_delay: Duration,
) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test HTTP server should bind");
    let addr = listener
        .local_addr()
        .expect("test HTTP server should expose its address");
    let server = tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            return;
        };
        let mut request = [0_u8; 4096];
        let _ = socket.readable().await;
        let _ = socket.try_read(&mut request);
        tokio::time::sleep(header_delay).await;
        let body_len = chunks.iter().map(|chunk| chunk.len()).sum::<usize>();
        let headers = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {SIDECAR_EVENT_CONTENT_TYPE}\r\nContent-Length: {body_len}\r\nConnection: close\r\n\r\n"
        );
        if socket.write_all(headers.as_bytes()).await.is_err() {
            return;
        }
        for (index, chunk) in chunks.iter().enumerate() {
            if index > 0 {
                tokio::time::sleep(inter_chunk_delay).await;
            }
            if socket.write_all(chunk.as_bytes()).await.is_err() {
                return;
            }
            let _ = socket.flush().await;
        }
    });
    (addr, server)
}

fn test_json_request() -> JsonRequest {
    a4n_service_protocol::ServiceRequestContext::deny_all(
        "transport-timeout-test",
        a4n_service_protocol::TraceContext {
            trace_id: "transport-timeout-trace".to_string(),
            parent_span_id: None,
        },
    )
    .with_payload("transport-test-v1", serde_json::json!({}))
    .expect("transport timeout request should be valid")
}

fn typed_test_request<T>(capability_version: &str, payload: T) -> ServiceRequest<T> {
    let mut context = a4n_service_protocol::ServiceRequestContext::deny_all(
        "request-1",
        a4n_service_protocol::TraceContext {
            trace_id: "trace-1".to_string(),
            parent_span_id: Some("parent-1".to_string()),
        },
    );
    context.user_id = Some(100);
    context.project_id = Some(42);
    context.root_project_id = Some(7);
    context.runtime_config_snapshot = Some(serde_json::json!({ "profile": "sidecar" }));
    context
        .with_payload(capability_version, payload)
        .expect("typed test request context should be valid")
}

fn assert_import_client_object(_: Arc<dyn ImportClient>) {}

fn assert_export_client_object(_: Arc<dyn ExportClient>) {}

fn capability_version_for(service_name: &'static str) -> &'static str {
    expected_capability_version_for_service(service_name)
        .expect("test service must have protocol capability version")
}

fn health(
    service: &str,
    protocol_version: u32,
    capability_version: &str,
) -> a4n_service_protocol::ServiceHealth {
    a4n_service_protocol::ServiceHealth {
        service: service.to_string(),
        status: a4n_service_protocol::HealthStatus::Healthy,
        version: a4n_service_protocol::ServiceVersion {
            service_version: "test".to_string(),
            protocol_version,
            capability_version: capability_version.to_string(),
        },
    }
}
