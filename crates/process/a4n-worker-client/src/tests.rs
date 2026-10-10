use std::{collections::HashMap, time::Duration};

use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerCancelRequest, WorkerCapabilities, WorkerEnvelope, WorkerError,
    WorkerErrorCode, WorkerHealth, WorkerHealthStatus, WorkerLease, WorkerOperationRef,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerRecoveryRequest,
    WorkerResponse,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

use super::{
    HttpWorkerClient, WORKER_AUTH_HEADER, WorkerClientConfig, WorkerClientError,
    WorkerClientErrorPolicy, WorkerJsonResponse, map_worker_client_error,
    map_worker_response_error, optional_positive_id, positive_id, resolve_worker_client_connection,
    user_principal, user_principal_with_scope,
};

#[test]
fn worker_client_error_mapping_preserves_transport_policy() {
    assert!(matches!(
        map_worker_client_error(
            WorkerClientError::Timeout,
            "worker",
            WorkerClientErrorPolicy::Configuration,
        ),
        a4n_error::AppError::ConfigError(message) if message == "worker 请求失败: worker request timed out"
    ));
    assert!(matches!(
        map_worker_client_error(
            WorkerClientError::Timeout,
            "worker",
            WorkerClientErrorPolicy::TransientAsTimeout,
        ),
        a4n_error::AppError::Timeout(message) if message == "worker 请求超时"
    ));
    assert!(matches!(
        map_worker_client_error(
            WorkerClientError::HttpStatus { status: 503 },
            "worker",
            WorkerClientErrorPolicy::TransientAsTimeout,
        ),
        a4n_error::AppError::Timeout(message) if message == "worker 返回 HTTP 503"
    ));
    assert!(matches!(
        map_worker_client_error(
            WorkerClientError::HttpStatus { status: 409 },
            "worker",
            WorkerClientErrorPolicy::TransientAsTimeout,
        ),
        a4n_error::AppError::ConfigError(message) if message == "worker 请求失败: worker returned HTTP status 409"
    ));
}

#[test]
fn worker_response_error_mapping_preserves_basic_contract() {
    assert!(matches!(
        map_worker_response_error(
            &WorkerError {
                code: WorkerErrorCode::Conflict,
                message: "revision conflict".into(),
                retryable: false,
            },
            "demo worker",
            "demo.save",
        ),
        a4n_error::AppError::InvalidState { current, operation }
            if current == "revision conflict" && operation == "demo.save"
    ));
    assert!(matches!(
        map_worker_response_error(
            &WorkerError {
                code: WorkerErrorCode::Internal,
                message: "internal failure".into(),
                retryable: false,
            },
            "demo worker",
            "demo.save",
        ),
        a4n_error::AppError::ConfigError(message)
            if message == "demo worker demo.save 失败: internal failure"
    ));
}

#[test]
fn user_principal_requires_a_positive_id() {
    let principal = user_principal(7).expect("positive user ID should build a principal");
    assert_eq!(principal.kind, WorkerPrincipalKind::User);
    assert_eq!(principal.subject, "7");

    for user_id in [0, -1] {
        assert!(matches!(
            user_principal(user_id),
            Err(a4n_error::AppError::InvalidArgument(message))
                if message == "user_id 必须大于 0"
        ));
    }
}

#[test]
fn user_principal_with_scope_preserves_project_validation_policy() {
    const ERROR: &str = "user_id 和 project_id 必须大于 0";
    assert_eq!(
        user_principal_with_scope(7, Some(3), ERROR)
            .expect("valid project-scoped principal")
            .subject,
        "7"
    );
    assert!(user_principal_with_scope(7, None, ERROR).is_ok());

    for (user_id, project_id) in [(0, Some(3)), (7, Some(0)), (7, Some(-1))] {
        assert!(matches!(
            user_principal_with_scope(user_id, project_id, ERROR),
            Err(a4n_error::AppError::InvalidArgument(message)) if message == ERROR
        ));
    }
}

