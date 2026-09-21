use a4n_domain::ProviderError as AppError;
use a4n_embedding_contract::EmbedTextsPayload;
#[cfg(feature = "fastembed")]
use a4n_embedding_contract::EmbeddingModelPayload;
use a4n_service_protocol::{
    EMBEDDING_CAPABILITY_VERSION, EMBEDDING_SERVICE_NAME, PROTOCOL_VERSION, ServiceErrorCode,
    ServiceRequest, ServiceResponse, ServiceStatus,
};

use crate::handlers::embedding::embedding_provider;
#[cfg(feature = "fastembed")]
use crate::handlers::models::required_model;
#[cfg(feature = "fastembed")]
use crate::protocol::service_error;
use crate::protocol::validate_embedding_request;

#[tokio::test]
async fn local_hash_embedding_runs_inside_sidecar_contract() {
    let provider = embedding_provider(&EmbedTextsPayload {
        backend: "local_hash".to_string(),
        base_url: None,
        api_key: None,
        model: "local-sha256-hash".to_string(),
        dimension: 64,
        batch_size: 8,
        texts: vec!["测试".to_string()],
    })
    .expect("local_hash sidecar provider");

    let identity = provider.model_identity().await.expect("model identity");
    let embeddings = provider
        .embed(vec!["测试".to_string()])
        .await
        .expect("local hash embedding");
    assert_eq!(identity.backend, "LocalHash");
    assert_eq!(identity.model, "local-sha256-hash");
    assert_eq!(identity.dimension, 64);
    assert_eq!(embeddings.len(), 1);
    assert_eq!(embeddings[0].len(), 64);
}

#[test]
fn embedding_sidecar_accepts_api_backend() {
    let provider = embedding_provider(&EmbedTextsPayload {
        backend: "api".to_string(),
        base_url: Some("https://provider.example/v1".to_string()),
        api_key: Some("secret".to_string()),
        model: "remote-model".to_string(),
        dimension: 384,
        batch_size: 8,
        texts: Vec::new(),
    })
    .expect("API embedding must use the trusted provider sidecar");

    assert_eq!(provider.model_name(), "remote-model");
    assert_eq!(provider.dimension(), 384);
}

#[test]
fn embedding_sidecar_rejects_zero_batch_size() {
    let result = embedding_provider(&EmbedTextsPayload {
        backend: "local_hash".to_string(),
        base_url: None,
        api_key: None,
        model: "local-sha256-hash".to_string(),
        dimension: 64,
        batch_size: 0,
        texts: Vec::new(),
    });

    assert!(matches!(
        result,
        Err(AppError::Configuration(message)) if message.contains("batch_size 必须大于 0")
    ));
}

#[test]
fn embedding_payload_rejects_removed_host_cache_path() {
    let error = serde_json::from_value::<EmbedTextsPayload>(serde_json::json!({
        "backend": "local_hash",
        "model": "local-sha256-hash",
        "dimension": 64,
        "batch_size": 8,
        "cache_dir": "/host/private/cache",
        "texts": ["测试"]
    }))
    .expect_err("embedding v2 must reject host filesystem paths");

    assert!(error.to_string().contains("unknown field `cache_dir`"));
}

#[test]
fn embedding_payload_requires_batch_size() {
    let error = serde_json::from_value::<EmbedTextsPayload>(serde_json::json!({
        "backend": "local_hash",
        "model": "local-sha256-hash",
        "dimension": 64,
        "texts": ["测试"]
    }))
    .expect_err("embedding v4 requires an explicit batch_size");

    assert!(error.to_string().contains("missing field `batch_size`"));
}

#[test]
#[cfg(feature = "fastembed")]
fn model_operation_requires_a_nonblank_model() {
    let missing = serde_json::from_value::<EmbeddingModelPayload>(serde_json::json!({}))
        .expect_err("model operation payload requires a model field");
    assert!(missing.to_string().contains("missing field `model`"));

    let blank =
        serde_json::from_value::<EmbeddingModelPayload>(serde_json::json!({ "model": "  " }))
            .expect("blank model is validated after deserialization");
    assert!(required_model(blank).is_err());
}

#[test]
fn api_key_requires_local_user_secret_policy() {
    let payload = EmbedTextsPayload {
        backend: "api".to_string(),
        base_url: Some("https://provider.example/v1".to_string()),
        api_key: Some("secret".to_string()),
        model: "remote-model".to_string(),
        dimension: 384,
        batch_size: 8,
        texts: vec!["test".to_string()],
    };
    let denied = service_request(payload);
    assert_eq!(
        validate_embedding_request(&denied)
            .expect_err("deny-all request cannot carry an API key")
            .code,
        ServiceErrorCode::Unauthorized
    );

    let mut allowed = service_request(denied.payload);
    allowed.secret_policy = a4n_service_protocol::SecretForwardingPolicy::local_user_secret();
    validate_embedding_request(&allowed).expect("local provider sidecar may receive user key");
}

#[test]
#[cfg(feature = "fastembed")]
fn service_error_redacts_secret_material() {
    let message = service_error(AppError::Configuration(
        "base_url=https://provider.example api_key=sk-test".to_string(),
    ))
    .message;
    assert!(message.contains("base_url=[REDACTED]"));
    assert!(message.contains("api_key=[REDACTED]"));
    assert!(!message.contains("provider.example"));
    assert!(!message.contains("sk-test"));
}

#[test]
fn rejects_incompatible_capability_version() {
    let mut request = service_request(serde_json::json!({}));
    request.capability_version = "embedding-v0".to_string();
    let error = request
        .validate_known_service(EMBEDDING_SERVICE_NAME)
        .expect_err("version mismatch");
    assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
    assert!(error.message.contains("embedding capability version"));
}

#[test]
fn rejects_incompatible_protocol_version() {
    let mut request = service_request(serde_json::json!({}));
    request.schema_version.protocol = PROTOCOL_VERSION + 1;
    let response: ServiceResponse<()> = request.error_response(
        request
            .validate_known_service(EMBEDDING_SERVICE_NAME)
            .unwrap_err(),
    );
    assert_eq!(response.status, ServiceStatus::Error);
    assert_eq!(response.trace_id.as_deref(), Some("trace-1"));
    let error = response.error.expect("version mismatch error");
    assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
    assert!(error.message.contains("unsupported protocol version"));
}

fn service_request<T>(payload: T) -> ServiceRequest<T> {
    ServiceRequest {
        request_id: "request-1".to_string(),
        trace_context: a4n_service_protocol::TraceContext {
            trace_id: "trace-1".to_string(),
            parent_span_id: None,
        },
        schema_version: a4n_service_protocol::SchemaVersion::default(),
        user_id: None,
        project_id: None,
        root_project_id: None,
        runtime_config_snapshot: None,
        secret_policy: a4n_service_protocol::SecretForwardingPolicy::deny_all(),
        capability_version: EMBEDDING_CAPABILITY_VERSION.to_string(),
        payload,
    }
}
