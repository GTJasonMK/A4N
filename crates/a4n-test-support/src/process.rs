use std::{
    collections::HashMap,
    convert::Infallible,
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use a4n_service_manager::{
    ManagedServiceConfig, ManagedSidecarProcess, ServiceManager, ServiceManagerError,
};
use a4n_service_protocol::{HealthStatus, SIDECAR_AUTH_TOKEN_ENV, ServiceRequest};
use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::State,
    http::{StatusCode, header},
    routing::any,
};
use futures::{future::pending, stream};
use serde_json::Value;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tokio::{io::AsyncWriteExt, net::TcpStream};

pub const SIDECAR_PROCESS_TEST_TOKEN: &str = "a4n-process-test-token";

pub use a4n_embedding_client::{EmbeddingClient, HttpEmbeddingClient};
pub use a4n_export_client::{ExportClient, HttpExportClient};
pub use a4n_export_contract::{
    EXPORT_RENDER_CAPABILITY_VERSION, EXPORT_SERVICE_NAME, ExportChapter, ExportDocument,
    ExportDocumentFormat, ExportRenderDocumentInput, ExportRenderJsonInput, ExportRenderZipInput,
    ExportZipEntryBytes, ExportZipPackageInput,
};
pub use a4n_image_client::{HttpImageClient, ImageClient};
pub use a4n_import_client::{HttpImportClient, ImportClient};
pub use a4n_import_contract::{
    IMPORT_SERVICE_NAME, IMPORT_TRANSFORM_CAPABILITY_VERSION, ImportReadMediaPackageZipInput,
};
pub use a4n_llm_client::{HttpLlmClient, LlmClient};
pub use a4n_prompt_client::{HttpPromptClient, PromptClient};
pub type JsonRequest = ServiceRequest<Value>;

pub use a4n_rag_client::{HttpRagClient, RagClient};
pub use a4n_rag_sidecar_contract::{
    CompletenessByIdsPayload, DeleteFilterPayload, DeleteRecordsPayload, DropCollectionPayload,
    EmbeddingRecordPayload, IngestPayload, MetadataByFilterPayload, MetadataByIdsPayload,
    QueryPayload, SearchPayload, SearchTextPayload,
};
pub use a4n_service_client::{
    ServiceClient as CapabilityClient, ServiceClientResult as CapabilityResult,
};
pub use a4n_service_protocol::{
    EMBEDDING_CAPABILITY_VERSION, EMBEDDING_SERVICE_NAME, IMAGE_PROVIDER_CAPABILITY_VERSION,
    IMAGE_SERVICE_NAME, PROMPT_RENDER_CAPABILITY_VERSION, PROMPT_SERVICE_NAME,
    RAG_INDEX_CAPABILITY_VERSION, RAG_SERVICE_NAME, ServiceRequestContext, ServiceStatus,
    TraceContext, VIDEO_PROVIDER_CAPABILITY_VERSION, VIDEO_SERVICE_NAME,
};
pub use a4n_video_client::{HttpVideoClient, VideoClient, VideoPollRequest, VideoProviderRequest};
pub use a4n_video_sidecar_contract::{
    VideoDeleteOutput, VideoGenerationRequest, VideoJobOutput, VideoMediaInput, VideoPollPayload,
    VideoProviderPayload, VideoStatusOutput,
};

pub fn sidecar_base_url(addr: SocketAddr) -> String {
    format!("http://{addr}")
}

pub async fn assert_protected_route_rejects_invalid_auth(base_url: &str, path: &str) {
    const WRONG_TOKEN: &str = "a4n-wrong-process-test-token";

    let client = reqwest::Client::new();
    for token in [None, Some(WRONG_TOKEN)] {
        let mut request = client
            .post(format!("{base_url}{path}"))
            .json(&serde_json::json!({}));
        if let Some(token) = token {
            request = request.header(a4n_service_protocol::SIDECAR_AUTH_HEADER, token);
        }
        let response = request
            .send()
            .await
            .expect("independent sidecar should answer invalid authentication");
        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
        let body = response
            .text()
            .await
            .expect("authentication error body should be readable");
        assert!(!body.contains(WRONG_TOKEN));
        assert!(!body.contains(SIDECAR_PROCESS_TEST_TOKEN));
    }
}