#[test]
fn positive_id_helpers_preserve_field_errors_and_optional_none() {
    assert_eq!(positive_id("project_id", 7).expect("positive ID"), 7);
    assert_eq!(
        optional_positive_id("project_id", None).expect("optional ID"),
        None
    );
    assert_eq!(
        optional_positive_id("project_id", Some(7)).expect("positive optional ID"),
        Some(7)
    );

    for value in [0, -1] {
        assert!(matches!(
            positive_id("project_id", value),
            Err(a4n_error::AppError::InvalidArgument(message))
                if message == "project_id 必须大于 0"
        ));
        assert!(matches!(
            optional_positive_id("project_id", Some(value)),
            Err(a4n_error::AppError::InvalidArgument(message))
                if message == "project_id 必须大于 0"
        ));
    }
}

#[test]
fn worker_client_connection_keeps_default_and_optional_token_rules() {
    assert_eq!(
        resolve_worker_client_connection(
            Err(std::env::VarError::NotPresent),
            Err(std::env::VarError::NotPresent),
            "http://127.0.0.1:47650",
        ),
        ("http://127.0.0.1:47650".to_string(), None),
    );
    assert_eq!(
        resolve_worker_client_connection(
            Ok(" https://worker.example.test ".to_string()),
            Ok(" token ".to_string()),
            "http://127.0.0.1:47650",
        ),
        (
            " https://worker.example.test ".to_string(),
            Some(" token ".to_string()),
        ),
    );
    assert_eq!(
        resolve_worker_client_connection(
            Ok("   ".to_string()),
            Ok("   ".to_string()),
            "http://127.0.0.1:47650",
        ),
        ("http://127.0.0.1:47650".to_string(), None),
    );
}

#[tokio::test]
async fn health_and_capabilities_roundtrip() {
    let (health_url, health_server) =
        mock_server(json_body(&health_response()), 200, Duration::ZERO).await;
    let client = HttpWorkerClient::new(health_url, "worker-test").with_auth_token("worker-token");
    assert_eq!(client.health().await.unwrap().worker_id, "worker-test");
    let health_request = health_server.await.unwrap();
    assert_eq!(health_request.path(), "/healthz");
    assert_eq!(
        health_request.header(WORKER_AUTH_HEADER),
        Some("worker-token")
    );

    let (capabilities_url, capabilities_server) =
        mock_server(json_body(&capabilities_response()), 200, Duration::ZERO).await;
    let capabilities = HttpWorkerClient::new(capabilities_url, "worker-test")
        .with_auth_token("worker-token")
        .capabilities()
        .await
        .unwrap();
    assert_eq!(capabilities.worker_id, "worker-test");
    let capabilities_request = capabilities_server.await.unwrap();
    assert_eq!(capabilities_request.path(), "/capabilities");
}

#[tokio::test]
async fn custom_auth_header_is_used() {
    let (url, server) = mock_server(json_body(&health_response()), 200, Duration::ZERO).await;
    let client = HttpWorkerClient::new(url, "worker-test")
        .with_auth_header("x-a4n-agent-worker-token")
        .with_auth_token("worker-token");

    client.health().await.unwrap();

    let request = server.await.unwrap();
    assert_eq!(
        request.header("x-a4n-agent-worker-token"),
        Some("worker-token")
    );
    assert_eq!(request.header(WORKER_AUTH_HEADER), None);
}

#[tokio::test]
async fn command_roundtrip_sends_token_and_worker_envelope() {
    let response = completed_response("request-1", "trace-1", serde_json::json!({"ok": true}));
    let (url, server) = mock_server(json_body(&response), 200, Duration::ZERO).await;
    let client = HttpWorkerClient::new(url, "worker-test").with_auth_token("worker-token");
    let request = envelope("request-1", "trace-1");

    let response: WorkerResponse<serde_json::Value> =
        client.command("demo.run", request.clone()).await.unwrap();

    assert_eq!(response.status, WorkerOperationStatus::Completed);
    assert_eq!(response.payload, Some(serde_json::json!({"ok": true})));
    let captured = server.await.unwrap();
    assert_eq!(captured.path(), "/v1/commands/demo.run");
    assert_eq!(captured.header(WORKER_AUTH_HEADER), Some("worker-token"));
    assert_eq!(captured.json_body(), serde_json::to_value(request).unwrap());
}

