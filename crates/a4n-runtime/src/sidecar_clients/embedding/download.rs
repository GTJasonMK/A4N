use a4n_capabilities::A4nCapabilities;
use a4n_core::error::AppError;
use a4n_core::models::ai::embedding_config;
use a4n_embedding::provider::DownloadProgress;
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, EMBEDDING_CAPABILITY_VERSION, ServiceErrorCode,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use super::super::{
    global_provider_service_request, global_service_request_without_secrets,
    missing_sidecar_capability_message, service_app_error, service_error_app_error,
    service_response_payload,
};
use super::contracts::{
    EmbeddingTextsOutput, embedding_connectivity_output_matches,
    embedding_connectivity_test_payload, embedding_model_operation_payload, positive_usize,
};

pub(crate) async fn embedding_model_operation<T: DeserializeOwned>(
    capabilities: &A4nCapabilities,
    operation: &str,
    model: &str,
) -> Result<T, AppError> {
    let client = capabilities
        .embedding
        .clone()
        .ok_or_else(|| AppError::ConfigError(missing_sidecar_capability_message("embedding")))?;
    let payload = embedding_model_operation_payload(operation, model);
    let request = global_service_request_without_secrets(EMBEDDING_CAPABILITY_VERSION, payload)
        .map_err(service_app_error)?;
    let response = client
        .model_operation(request)
        .await
        .map_err(service_error_app_error)?;
    service_response_payload(response, "embedding").map_err(service_app_error)
}

pub(crate) async fn test_embedding_config_via_sidecar(
    capabilities: &A4nCapabilities,
    model: &embedding_config::Model,
    text: &str,
) -> Result<bool, AppError> {
    super::provider::SidecarEmbeddingProvider::normalized_sidecar_backend(model)?;
    let expected_dimension = positive_usize(model.dimension, "向量维度", &model.name)?;
    let client = capabilities
        .embedding
        .clone()
        .ok_or_else(|| AppError::ConfigError(missing_sidecar_capability_message("embedding")))?;
    let payload = embedding_connectivity_test_payload(model, text)?;
    let request = global_provider_service_request(
        a4n_service_protocol::EMBEDDING_SERVICE_NAME,
        EMBEDDING_CAPABILITY_VERSION,
        payload,
    )
    .map_err(service_app_error)?;
    let response = client
        .embed(request)
        .await
        .map_err(service_error_app_error)?;
    let output: EmbeddingTextsOutput =
        service_response_payload(response, "embedding").map_err(service_app_error)?;
    Ok(embedding_connectivity_output_matches(
        &output,
        expected_dimension,
    ))
}

pub(crate) async fn download_embedding_model_events(
    capabilities: &A4nCapabilities,
    model: &str,
    progress_tx: Option<UnboundedSender<DownloadProgress>>,
    cancel_token: Option<CancellationToken>,
) -> Result<Value, AppError> {
    let client = capabilities
        .embedding
        .clone()
        .ok_or_else(|| AppError::ConfigError(missing_sidecar_capability_message("embedding")))?;
    let payload = embedding_model_operation_payload("download", model);
    let request = global_service_request_without_secrets(EMBEDDING_CAPABILITY_VERSION, payload)
        .map_err(service_app_error)?;
    let (event_tx, mut event_rx) =
        tokio::sync::mpsc::channel(a4n_capabilities::EVENT_QUEUE_CAPACITY);
    let mut request_future = Box::pin(client.download_model_events(request, event_tx));
    let cancel_token = cancel_token.unwrap_or_default();
    let mut transport_complete = false;

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => return Err(AppError::Cancelled),
            result = &mut request_future, if !transport_complete => {
                result.map_err(service_error_app_error)?;
                transport_complete = true;
            }
            event = event_rx.recv() => {
                let event = event.ok_or_else(|| AppError::ConfigError(
                    "embedding sidecar 下载事件流结束但缺少终止事件".to_string()
                ))?;
                if let Some(models) = handle_embedding_download_event(event, progress_tx.as_ref())? {
                    return Ok(models);
                }
            }
        }
    }
}

pub(super) fn handle_embedding_download_event(
    event: CapabilityTaskEvent,
    progress_tx: Option<&UnboundedSender<DownloadProgress>>,
) -> Result<Option<Value>, AppError> {
    match event.event_kind {
        CapabilityTaskEventKind::Progress | CapabilityTaskEventKind::StructuredProgress => {
            let payload = event.payload.ok_or_else(|| {
                AppError::ConfigError("embedding sidecar progress 事件缺少 payload".to_string())
            })?;
            let progress: SidecarEmbeddingDownloadProgress = serde_json::from_value(payload)
                .map_err(|error| {
                    AppError::ConfigError(format!(
                        "embedding sidecar progress payload 解析失败: {error}"
                    ))
                })?;
            if let Some(progress_tx) = progress_tx {
                let _ = progress_tx.send(progress.into());
            }
            Ok(None)
        }
        CapabilityTaskEventKind::Completed => {
            let mut payload = event.payload.ok_or_else(|| {
                AppError::ConfigError("embedding sidecar completed 事件缺少 payload".to_string())
            })?;
            let models = payload.get_mut("models").map(Value::take).ok_or_else(|| {
                AppError::ConfigError("embedding sidecar completed 事件缺少 models".to_string())
            })?;
            Ok(Some(models))
        }
        CapabilityTaskEventKind::Failed => {
            let error = event.error.ok_or_else(|| {
                AppError::ConfigError("embedding sidecar failed 事件缺少 error".to_string())
            })?;
            if error.code == ServiceErrorCode::Cancelled {
                Err(AppError::Cancelled)
            } else {
                Err(service_error_app_error(error))
            }
        }
        CapabilityTaskEventKind::CancelAck => Err(AppError::Cancelled),
        other => Err(AppError::ConfigError(format!(
            "embedding sidecar 下载事件类型不符合合同: {other:?}"
        ))),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SidecarEmbeddingDownloadProgress {
    downloaded_bytes: u64,
    total_bytes: u64,
    percent: f32,
    message: String,
}

impl From<SidecarEmbeddingDownloadProgress> for DownloadProgress {
    fn from(value: SidecarEmbeddingDownloadProgress) -> Self {
        Self {
            downloaded_bytes: value.downloaded_bytes,
            total_bytes: value.total_bytes,
            percent: value.percent,
            message: value.message,
        }
    }
}
