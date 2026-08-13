#[cfg(feature = "fastembed")]
use a4n_service_protocol::{CapabilityTaskEventKind, ServiceErrorCode};
#[cfg(feature = "fastembed")]
use a4n_test_support::process::force_finish_sidecar;
use a4n_test_support::process::{
    EMBEDDING_CAPABILITY_VERSION, EMBEDDING_SERVICE_NAME, EmbeddingClient, EmbeddingModelOperation,
    HttpEmbeddingClient, SIDECAR_PROCESS_TEST_TOKEN, ServiceStatus, SidecarProcessTestDir,
    assert_process_concurrency_saturation, assert_protected_route_rejects_invalid_auth,
    assert_route_rejects_malformed_json, assert_route_rejects_oversized_json,
    assert_route_rejects_unknown_payload_field, assert_route_rejects_version_mismatch,
    assert_sidecar_rejects_non_loopback_without_auth, disconnect_during_request_body,
    finish_sidecar, request_context, sidecar_base_url, sidecar_manager_with_dir, start_sidecar,
    trusted_remote_request_context, unused_local_addr,
};
#[cfg(feature = "fastembed")]
use tokio::{io::AsyncReadExt, sync::oneshot, task::JoinHandle};

#[allow(unused_mut)]
fn embedding_process_env(cache_dir: &std::path::Path) -> Vec<(String, String)> {
    let mut env = vec![(
        "A4N_EMBEDDING_CACHE_DIR".to_string(),
        cache_dir.to_string_lossy().into_owned(),
    )];
    #[cfg(feature = "fastembed")]
    for key in [
        "A4N_ONNXRUNTIME_ROOT",
        "ORT_LIB_PATH",
        "ORT_PREFER_DYNAMIC_LINK",
        "LD_LIBRARY_PATH",
    ] {
        env.push((
            key.to_string(),
            std::env::var(key).unwrap_or_else(|_| {
                panic!("{key} must be configured before the FastEmbed process test")
            }),
        ));
    }
    env
}

#[cfg(feature = "fastembed")]
struct HangingHttpEndpoint {
    addr: std::net::SocketAddr,
    task: JoinHandle<()>,
}

#[cfg(feature = "fastembed")]
impl HangingHttpEndpoint {
    async fn start() -> (Self, oneshot::Receiver<()>, oneshot::Receiver<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("hanging HuggingFace endpoint should bind a loopback port");
        let addr = listener
            .local_addr()
            .expect("hanging HuggingFace endpoint should expose its address");
        let (request_started_tx, request_started_rx) = oneshot::channel();
        let (connection_closed_tx, connection_closed_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("hanging HuggingFace endpoint should accept metadata request");
            let mut buffer = [0_u8; 4096];
            let first = stream
                .read(&mut buffer)
                .await
                .expect("HuggingFace metadata request should be readable");
            assert!(
                first > 0,
                "HuggingFace metadata request should not be empty"
            );
            let _ = request_started_tx.send(());
            loop {
                match stream.read(&mut buffer).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
            }
            let _ = connection_closed_tx.send(());
        });
        (
            Self { addr, task },
            request_started_rx,
            connection_closed_rx,
        )
    }

    fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    async fn stop(self) {
        tokio::time::timeout(std::time::Duration::from_secs(5), self.task)
            .await
            .expect("hanging HuggingFace endpoint should stop before timeout")
            .expect("hanging HuggingFace endpoint task should join");
    }
}

#[cfg(feature = "fastembed")]
fn model_cache_path(cache_dir: &std::path::Path, model_code: &str) -> std::path::PathBuf {
    cache_dir.join(format!("models--{model_code}").replace('/', "--"))
}

#[cfg(feature = "fastembed")]
fn create_incomplete_cache(cache_path: &std::path::Path, cache_dir: &std::path::Path) {
    std::fs::create_dir_all(cache_path.join("snapshots/incomplete")).unwrap();
    let repo_name = cache_path.file_name().expect("model cache repo name");
    std::fs::create_dir_all(cache_dir.join(".locks").join(repo_name)).unwrap();
}