pub async fn assert_route_rejects_version_mismatch(
    base_url: &str,
    path: &str,
    request: &impl serde::Serialize,
) {
    let client = reqwest::Client::new();
    let original = serde_json::to_value(request).expect("typed request should serialize");
    let mut cases = Vec::with_capacity(2);

    let mut capability = original.clone();
    capability["capability_version"] = serde_json::json!("process-incompatible-v0");
    cases.push(capability);

    let mut protocol = original;
    protocol["schema_version"]["protocol"] =
        serde_json::json!(a4n_service_protocol::PROTOCOL_VERSION + 1);
    cases.push(protocol);

    for body in cases {
        let response = client
            .post(format!("{base_url}{path}"))
            .header(
                a4n_service_protocol::SIDECAR_AUTH_HEADER,
                SIDECAR_PROCESS_TEST_TOKEN,
            )
            .json(&body)
            .send()
            .await
            .expect("independent sidecar should answer version mismatch");
        let response: a4n_service_protocol::ServiceResponse<serde_json::Value> = response
            .json()
            .await
            .expect("version mismatch should use the service response contract");
        assert_eq!(response.status, a4n_service_protocol::ServiceStatus::Error);
        assert_eq!(
            response.error.expect("version mismatch error").code,
            a4n_service_protocol::ServiceErrorCode::VersionMismatch
        );
    }
}

pub async fn assert_route_rejects_malformed_json(base_url: &str, path: &str) {
    let response = reqwest::Client::new()
        .post(format!("{base_url}{path}"))
        .header(
            a4n_service_protocol::SIDECAR_AUTH_HEADER,
            SIDECAR_PROCESS_TEST_TOKEN,
        )
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body("{")
        .send()
        .await
        .expect("independent sidecar should reject malformed JSON");
    assert!(response.status().is_client_error());
    let body = response
        .text()
        .await
        .expect("malformed JSON error body should be readable");
    assert!(!body.contains(SIDECAR_PROCESS_TEST_TOKEN));
}

pub async fn assert_route_rejects_unknown_payload_field(
    base_url: &str,
    path: &str,
    request: &impl serde::Serialize,
) {
    let mut body = serde_json::to_value(request).expect("typed request should serialize");
    body["payload"]["process_unknown_field"] = serde_json::json!(true);
    assert_route_rejects_unknown_json_field(base_url, path, body, "payload").await;
}

pub async fn assert_route_rejects_unknown_envelope_field(
    base_url: &str,
    path: &str,
    request: &impl serde::Serialize,
) {
    let mut body = serde_json::to_value(request).expect("typed request should serialize");
    body["process_unknown_field"] = serde_json::json!(true);
    assert_route_rejects_unknown_json_field(base_url, path, body, "envelope").await;
}

async fn assert_route_rejects_unknown_json_field(
    base_url: &str,
    path: &str,
    body: serde_json::Value,
    location: &str,
) {
    let response = reqwest::Client::new()
        .post(format!("{base_url}{path}"))
        .header(
            a4n_service_protocol::SIDECAR_AUTH_HEADER,
            SIDECAR_PROCESS_TEST_TOKEN,
        )
        .json(&body)
        .send()
        .await
        .unwrap_or_else(|error| panic!("sidecar did not answer unknown {location} field: {error}"));
    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|error| panic!("unknown {location} field body is unreadable: {error}"));
    assert!(
        status.is_client_error(),
        "unknown {location} field should be rejected: status={status}, body={body}"
    );
    assert!(!body.contains(SIDECAR_PROCESS_TEST_TOKEN));
}