#[tokio::test]
async fn cancel_and_recovery_use_transport_only_json_responses() {
    let response = completed_response("cancel-request", "cancel-trace", serde_json::Value::Null);
    let (cancel_url, cancel_server) = mock_server(json_body(&response), 200, Duration::ZERO).await;
    let client = HttpWorkerClient::new(cancel_url, "worker-test");
    let cancel = client
        .cancel(WorkerCancelRequest {
            operation: operation("operation-1"),
            reason: "user requested".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(cancel.status, WorkerOperationStatus::Completed);
    assert_eq!(
        cancel_server.await.unwrap().path(),
        "/v1/operations/operation-1/cancel"
    );

    let (recovery_url, recovery_server) =
        mock_server(json_body(&response), 200, Duration::ZERO).await;
    let recovery = HttpWorkerClient::new(recovery_url, "worker-test")
        .recover(WorkerRecoveryRequest {
            operation: operation("operation-1"),
            reason: "worker restarted".to_string(),
            checkpoint_ref: Some("checkpoint-1".to_string()),
        })
        .await
        .unwrap();
    assert_eq!(recovery.status, WorkerOperationStatus::Completed);
    assert_eq!(
        recovery_server.await.unwrap().path(),
        "/v1/operations/operation-1/recover"
    );
}

#[tokio::test]
async fn unauthorized_response_is_explicit() {
    let (url, server) = mock_server(String::new(), 401, Duration::ZERO).await;
    let client = HttpWorkerClient::new(url, "worker-test");

    assert!(matches!(
        client.health().await,
        Err(WorkerClientError::Unauthorized)
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn schema_mismatch_is_rejected_before_use() {
    let mut response = health_response();
    response.schema_version += 1;
    let (url, server) = mock_server(json_body(&response), 200, Duration::ZERO).await;
    let client = HttpWorkerClient::new(url, "worker-test");

    assert!(matches!(
        client.health().await,
        Err(WorkerClientError::InvalidResponse(
            a4n_worker_contract::WorkerContractError::SchemaVersion { .. }
        ))
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn invalid_response_is_rejected_by_worker_contract() {
    let mut response = completed_response("request-1", "trace-1", serde_json::json!({}));
    response.status = WorkerOperationStatus::Failed;
    response.payload = None;
    response.error = None;
    let (url, server) = mock_server(json_body(&response), 200, Duration::ZERO).await;
    let client = HttpWorkerClient::new(url, "worker-test");

    assert!(matches!(
        client
            .command::<_, serde_json::Value>("demo.run", envelope("request-1", "trace-1"))
            .await,
        Err(WorkerClientError::InvalidResponse(
            a4n_worker_contract::WorkerContractError::InvalidRelation(_)
        ))
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn non_success_response_is_explicit() {
    let (url, server) = mock_server(String::new(), 503, Duration::ZERO).await;
    let client = HttpWorkerClient::new(url, "worker-test");

    assert!(matches!(
        client.health().await,
        Err(WorkerClientError::HttpStatus { status: 503 })
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn delayed_response_hits_request_timeout() {
    let (url, server) = mock_server(
        json_body(&health_response()),
        200,
        Duration::from_millis(80),
    )
    .await;
    let client = HttpWorkerClient::with_config(
        url,
        "worker-test",
        WorkerClientConfig::new(Duration::from_secs(1), Duration::from_millis(10)),
    );

    assert!(matches!(
        client.health().await,
        Err(WorkerClientError::Timeout)
    ));
    server.abort();
}

#[test]
fn debug_output_redacts_connection_details_and_token() {
    let client = HttpWorkerClient::new("https://user:secret@worker.example.test", "worker-test")
        .with_auth_token("worker-token");
    let debug = format!("{client:?}");

    assert!(!debug.contains("worker.example.test"));
    assert!(!debug.contains("worker-token"));
    assert!(debug.contains("auth_token_configured: true"));
}

fn health_response() -> WorkerHealth {
    WorkerHealth {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: "worker-test".to_string(),
        status: WorkerHealthStatus::Healthy,
        version: "test-version".to_string(),
    }
}

fn capabilities_response() -> WorkerCapabilities {
    WorkerCapabilities {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: "worker-test".to_string(),
        capabilities: vec![],
    }
}

fn envelope(request_id: &str, trace_id: &str) -> WorkerEnvelope<serde_json::Value> {
    WorkerEnvelope {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request_id.to_string(),
        trace_id: trace_id.to_string(),
        principal: WorkerPrincipal {
            kind: WorkerPrincipalKind::User,
            subject: "user-1".to_string(),
        },
        operation_id: "operation-1".to_string(),
        idempotency_key: "idempotency-1".to_string(),
        project_id: Some(1),
        root_project_id: None,
        target_revision: Some(1),
        runtime_config_snapshot_ref: None,
        lease: WorkerLease {
            worker_id: "worker-test".to_string(),
            epoch: 1,
            fence_token: "fence-1".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        },
        payload: serde_json::json!({"command": "run"}),
    }
}

fn operation(operation_id: &str) -> WorkerOperationRef {
    WorkerOperationRef {
        operation_id: operation_id.to_string(),
        task_id: None,
    }
}

fn completed_response(
    request_id: &str,
    trace_id: &str,
    payload: serde_json::Value,
) -> WorkerJsonResponse {
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request_id.to_string(),
        trace_id: trace_id.to_string(),
        operation: operation("operation-1"),
        status: WorkerOperationStatus::Completed,
        payload: Some(payload),
        resource_refs: vec![],
        committed_revision: None,
        provider_job_id: None,
        result_summary: None,
        pagination: None,
        error: None,
    }
}

async fn mock_server(
    body: String,
    status: u16,
    delay: Duration,
) -> (String, JoinHandle<CapturedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_request(&mut stream).await;
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        write_response(&mut stream, status, &body).await;
        request
    });
    (format!("http://{address}"), server)
}

async fn read_request(stream: &mut TcpStream) -> CapturedRequest {
    let mut bytes = Vec::new();
    let mut buffer = [0; 1024];
    let header_end = loop {
        let count = stream.read(&mut buffer).await.unwrap();
        assert!(count > 0, "mock client closed before sending HTTP request");
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..header_end]);
    let mut lines = headers.split("\r\n");
    let request_line = lines.next().unwrap_or_default().to_string();
    let mut header_map = HashMap::new();
    let mut content_length = 0;
    for line in lines.filter(|line| !line.is_empty()) {
        if let Some((name, value)) = line.split_once(':') {
            let name = name.to_ascii_lowercase();
            let value = value.trim().to_string();
            if name == "content-length" {
                content_length = value.parse().unwrap();
            }
            header_map.insert(name, value);
        }
    }
    while bytes.len() < header_end + content_length {
        let count = stream.read(&mut buffer).await.unwrap();
        assert!(count > 0, "mock client closed before sending HTTP body");
        bytes.extend_from_slice(&buffer[..count]);
    }
    CapturedRequest {
        request_line,
        headers: header_map,
        body: bytes[header_end..header_end + content_length].to_vec(),
    }
}

async fn write_response(stream: &mut TcpStream, status: u16, body: &str) {
    let reason = match status {
        200 => "OK",
        401 => "Unauthorized",
        503 => "Service Unavailable",
        _ => "Test Response",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await.unwrap();
}

fn json_body<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap()
}

struct CapturedRequest {
    request_line: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl CapturedRequest {
    fn path(&self) -> &str {
        self.request_line
            .split_whitespace()
            .nth(1)
            .expect("mock request must contain a path")
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    fn json_body(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).unwrap()
    }
}