#[cfg(feature = "fastembed")]
fn create_complete_cache(cache_path: &std::path::Path) {
    let snapshot = cache_path.join("snapshots/complete");
    std::fs::create_dir_all(cache_path.join("refs")).unwrap();
    std::fs::write(cache_path.join("refs/main"), "complete").unwrap();
    for file in [
        "onnx/model.onnx",
        "tokenizer.json",
        "config.json",
        "special_tokens_map.json",
        "tokenizer_config.json",
    ] {
        let path = snapshot.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "x").unwrap();
    }
}

#[cfg(feature = "fastembed")]
#[tokio::test]
async fn embedding_download_stream_idle_releases_huggingface_request() {
    let (endpoint, request_started, connection_closed) = HangingHttpEndpoint::start().await;
    let addr = unused_local_addr();
    let temp_dir = SidecarProcessTestDir::new("embedding-stream-idle");
    let mut embedding_env = embedding_process_env(temp_dir.path());
    embedding_env.push(("HF_ENDPOINT".to_string(), endpoint.base_url()));
    let manager = sidecar_manager_with_dir(
        EMBEDDING_SERVICE_NAME,
        env!("CARGO_BIN_EXE_a4n-embedding-service"),
        addr,
        "A4N_EMBEDDING_SERVICE_ADDR",
        &temp_dir,
        embedding_env,
    );
    let process = start_sidecar(&manager, EMBEDDING_SERVICE_NAME).await;
    let idle_client = HttpEmbeddingClient::new_with_auth_token(
        sidecar_base_url(addr),
        SIDECAR_PROCESS_TEST_TOKEN,
    )
    .with_test_timeouts(
        std::time::Duration::from_secs(1),
        std::time::Duration::from_secs(2),
        std::time::Duration::from_secs(1),
        std::time::Duration::from_secs(1),
    );
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(2);
    let idle_task = tokio::spawn(async move {
        idle_client
            .download_model_events(
                request_context(
                    "embedding-process-stream-idle",
                    "embedding-process-stream-idle-trace",
                    EMBEDDING_CAPABILITY_VERSION,
                    serde_json::json!({"model": "BGESmallZHV15"}),
                ),
                event_tx,
            )
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), request_started)
        .await
        .expect("HuggingFace metadata request should start before timeout")
        .expect("HuggingFace request start signal should be delivered");
    let progress = tokio::time::timeout(std::time::Duration::from_secs(5), event_rx.recv())
        .await
        .expect("embedding sidecar should emit progress before becoming idle")
        .expect("embedding progress event should be delivered");
    assert_eq!(progress.event_kind, CapabilityTaskEventKind::Progress);
    let idle_error = tokio::time::timeout(std::time::Duration::from_secs(5), idle_task)
        .await
        .expect("embedding stream idle timeout should be observed promptly")
        .expect("embedding event client task should join")
        .expect_err("a stalled HuggingFace metadata request must hit the stream idle deadline");
    assert_eq!(idle_error.code, ServiceErrorCode::ServiceUnavailable);
    assert!(idle_error.message.contains("event stream idle timeout"));
    assert!(!idle_error.message.contains(SIDECAR_PROCESS_TEST_TOKEN));
    assert!(!idle_error.message.contains(&endpoint.base_url()));
    tokio::time::timeout(std::time::Duration::from_secs(5), connection_closed)
        .await
        .expect("stream idle cancellation should release the HuggingFace request")
        .expect("HuggingFace connection close signal should be delivered");

    let recovery_client = HttpEmbeddingClient::new_with_auth_token(
        sidecar_base_url(addr),
        SIDECAR_PROCESS_TEST_TOKEN,
    );
    let recovered = recovery_client
        .embed(request_context(
            "embedding-process-after-stream-idle",
            "embedding-process-after-stream-idle-trace",
            EMBEDDING_CAPABILITY_VERSION,
            serde_json::json!({
                "backend": "local_hash",
                "model": "local-sha256-hash",
                "dimension": 8,
                "batch_size": 8,
                "texts": ["still responsive after stream idle"]
            }),
        ))
        .await
        .expect("embedding service should recover after stream idle cancellation");
    assert_eq!(recovered.status, ServiceStatus::Ok);
    finish_sidecar(process, &manager, EMBEDDING_SERVICE_NAME, addr).await;
    endpoint.stop().await;
}