pub async fn disconnect_during_request_body(addr: SocketAddr, path: &str) {
    use tokio::io::AsyncWriteExt;

    let mut stream = tokio::net::TcpStream::connect(addr)
        .await
        .expect("sidecar should accept a connection for the disconnect test");
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: 1048576\r\n{}: {}\r\nConnection: close\r\n\r\n{{",
        a4n_service_protocol::SIDECAR_AUTH_HEADER,
        SIDECAR_PROCESS_TEST_TOKEN
    );
    stream
        .write_all(request.as_bytes())
        .await
        .expect("partial request should be written");
    stream
        .shutdown()
        .await
        .expect("disconnect test stream should close cleanly");
    tokio::time::sleep(Duration::from_millis(25)).await;
}

pub async fn assert_route_rejects_oversized_json(base_url: &str, path: &str, limit: usize) {
    let response = reqwest::Client::new()
        .post(format!("{base_url}{path}"))
        .header(
            a4n_service_protocol::SIDECAR_AUTH_HEADER,
            SIDECAR_PROCESS_TEST_TOKEN,
        )
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(vec![b' '; limit + 1])
        .send()
        .await
        .expect("independent sidecar should reject an oversized JSON body");
    assert_eq!(response.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);
    let body = response
        .text()
        .await
        .expect("oversized JSON error body should be readable");
    assert!(!body.contains(SIDECAR_PROCESS_TEST_TOKEN));
}

pub async fn hold_partial_requests(addr: SocketAddr, path: &str, count: usize) -> Vec<TcpStream> {
    let mut streams = Vec::with_capacity(count);
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: 1048576\r\n{}: {}\r\nConnection: keep-alive\r\n\r\n{{",
        a4n_service_protocol::SIDECAR_AUTH_HEADER,
        SIDECAR_PROCESS_TEST_TOKEN
    );
    for _ in 0..count {
        let mut stream = TcpStream::connect(addr)
            .await
            .expect("sidecar should accept a partial request body");
        stream
            .write_all(request.as_bytes())
            .await
            .expect("partial request headers should be written");
        streams.push(stream);
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    streams
}

/// Prove that the shared sidecar semaphore spans business routes and health.
pub async fn assert_process_concurrency_saturation(addr: SocketAddr, service: &str, path: &str) {
    let policy = a4n_service_protocol::sidecar_resource_policy(service)
        .expect("process test service should have a resource policy");
    let held = hold_partial_requests(addr, path, policy.max_concurrent_requests).await;
    let client = reqwest::Client::new();
    let health_url = format!("http://{addr}{}", a4n_service_protocol::SIDECAR_HEALTH_PATH);

    assert!(
        timeout(Duration::from_millis(250), client.get(&health_url).send())
            .await
            .is_err(),
        "{service} health should wait while all sidecar concurrency permits are held"
    );

    drop(held);
    let response = timeout(Duration::from_secs(2), client.get(health_url).send())
        .await
        .expect("sidecar health should recover after partial requests close")
        .expect("sidecar health request should succeed after backpressure clears");
    assert!(
        response.status().is_success(),
        "{service} health should return success after concurrency recovery"
    );
}

pub async fn assert_sidecar_rejects_non_loopback_without_auth(
    service_name: &str,
    binary: impl Into<PathBuf>,
    addr_env: &str,
    extra_env: impl IntoIterator<Item = (String, String)>,
) {
    let loopback_addr = unused_local_addr();
    let bind_addr = SocketAddr::from(([0, 0, 0, 0], loopback_addr.port()));
    let mut config =
        ManagedServiceConfig::sidecar(service_name, binary, format!("http://{loopback_addr}"));
    config.env = HashMap::from([(addr_env.to_string(), bind_addr.to_string())]);
    config.env.extend(extra_env);
    config.startup_timeout_ms = 3_000;
    let manager = ServiceManager::new(vec![config])
        .expect("non-loopback process test config should be valid");
    let error = manager
        .start_sidecar(service_name)
        .await
        .expect_err("sidecar should reject non-loopback bind without authentication");
    assert!(matches!(error, ServiceManagerError::Exited { .. }));
    assert_port_released(loopback_addr);
}

pub fn unused_local_addr() -> SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("temporary listener should reserve a loopback port");
    listener
        .local_addr()
        .expect("temporary listener should expose its local address")
}

pub struct SidecarProcessTestDir {
    path: PathBuf,
}

impl SidecarProcessTestDir {
    pub fn new(service: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "a4n-{service}-process-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("sidecar process test directory should be created");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SidecarProcessTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn sidecar_manager(
    service_name: &str,
    binary: impl Into<PathBuf>,
    addr: SocketAddr,
    addr_env: &str,
    extra_env: impl IntoIterator<Item = (String, String)>,
) -> (ServiceManager, SidecarProcessTestDir) {
    let temp_dir = SidecarProcessTestDir::new(service_name);
    let manager =
        sidecar_manager_with_dir(service_name, binary, addr, addr_env, &temp_dir, extra_env);
    (manager, temp_dir)
}

pub fn sidecar_manager_with_dir(
    service_name: &str,
    binary: impl Into<PathBuf>,
    addr: SocketAddr,
    addr_env: &str,
    _temp_dir: &SidecarProcessTestDir,
    extra_env: impl IntoIterator<Item = (String, String)>,
) -> ServiceManager {
    let mut env = HashMap::from([
        (addr_env.to_string(), addr.to_string()),
        (
            SIDECAR_AUTH_TOKEN_ENV.to_string(),
            SIDECAR_PROCESS_TEST_TOKEN.to_string(),
        ),
    ]);
    env.extend(extra_env);
    let mut config = ManagedServiceConfig::sidecar(service_name, binary, format!("http://{addr}"));
    config.env = env;
    config.startup_timeout_ms = 10_000;
    ServiceManager::new(vec![config]).expect("sidecar process test config should be valid")
}

pub async fn start_sidecar(manager: &ServiceManager, service_name: &str) -> ManagedSidecarProcess {
    let process = manager
        .start_sidecar(service_name)
        .await
        .expect("manager should start the independent sidecar binary");
    assert!(
        manager
            .process_status(service_name)
            .await
            .unwrap()
            .and_then(|status| status.pid)
            .is_some_and(|pid| pid > 0)
    );
    let health = manager
        .health(service_name)
        .await
        .expect("manager should validate sidecar health");
    assert_eq!(health.service, service_name);
    assert_eq!(health.status, HealthStatus::Healthy);
    process
}

pub async fn finish_sidecar(
    process: ManagedSidecarProcess,
    manager: &ServiceManager,
    service_name: &str,
    addr: SocketAddr,
) {
    process
        .stop()
        .await
        .expect("manager-owned sidecar should stop explicitly");
    assert_sidecar_stopped(manager, service_name, addr).await;
}

pub async fn force_finish_sidecar(
    process: ManagedSidecarProcess,
    manager: &ServiceManager,
    service_name: &str,
    addr: SocketAddr,
) {
    process
        .force_stop()
        .await
        .expect("manager-owned sidecar should be force-stopped explicitly");
    assert_sidecar_stopped(manager, service_name, addr).await;
}

async fn assert_sidecar_stopped(manager: &ServiceManager, service_name: &str, addr: SocketAddr) {
    let error = wait_until_unavailable(manager, service_name).await;
    assert!(matches!(
        error,
        ServiceManagerError::HealthCheckFailed { .. }
    ));
    assert_port_released(addr);
}

pub async fn wait_until_unavailable(
    manager: &ServiceManager,
    service_name: &str,
) -> ServiceManagerError {
    timeout(Duration::from_secs(5), async {
        loop {
            match manager.health(service_name).await {
                Ok(_) => tokio::task::yield_now().await,
                Err(error) => return error,
            }
        }
    })
    .await
    .expect("stopped sidecar should become unavailable before timeout")
}

fn assert_port_released(addr: SocketAddr) {
    let listener = std::net::TcpListener::bind(addr)
        .expect("stopped sidecar should release its loopback port");
    drop(listener);
}

pub struct FakeJsonProvider {
    addr: SocketAddr,
    shutdown: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<Result<(), std::io::Error>>,
}

#[derive(Clone)]
struct HangingJsonProviderState {
    request_started_signal: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    body_drop_signal: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

struct ProviderBodyDropSignal(Option<oneshot::Sender<()>>);

impl Drop for ProviderBodyDropSignal {
    fn drop(&mut self) {
        if let Some(signal) = self.0.take() {
            let _ = signal.send(());
        }
    }
}

pub struct HangingJsonProvider {
    addr: SocketAddr,
    shutdown: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<Result<(), std::io::Error>>,
}

impl HangingJsonProvider {
    pub async fn start() -> (Self, oneshot::Receiver<()>, oneshot::Receiver<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("hanging JSON provider should bind a loopback port");
        let addr = listener
            .local_addr()
            .expect("hanging JSON provider should expose its local address");
        let (request_started_signal, request_started_receiver) = oneshot::channel();
        let (body_drop_signal, body_drop_receiver) = oneshot::channel();
        let router = Router::new()
            .fallback(any(hanging_json_response))
            .with_state(HangingJsonProviderState {
                request_started_signal: Arc::new(Mutex::new(Some(request_started_signal))),
                body_drop_signal: Arc::new(Mutex::new(Some(body_drop_signal))),
            });
        let (shutdown, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
        });
        (
            Self {
                addr,
                shutdown,
                task,
            },
            request_started_receiver,
            body_drop_receiver,
        )
    }

    pub fn base_url(&self) -> String {
        sidecar_base_url(self.addr)
    }

    pub async fn stop(self) {
        let _ = self.shutdown.send(());
        timeout(Duration::from_secs(5), self.task)
            .await
            .expect("hanging JSON provider should stop before timeout")
            .expect("hanging JSON provider task should join")
            .expect("hanging JSON provider should stop cleanly");
    }
}

async fn hanging_json_response(
    State(state): State<HangingJsonProviderState>,
) -> impl axum::response::IntoResponse {
    let request_started_signal = state
        .request_started_signal
        .lock()
        .expect("hanging JSON provider request signal lock should not be poisoned")
        .take()
        .expect("hanging JSON provider should be called once");
    let signal = state
        .body_drop_signal
        .lock()
        .expect("hanging JSON provider signal lock should not be poisoned")
        .take()
        .expect("hanging JSON provider should be called once");
    let body = stream::unfold(
        (false, ProviderBodyDropSignal(Some(signal))),
        |(first_sent, drop_signal)| async move {
            if first_sent {
                return pending::<
                    Option<(Result<Bytes, Infallible>, (bool, ProviderBodyDropSignal))>,
                >()
                .await;
            }
            Some((Ok(Bytes::from_static(b"{")), (true, drop_signal)))
        },
    );
    let _ = request_started_signal.send(());
    (
        [(header::CONTENT_TYPE, "application/json")],
        Body::from_stream(body),
    )
}

impl FakeJsonProvider {
    pub async fn start(response: Value) -> Self {
        Self::start_with_status(StatusCode::OK, response).await
    }

    pub async fn start_delayed(response: Value, delay: Duration) -> Self {
        Self::start_with_status_delayed(StatusCode::OK, response, delay).await
    }

    pub async fn start_with_status(status: StatusCode, response: Value) -> Self {
        Self::start_with_status_delayed(status, response, Duration::ZERO).await
    }

    async fn start_with_status_delayed(
        status: StatusCode,
        response: Value,
        delay: Duration,
    ) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fake provider should bind a loopback port");
        let addr = listener
            .local_addr()
            .expect("fake provider should expose its local address");
        let response = Arc::new(response);
        let router = Router::new().fallback(any(move || {
            let response = response.clone();
            async move {
                tokio::time::sleep(delay).await;
                (status, Json((*response).clone()))
            }
        }));
        let (shutdown, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
        });
        Self {
            addr,
            shutdown,
            task,
        }
    }

    pub fn base_url(&self) -> String {
        sidecar_base_url(self.addr)
    }

    pub async fn stop(self) {
        let _ = self.shutdown.send(());
        timeout(Duration::from_secs(5), self.task)
            .await
            .expect("fake provider should stop before timeout")
            .expect("fake provider task should join")
            .expect("fake provider should stop cleanly");
    }
}

pub fn request_context<T>(
    request_id: &str,
    trace_id: &str,
    capability_version: &str,
    payload: T,
) -> ServiceRequest<T> {
    ServiceRequestContext::deny_all(
        request_id,
        TraceContext {
            trace_id: trace_id.to_string(),
            parent_span_id: None,
        },
    )
    .with_payload(capability_version, payload)
    .expect("sidecar process request context should be valid")
}

/// Build a secret-free request explicitly authorized for a trusted remote sidecar.
/// Keep this in the shared process support so remote egress tests do not each
/// invent their own envelope mutation.
pub fn trusted_remote_request_context<T>(
    request_id: &str,
    trace_id: &str,
    capability_version: &str,
    payload: T,
) -> ServiceRequest<T> {
    ServiceRequestContext::new(
        request_id,
        TraceContext {
            trace_id: trace_id.to_string(),
            parent_span_id: None,
        },
        a4n_service_protocol::SecretForwardingPolicy::trusted_remote_no_secrets(),
    )
    .with_payload(capability_version, payload)
    .expect("trusted-remote process request context should be valid")
}

pub fn provider_request_context<T>(
    request_id: &str,
    trace_id: &str,
    capability_version: &str,
    payload: T,
) -> ServiceRequest<T> {
    ServiceRequestContext::local_user_secret(
        request_id,
        TraceContext {
            trace_id: trace_id.to_string(),
            parent_span_id: None,
        },
    )
    .with_payload(capability_version, payload)
    .expect("provider sidecar process request context should be valid")
}
