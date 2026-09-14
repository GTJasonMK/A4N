use super::super::required_sidecar_client;
use super::contracts::{
    EmbedTextsOutput, embedding_connectivity_test_payload, embedding_model_payload,
    validate_embedding_connectivity_output,
};
use super::download::{
    download_embedding_model_events, handle_embedding_download_event,
    test_embedding_config_via_sidecar,
};
use crate::state::RuntimeProfile;
use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_error::AppError;
use a4n_provider_contract::EmbeddingProviderConfig;
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, ServiceErrorCode, ServiceResponse, ServiceStatus,
};
use a4n_test_support::capability::{PendingEmbeddingDownloadClient, RecordingEmbeddingClient};
use serde_json::Value;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

fn embedding_model(backend: &str) -> EmbeddingProviderConfig {
    EmbeddingProviderConfig {
        id: 1,
        name: format!("{backend}-model"),
        backend: backend.to_string(),
        base_url: None,
        api_key: None,
        model: "test-model".to_string(),
        dimension: 384,
        batch_size: 8,
        local_model_path: None,
    }
}

#[test]
fn embedding_sidecar_accepts_supported_backends_case_insensitively() {
    let model = embedding_model(" FastEmbed ");
    assert_eq!(
        embedding_connectivity_test_payload(&model)
            .expect("fastembed must be supported")
            .backend,
        "fastembed"
    );

    let model = embedding_model(" Local_Hash ");
    assert_eq!(
        embedding_connectivity_test_payload(&model)
            .expect("local_hash must be supported")
            .backend,
        "local_hash"
    );
}

#[test]
fn embedding_sidecar_accepts_api_backend() {
    let mut model = embedding_model("api");
    model.base_url = Some("https://provider.example/v1".to_string());
    assert_eq!(
        embedding_connectivity_test_payload(&model)
            .expect("API embedding must run in the trusted sidecar")
            .backend,
        "api"
    );
    assert!(embedding_connectivity_test_payload(&embedding_model("unknown")).is_err());
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
    let payload = embedding_model_payload("test-model");

    assert_eq!(payload.model, "test-model");
}

#[test]
fn embedding_connectivity_payload_omits_host_cache_path_and_keeps_explicit_model() {
    let mut model = embedding_model("fastembed");
    model.base_url = Some("https://unused.example/v1".to_string());
    model.api_key = Some("unused-secret".to_string());
    model.local_model_path = Some("/host/private/cache".to_string());
    model.batch_size = 16;

    let payload = embedding_connectivity_test_payload(&model).expect("connectivity payload");

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
    let matching = EmbedTextsOutput {
        identity: EmbeddingModelIdentity {
            backend: "FastEmbed".to_string(),
            model: "test-model".to_string(),
            dimension: 3,
        },
        embeddings: vec![vec![0.1, 0.2, 0.3]],
    };
    validate_embedding_connectivity_output(&matching, 3).expect("matching output");

    let wrong_identity = EmbedTextsOutput {
        identity: EmbeddingModelIdentity {
            dimension: 4,
            ..matching.identity.clone()
        },
        embeddings: matching.embeddings.clone(),
    };
    assert!(validate_embedding_connectivity_output(&wrong_identity, 3).is_err());

    let wrong_count = EmbedTextsOutput {
        identity: matching.identity.clone(),
        embeddings: vec![vec![0.1, 0.2, 0.3], vec![0.1, 0.2, 0.3]],
    };
    assert!(validate_embedding_connectivity_output(&wrong_count, 3).is_err());

    let wrong_vector = EmbedTextsOutput {
        identity: matching.identity,
        embeddings: vec![vec![0.1, 0.2]],
    };
    assert!(validate_embedding_connectivity_output(&wrong_vector, 3).is_err());
}

#[tokio::test]
async fn embedding_connectivity_uses_typed_capability_and_deny_all_request() {
    let client = Arc::new(RecordingEmbeddingClient::new(
        successful_embedding_response(),
    ));
    let mut model = embedding_model("fastembed");
    model.local_model_path = Some("/host/private/cache".to_string());

    test_embedding_config_via_sidecar(Some(client.clone()), &model)
        .await
        .expect("sidecar connectivity test");

    assert_eq!(client.embed_call_count(), 1);
    let request = client.last_embed_request().expect("recorded embed request");
    assert!(request.secret_policy.is_deny_all());
    assert_eq!(request.payload.model, "test-model");
    assert_eq!(request.payload.texts, vec!["test"]);
    assert!(request.payload.base_url.is_none());
    assert!(request.payload.api_key.is_none());
}

#[tokio::test]
async fn api_embedding_connectivity_forwards_secret_only_with_local_user_policy() {
    let client = Arc::new(RecordingEmbeddingClient::new(
        successful_embedding_response(),
    ));
    let mut model = embedding_model("api");
    model.base_url = Some("https://provider.example/v1".to_string());
    model.api_key = Some("secret-key".to_string());

    test_embedding_config_via_sidecar(Some(client.clone()), &model)
        .await
        .expect("API embedding connectivity test");
    let request = client.last_embed_request().expect("recorded embed request");
    assert!(request.secret_policy.is_local_user_secret_only());
    assert_eq!(
        request.payload.base_url.as_deref(),
        Some("https://provider.example/v1")
    );
    assert_eq!(request.payload.api_key.as_deref(), Some("secret-key"));
}

#[test]
fn embedding_download_events_map_progress_and_completed_payload() {
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();
    let progress = embedding_download_event(
        CapabilityTaskEventKind::Progress,
        Some(serde_json::json!({
            "downloaded_bytes": 21,
            "total_bytes": 42,
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
        Some(serde_json::json!({
            "name": "model",
            "model_code": "owner/model",
            "description": "test model",
            "dimension": 384,
            "downloaded": true,
            "size_bytes": 42
        })),
        None,
    );
    let model = handle_embedding_download_event(completed, None)
        .expect("completed event")
        .expect("completed model");
    assert_eq!(model.name, "model");
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
    let cancel_token = CancellationToken::new();
    let task_cancel_token = cancel_token.clone();
    let task_client = client.clone();
    let task = tokio::spawn(async move {
        download_embedding_model_events(Some(task_client), "test-model", None, task_cancel_token)
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
    assert_eq!(request.payload.model, "test-model");
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
        error,
        payload,
    }
}

fn successful_embedding_response() -> ServiceResponse<EmbedTextsOutput> {
    ServiceResponse {
        status: ServiceStatus::Ok,
        trace_id: Some("trace".to_string()),
        usage: None,
        artifact_refs: Vec::new(),
        payload: Some(EmbedTextsOutput {
            identity: EmbeddingModelIdentity {
                backend: "FastEmbed".to_string(),
                model: "test-model".to_string(),
                dimension: 384,
            },
            embeddings: vec![vec![0.0_f32; 384]],
        }),
        error: None,
    }
}