#[tokio::test]
async fn embedding_service_process_serves_local_hash_and_recovers_cache() {
    let addr = unused_local_addr();
    let temp_dir = SidecarProcessTestDir::new("embedding");
    let embedding_env = embedding_process_env(temp_dir.path());
    assert_sidecar_rejects_non_loopback_without_auth(
        "embedding-non-loopback",
        env!("CARGO_BIN_EXE_a4n-embedding-service"),
        "A4N_EMBEDDING_SERVICE_ADDR",
        embedding_env.clone(),
    )
    .await;
    let manager = sidecar_manager_with_dir(
        EMBEDDING_SERVICE_NAME,
        env!("CARGO_BIN_EXE_a4n-embedding-service"),
        addr,
        "A4N_EMBEDDING_SERVICE_ADDR",
        &temp_dir,
        embedding_env,
    );
    let process = start_sidecar(&manager, EMBEDDING_SERVICE_NAME).await;
    assert_process_concurrency_saturation(addr, EMBEDDING_SERVICE_NAME, "/embedding/embed_texts")
        .await;
    let base_url = sidecar_base_url(addr);
    assert_protected_route_rejects_invalid_auth(&base_url, "/embedding/embed_texts").await;
    assert_route_rejects_malformed_json(&base_url, "/embedding/embed_texts").await;
    assert_route_rejects_oversized_json(&base_url, "/embedding/embed_texts", 16 * 1024 * 1024)
        .await;
    let client = HttpEmbeddingClient::new_with_auth_token(base_url, SIDECAR_PROCESS_TEST_TOKEN);
    let request = request_context(
        "embedding-process-embed",
        "embedding-process-trace",
        EMBEDDING_CAPABILITY_VERSION,
        serde_json::json!({
            "backend": "local_hash",
            "model": "local-sha256-hash",
            "dimension": 64,
            "batch_size": 8,
            "texts": ["process test"]
        }),
    );
    assert_route_rejects_unknown_payload_field(
        &sidecar_base_url(addr),
        "/embedding/embed_texts",
        &request,
    )
    .await;
    assert_route_rejects_version_mismatch(
        &sidecar_base_url(addr),
        "/embedding/embed_texts",
        &request,
    )
    .await;
    let response = client
        .embed(request)
        .await
        .expect("local_hash embedding should cross the independent process");
    assert_eq!(response.status, ServiceStatus::Ok);
    assert_eq!(
        response.payload.expect("embedding payload")["embeddings"][0]
            .as_array()
            .map(Vec::len),
        Some(64)
    );

    let remote_response = client
        .embed(trusted_remote_request_context(
            "embedding-process-remote-content",
            "embedding-process-remote-content-trace",
            EMBEDDING_CAPABILITY_VERSION,
            serde_json::json!({
                "backend": "local_hash",
                "model": "local-sha256-hash",
                "dimension": 8,
                "batch_size": 8,
                "texts": ["trusted remote text egress"]
            }),
        ))
        .await
        .expect("secret-free remote embedding content should remain available");
    assert_eq!(remote_response.status, ServiceStatus::Ok);
    assert_eq!(
        remote_response.payload.expect("remote embedding payload")["embeddings"][0]
            .as_array()
            .map(Vec::len),
        Some(8)
    );

    disconnect_during_request_body(addr, "/embedding/embed_texts").await;
    let failed = client
        .embed(request_context(
            "embedding-process-failure",
            "embedding-process-failure-trace",
            EMBEDDING_CAPABILITY_VERSION,
            serde_json::json!({
                "backend": "unsupported-process-backend",
                "model": "local-sha256-hash",
                "dimension": 64,
                "batch_size": 8,
                "texts": ["failure then recovery"]
            }),
        ))
        .await
        .expect("embedding provider failure should return a typed response");
    assert_eq!(failed.status, ServiceStatus::Error);
    assert_eq!(
        failed.error.expect("embedding provider error").code,
        a4n_service_protocol::ServiceErrorCode::ConfigurationError
    );
    let recovered = client
        .embed(request_context(
            "embedding-process-after-failure",
            "embedding-process-after-failure-trace",
            EMBEDDING_CAPABILITY_VERSION,
            serde_json::json!({
                "backend": "local_hash",
                "model": "local-sha256-hash",
                "dimension": 8,
                "batch_size": 8,
                "texts": ["still responsive"]
            }),
        ))
        .await
        .expect("embedding service should recover after a provider failure");
    assert_eq!(recovered.status, ServiceStatus::Ok);
    assert_eq!(
        recovered.payload.expect("recovered embedding payload")["embeddings"][0]
            .as_array()
            .map(Vec::len),
        Some(8)
    );

    #[cfg(feature = "fastembed")]
    let cache_path = {
        let response = client
            .model_operation(
                EmbeddingModelOperation::List,
                request_context(
                    "embedding-process-model-list",
                    "embedding-process-model-list-trace",
                    EMBEDDING_CAPABILITY_VERSION,
                    serde_json::Value::Null,
                ),
            )
            .await
            .expect("FastEmbed model list should cross the independent process");
        assert_eq!(response.status, ServiceStatus::Ok);
        let payload = response.payload.expect("FastEmbed model list payload");
        let model = payload
            .as_array()
            .and_then(|models| {
                models
                    .iter()
                    .find(|model| model["name"] == serde_json::json!("BGESmallZHV15"))
            })
            .expect("default FastEmbed model should be listed");
        assert!(model.get("cache_path").is_none());
        model_cache_path(
            temp_dir.path(),
            model["model_code"]
                .as_str()
                .expect("FastEmbed model code should be text"),
        )
    };

    #[cfg(feature = "fastembed")]
    {
        let event_client = HttpEmbeddingClient::new_with_auth_token(
            sidecar_base_url(addr),
            SIDECAR_PROCESS_TEST_TOKEN,
        );
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(1);
        let event_task = tokio::spawn(async move {
            event_client
                .download_model_events(
                    request_context(
                        "embedding-process-download-backpressure",
                        "embedding-process-download-backpressure-trace",
                        EMBEDDING_CAPABILITY_VERSION,
                        serde_json::json!({
                            "model": "BGESmallZHV15"
                        }),
                    ),
                    event_tx,
                )
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while event_rx.is_empty() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("FastEmbed download should fill the bounded event queue");
        assert!(
            !event_task.is_finished(),
            "a full FastEmbed event queue must apply backpressure"
        );
        drop(event_rx);
        let cancelled = event_task
            .await
            .expect("FastEmbed event client task should join")
            .expect_err("closed receiver should cancel FastEmbed download");
        assert_eq!(cancelled.code, ServiceErrorCode::Cancelled);

        let responsive = client
            .embed(request_context(
                "embedding-process-after-cancel",
                "embedding-process-after-cancel-trace",
                EMBEDDING_CAPABILITY_VERSION,
                serde_json::json!({
                    "backend": "local_hash",
                    "model": "local-sha256-hash",
                    "dimension": 8,
                    "batch_size": 8,
                    "texts": ["still responsive"]
                }),
            ))
            .await
            .expect("embedding service should remain responsive after download cancellation");
        assert_eq!(responsive.status, ServiceStatus::Ok);
    }

    finish_sidecar(process, &manager, EMBEDDING_SERVICE_NAME, addr).await;

    #[cfg(feature = "fastembed")]
    {
        let unknown_cache = temp_dir.path().join("models--unknown--preserve-me");
        std::fs::create_dir_all(&unknown_cache).unwrap();
        std::fs::write(unknown_cache.join("sentinel"), "keep").unwrap();
        create_incomplete_cache(&cache_path, temp_dir.path());
        let lock_path = temp_dir
            .path()
            .join(".locks")
            .join(cache_path.file_name().unwrap());

        let cleaned = start_sidecar(&manager, EMBEDDING_SERVICE_NAME).await;
        assert!(!cache_path.exists());
        assert!(!lock_path.exists());
        assert!(unknown_cache.join("sentinel").is_file());
        finish_sidecar(cleaned, &manager, EMBEDDING_SERVICE_NAME, addr).await;

        create_complete_cache(&cache_path);
        let complete = start_sidecar(&manager, EMBEDDING_SERVICE_NAME).await;
        assert!(cache_path.exists());
        let response = client
            .model_operation(
                EmbeddingModelOperation::List,
                request_context(
                    "embedding-process-complete-model-list",
                    "embedding-process-complete-model-list-trace",
                    EMBEDDING_CAPABILITY_VERSION,
                    serde_json::Value::Null,
                ),
            )
            .await
            .expect("complete FastEmbed cache should remain readable");
        assert_eq!(response.status, ServiceStatus::Ok);
        assert!(
            response
                .payload
                .expect("complete model list payload")
                .as_array()
                .unwrap()
                .iter()
                .any(|model| {
                    model["name"] == serde_json::json!("BGESmallZHV15")
                        && model["downloaded"] == serde_json::json!(true)
                })
        );
        let deleted = client
            .model_operation(
                EmbeddingModelOperation::Delete,
                request_context(
                    "embedding-process-model-delete",
                    "embedding-process-model-delete-trace",
                    EMBEDDING_CAPABILITY_VERSION,
                    serde_json::json!({"model": "BGESmallZHV15"}),
                ),
            )
            .await
            .expect("FastEmbed model delete should cross the independent process");
        assert_eq!(deleted.status, ServiceStatus::Ok);
        assert_eq!(deleted.payload, Some(serde_json::Value::Null));
        assert!(!cache_path.exists());
        force_finish_sidecar(complete, &manager, EMBEDDING_SERVICE_NAME, addr).await;

        create_incomplete_cache(&cache_path, temp_dir.path());
        let recovered = start_sidecar(&manager, EMBEDDING_SERVICE_NAME).await;
        assert!(!cache_path.exists());
        assert!(!lock_path.exists());
        assert!(unknown_cache.join("sentinel").is_file());
        finish_sidecar(recovered, &manager, EMBEDDING_SERVICE_NAME, addr).await;

        let manifest_path = temp_dir.path().join(".a4n-embedding-model-cache-data.json");
        let manifest =
            std::fs::read(&manifest_path).expect("embedding cache manifest should exist");
        let mut incompatible: serde_json::Value = serde_json::from_slice(&manifest)
            .expect("embedding cache manifest should be valid JSON");
        incompatible["format_version"] = serde_json::json!(0);
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&incompatible).unwrap(),
        )
        .unwrap();
        assert!(manager.start_sidecar(EMBEDDING_SERVICE_NAME).await.is_err());
        std::fs::write(&manifest_path, b"{").unwrap();
        assert!(manager.start_sidecar(EMBEDDING_SERVICE_NAME).await.is_err());
        std::fs::write(&manifest_path, manifest).unwrap();
        let final_process = start_sidecar(&manager, EMBEDDING_SERVICE_NAME).await;
        finish_sidecar(final_process, &manager, EMBEDDING_SERVICE_NAME, addr).await;
    }
}
