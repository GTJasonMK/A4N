use super::super::required_sidecar_client;
use super::contracts::{
    EmbeddingTextsOutput, embedding_connectivity_output_matches,
    embedding_connectivity_test_payload, embedding_model_operation_payload,
};
use super::download::{
    download_embedding_model_events, handle_embedding_download_event,
    test_embedding_config_via_sidecar,
};
use super::provider::SidecarEmbeddingProvider;
use crate::state::RuntimeProfile;
use a4n_capabilities::A4nCapabilities;
use a4n_capabilities::JsonResponse;
use a4n_core::error::AppError;
use a4n_core::models::ai::embedding_config;
use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, ServiceErrorCode, ServiceResponse, ServiceStatus,
};
use a4n_test_support::capability::{PendingEmbeddingDownloadClient, RecordingEmbeddingClient};
use serde_json::Value;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

fn embedding_model(backend: &str) -> embedding_config::Model {
    let now = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    embedding_config::Model {
        id: 1,
        name: format!("{backend}-model"),
        backend: backend.to_string(),
        base_url: None,
        api_key_encrypted: None,
        model: "test-model".to_string(),
        dimension: 384,
        batch_size: 8,
        local_model_path: None,
        is_default: true,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn embedding_sidecar_accepts_supported_backends_case_insensitively() {
    let model = embedding_model(" FastEmbed ");
    assert_eq!(
        SidecarEmbeddingProvider::normalized_sidecar_backend(&model)
            .expect("fastembed must be supported"),
        "fastembed"
    );

    let model = embedding_model(" Local_Hash ");
    assert_eq!(
        SidecarEmbeddingProvider::normalized_sidecar_backend(&model)
            .expect("local_hash must be supported"),
        "local_hash"
    );
}

#[test]
fn embedding_sidecar_accepts_api_backend() {
    let model = embedding_model("api");
    assert_eq!(
        SidecarEmbeddingProvider::normalized_sidecar_backend(&model)
            .expect("API embedding must run in the trusted sidecar"),
        "api"
    );
}

#[test]
fn sidecar_provider_factories_require_capability_at_construction_time() {
    let embedding_error = required_sidecar_client::<()>(RuntimeProfile::Sidecar, None, "embedding")
        .expect_err("missing embedding capability must fail during construction");
    assert!(matches!(
        embedding_error,
        AppError::ConfigError(message)
            if message.contains("embedding sidecar capability 未配置")
    ));

    let llm_error = required_sidecar_client::<()>(RuntimeProfile::Sidecar, None, "LLM")
        .expect_err("missing LLM capability must fail during construction");
    assert!(matches!(
        llm_error,
        AppError::ConfigError(message) if message.contains("LLM sidecar capability 未配置")
    ));
}

#[test]
fn embedded_provider_factories_do_not_require_sidecar_capability() {
    assert_eq!(
        required_sidecar_client::<()>(RuntimeProfile::Full, None, "LLM")
            .expect("full profile must allow embedded provider"),
        None
    );
    assert_eq!(
        required_sidecar_client::<()>(RuntimeProfile::Minimal, None, "embedding")
            .expect("minimal profile must allow embedded provider"),
        None
    );
}

#[test]
fn embedding_model_sidecar_payload_never_forwards_host_cache_path() {
    let payload = embedding_model_operation_payload("download_info", "test-model");

    assert_eq!(payload.operation, "download_info");
    assert_eq!(payload.model, "test-model");
}

#[test]
fn embedding_connectivity_payload_omits_host_cache_path_and_keeps_explicit_model() {
    let mut model = embedding_model("fastembed");
    model.local_model_path = Some("/host/private/cache".to_string());
    model.batch_size = 16;

    let payload =
        embedding_connectivity_test_payload(&model, "test").expect("connectivity payload");

    assert_eq!(payload.model, "test-model");
    assert_eq!(payload.backend, "fastembed");
    assert_eq!(payload.dimension, 384);
    assert_eq!(payload.batch_size, 16);
    assert_eq!(payload.texts, vec!["test"]);
    assert!(payload.base_url.is_none());
    assert!(payload.api_key.is_none());
}

#[test]
fn embedding_connectivity_response_checks_identity_count_and_vector_dimension() {
    let matching = EmbeddingTextsOutput {
        identity: EmbeddingModelIdentity {
            backend: "FastEmbed".to_string(),
            model: "test-model".to_string(),
            dimension: 3,
        },
        embeddings: vec![vec![0.1, 0.2, 0.3]],
    };
    assert!(embedding_connectivity_output_matches(&matching, 3));

    let wrong_identity = EmbeddingTextsOutput {
        identity: EmbeddingModelIdentity {
            dimension: 4,
            ..matching.identity.clone()
        },
        embeddings: matching.embeddings.clone(),
    };
    assert!(!embedding_connectivity_output_matches(&wrong_identity, 3));

    let wrong_count = EmbeddingTextsOutput {
        identity: matching.identity.clone(),
        embeddings: vec![vec![0.1, 0.2, 0.3], vec![0.1, 0.2, 0.3]],
    };
    assert!(!embedding_connectivity_output_matches(&wrong_count, 3));

    let wrong_vector = EmbeddingTextsOutput {
        identity: matching.identity,
        embeddings: vec![vec![0.1, 0.2]],
    };
    assert!(!embedding_connectivity_output_matches(&wrong_vector, 3));
}

#[tokio::test]
async fn embedding_connectivity_uses_typed_capability_and_deny_all_request() {
    let client = Arc::new(RecordingEmbeddingClient::new(
        successful_embedding_response(),
    ));
    let capabilities = A4nCapabilities::empty().with_embedding(client.clone());
    let mut model = embedding_model("fastembed");
    model.local_model_path = Some("/host/private/cache".to_string());

    let connected = test_embedding_config_via_sidecar(&capabilities, &model, "test")
        .await
        .expect("sidecar connectivity test");

    assert!(connected);
    assert_eq!(client.embed_call_count(), 1);
    let request = client.last_embed_request().expect("recorded embed request");
    assert!(request.secret_policy.is_deny_all());
    assert_eq!(request.payload["model"], "test-model");
    assert_eq!(request.payload["texts"], serde_json::json!(["test"]));
    assert!(request.payload.get("cache_dir").is_none());
    assert!(request.payload.get("api_key").is_none());
}

#[tokio::test]
async fn api_embedding_connectivity_forwards_secret_only_with_local_user_policy() {
    let client = Arc::new(RecordingEmbeddingClient::new(
        successful_embedding_response(),
    ));
    let capabilities = A4nCapabilities::empty().with_embedding(client.clone());
    let mut model = embedding_model("api");
    model.base_url = Some("https://provider.example/v1".to_string());
    model.api_key_encrypted = Some("secret-key".to_string());

    assert!(
        test_embedding_config_via_sidecar(&capabilities, &model, "test")
            .await
            .expect("API embedding connectivity test")
    );
    let request = client.last_embed_request().expect("recorded embed request");
    assert!(request.secret_policy.is_local_user_secret_only());
    assert_eq!(request.payload["base_url"], "https://provider.example/v1");
    assert_eq!(request.payload["api_key"], "secret-key");
}

#[test]
fn embedding_download_events_map_progress_and_completed_payload() {
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();
    let progress = embedding_download_event(
        CapabilityTaskEventKind::Progress,
        Some(serde_json::json!({
            "downloaded_bytes": 21,
            "total_bytes": 42,
            "percent": 50.0,
            "message": "downloading"
        })),
        None,
    );

    assert!(
        handle_embedding_download_event(progress, Some(&progress_tx))
            .expect("progress event")
            .is_none()
    );
    let progress = progress_rx.try_recv().expect("forwarded progress");
    assert_eq!(progress.downloaded_bytes, 21);
    assert_eq!(progress.total_bytes, 42);

    let completed = embedding_download_event(
        CapabilityTaskEventKind::Completed,
        Some(serde_json::json!({"models": [{"name": "model"}]})),
        None,
    );
    let models = handle_embedding_download_event(completed, None)
        .expect("completed event")
        .expect("completed models");
    assert_eq!(models[0]["name"], "model");
}

#[test]
fn embedding_download_events_preserve_failure_and_cancel_semantics() {
    let failed = embedding_download_event(
        CapabilityTaskEventKind::Failed,
        None,
        Some(a4n_service_protocol::ServiceError {
            code: ServiceErrorCode::ProviderError,
            message: "provider failed".to_string(),
            retryable: Some(false),
        }),
    );
    let error = handle_embedding_download_event(failed, None).expect_err("failed event must fail");
    assert!(matches!(error, AppError::ConfigError(message) if message == "provider failed"));

    let cancel = embedding_download_event(CapabilityTaskEventKind::CancelAck, None, None);
    assert!(matches!(
        handle_embedding_download_event(cancel, None),
        Err(AppError::Cancelled)
    ));

    let cancelled_failure = embedding_download_event(
        CapabilityTaskEventKind::Failed,
        None,
        Some(a4n_service_protocol::ServiceError {
            code: ServiceErrorCode::Cancelled,
            message: "cancelled".to_string(),
            retryable: Some(false),
        }),
    );
    assert!(matches!(
        handle_embedding_download_event(cancelled_failure, None),
        Err(AppError::Cancelled)
    ));
}

#[tokio::test]
async fn host_cancel_drops_embedding_sidecar_event_stream() {
    let client = Arc::new(PendingEmbeddingDownloadClient::new());
    let observer = client.observer();
    let capabilities = A4nCapabilities::empty().with_embedding(client.clone());
    let cancel_token = CancellationToken::new();
    let task_cancel_token = cancel_token.clone();
    let task = tokio::spawn(async move {
        download_embedding_model_events(&capabilities, "test-model", None, Some(task_cancel_token))
            .await
    });

    observer.wait_until_started().await;
    cancel_token.cancel();
    let error = task
        .await
        .expect("download task join")
        .expect_err("cancelled download must fail");

    assert!(matches!(error, AppError::Cancelled));
    assert_eq!(client.download_model_events_call_count(), 1);
    let request = client
        .last_download_model_events_request()
        .expect("recorded embedding download request");
    assert!(request.secret_policy.is_deny_all());
    assert_eq!(request.payload["model"], "test-model");
    assert!(
        observer.stream_was_dropped(),
        "dropping the host request future must close the sidecar HTTP event stream"
    );
}

fn embedding_download_event(
    event_kind: CapabilityTaskEventKind,
    payload: Option<Value>,
    error: Option<a4n_service_protocol::ServiceError>,
) -> CapabilityTaskEvent {
    CapabilityTaskEvent {
        task_id: "download-task".to_string(),
        event_kind,
        sequence: 1,
        chunk_index: None,
        chunk_delta: None,
        usage_delta: None,
        final_usage: None,
        finish_reason: None,
        is_truncated_finish_reason: None,
        checkpoint_key: None,
        artifact_refs: Vec::new(),
        error,
        payload,
    }
}

fn successful_embedding_response() -> JsonResponse {
    ServiceResponse {
        status: ServiceStatus::Ok,
        trace_id: Some("trace".to_string()),
        usage: None,
        artifact_refs: Vec::new(),
        payload: Some(serde_json::json!({
            "identity": {
                "backend": "FastEmbed",
                "model": "test-model",
                "dimension": 384
            },
            "embeddings": [vec![0.0_f32; 384]]
        })),
        error: None,
    }
}
