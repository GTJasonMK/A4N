use a4n_core::error::AppError;
use a4n_service_protocol::{
    EMBEDDING_CAPABILITY_VERSION, PROTOCOL_VERSION, ServiceErrorCode, ServiceRequest,
    ServiceResponse, ServiceStatus,
};

#[cfg(feature = "fastembed")]
use crate::handlers::events::{
    completed_download_event, download_progress_event, event_line, failed_download_event,
};
#[cfg(feature = "fastembed")]
use crate::protocol::service_error;
use crate::protocol::{error_response, validate_service_request};
use crate::{contracts::EmbedTextsPayload, handlers::embedding::embedding_provider};
#[cfg(feature = "fastembed")]
use a4n_embedding::provider::DownloadProgress;
#[cfg(feature = "fastembed")]
use a4n_service_protocol::{CapabilityTaskEvent, CapabilityTaskEventKind};

#[tokio::test]
async fn local_hash_embedding_runs_inside_sidecar_contract() {
    let provider = embedding_provider(&EmbedTextsPayload {
        backend: "local_hash".to_string(),
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
fn embedding_sidecar_rejects_api_backend_without_host_fallback() {
    let result = embedding_provider(&EmbedTextsPayload {
        backend: "api".to_string(),
        model: "remote-model".to_string(),
        dimension: 384,
        batch_size: 8,
        texts: Vec::new(),
    });
    let error = match result {
        Ok(_) => panic!("API backend requires a dedicated trusted provider sidecar"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        AppError::ConfigError(message)
            if message.contains("不接收 API embedding")
                && message.contains("受信 provider sidecar")
    ));
}

#[test]
fn embedding_sidecar_rejects_zero_batch_size() {
    let result = embedding_provider(&EmbedTextsPayload {
        backend: "local_hash".to_string(),
        model: "local-sha256-hash".to_string(),
        dimension: 64,
        batch_size: 0,
        texts: Vec::new(),
    });

    assert!(matches!(
        result,
        Err(AppError::ConfigError(message)) if message.contains("batch_size 必须大于 0")
    ));
}

#[test]
fn embedding_v2_payload_rejects_removed_host_cache_path() {
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
#[cfg(feature = "fastembed")]
fn download_progress_event_preserves_progress_payload() {
    let event = download_progress_event(
        "download-1".to_string(),
        1,
        DownloadProgress {
            downloaded_bytes: 512,
            total_bytes: 1024,
            percent: 50.0,
            message: "downloading".to_string(),
        },
    )
    .expect("progress event");
    assert_eq!(event.task_id, "download-1");
    assert_eq!(event.event_kind, CapabilityTaskEventKind::Progress);
    assert_eq!(event.sequence, 1);
    assert!(event.error.is_none());
    let payload = event.payload.expect("progress payload");
    assert_eq!(payload["downloaded_bytes"], 512);
    assert_eq!(payload["total_bytes"], 1024);
    assert_eq!(payload["percent"], 50.0);
    assert_eq!(payload["message"], "downloading");
}

#[test]
#[cfg(feature = "fastembed")]
fn completed_download_event_wraps_model_list_without_error() {
    let event = completed_download_event(
        "download-1".to_string(),
        2,
        serde_json::json!([{ "model": "BAAI/bge-small-zh-v1.5" }]),
    );
    assert_eq!(event.event_kind, CapabilityTaskEventKind::Completed);
    assert_eq!(event.sequence, 2);
    assert!(event.error.is_none());
    assert_eq!(
        event.payload.expect("completed payload")["models"][0]["model"],
        "BAAI/bge-small-zh-v1.5"
    );
}

#[test]
#[cfg(feature = "fastembed")]
fn failed_download_event_redacts_secret_material() {
    let event = failed_download_event(
        "download-1".to_string(),
        3,
        service_error(&AppError::ConfigError(
            "base_url=https://provider.example api_key=sk-test".to_string(),
        )),
    );
    assert_eq!(event.event_kind, CapabilityTaskEventKind::Failed);
    assert!(event.payload.is_none());
    let message = event.error.expect("failed error").message;
    assert!(message.contains("base_url=[REDACTED]"));
    assert!(message.contains("api_key=[REDACTED]"));
    assert!(!message.contains("provider.example"));
    assert!(!message.contains("sk-test"));
}

#[test]
#[cfg(feature = "fastembed")]
fn event_line_is_newline_delimited_json() {
    let line = event_line(completed_download_event(
        "download-1".to_string(),
        1,
        serde_json::json!([]),
    ));
    assert_eq!(line.last(), Some(&b'\n'));
    let event: CapabilityTaskEvent =
        serde_json::from_slice(&line[..line.len() - 1]).expect("event json");
    assert_eq!(event.task_id, "download-1");
    assert_eq!(event.event_kind, CapabilityTaskEventKind::Completed);
}

#[test]
fn rejects_secret_forwarding_policy_before_json_or_event_work() {
    let mut request = service_request(serde_json::json!({}));
    request.secret_policy = a4n_service_protocol::SecretForwardingPolicy::local_user_secret();
    let error = validate_service_request(&request).expect_err("secret policy rejection");
    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("secret forwarding policy"));
    let response: ServiceResponse<()> = error_response(&request, error.clone());
    assert_eq!(response.status, ServiceStatus::Error);
    assert_eq!(response.trace_id.as_deref(), Some("trace-1"));
    assert_eq!(
        response.error.expect("json error").code,
        ServiceErrorCode::Unauthorized
    );
    #[cfg(feature = "fastembed")]
    {
        let event = failed_download_event("request-1".to_string(), 1, error);
        assert_eq!(event.event_kind, CapabilityTaskEventKind::Failed);
        assert_eq!(event.sequence, 1);
        assert_eq!(
            event.error.expect("event error").code,
            ServiceErrorCode::Unauthorized
        );
    }
}

#[test]
fn rejects_incompatible_capability_version() {
    let mut request = service_request(serde_json::json!({}));
    request.capability_version = "embedding-v0".to_string();
    let error = validate_service_request(&request).expect_err("version mismatch");
    assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
    assert!(error.message.contains("embedding capability version"));
}

#[test]
fn rejects_incompatible_protocol_version() {
    let mut request = service_request(serde_json::json!({}));
    request.schema_version.protocol = PROTOCOL_VERSION + 1;
    let response: ServiceResponse<()> =
        error_response(&request, validate_service_request(&request).unwrap_err());
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
