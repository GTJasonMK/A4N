use a4n_api_contract::embedding::{EmbeddingModelDownloadInfoSummary, LocalEmbeddingModelSummary};
use a4n_embedding::provider::DownloadProgress;
use a4n_embedding_client::{EmbeddingClient, EmbeddingModelOperation, JsonResponse};
use a4n_error::AppError;
use a4n_provider_contract::EmbeddingProviderConfig;
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, EMBEDDING_CAPABILITY_VERSION,
};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use super::super::{
    global_service_request_without_secrets, global_typed_provider_service_request,
    global_typed_service_request_without_secrets, missing_sidecar_capability_message,
    response_payload, service_app_error, service_error_app_error, typed_response_payload,
};
use super::contracts::{
    EmbedTextsOutput, embedding_connectivity_test_payload, embedding_model_payload,
    validate_embedding_connectivity_output,
};

async fn embedding_model_operation<P: serde::Serialize>(
    client: Option<Arc<dyn EmbeddingClient>>,
    operation: EmbeddingModelOperation,
    payload: P,
) -> Result<JsonResponse, AppError> {
    let client = required_embedding_client(client)?;
    let request = global_service_request_without_secrets(EMBEDDING_CAPABILITY_VERSION, payload)
        .map_err(service_app_error)?;
    client
        .model_operation(operation, request)
        .await
        .map_err(service_error_app_error)
}

pub(crate) async fn list_embedding_models(
    client: Option<Arc<dyn EmbeddingClient>>,
) -> Result<Vec<LocalEmbeddingModelSummary>, AppError> {
    let response = embedding_model_operation(client, EmbeddingModelOperation::List, ()).await?;
    response_payload(response, "embedding").map_err(service_app_error)
}

pub(crate) async fn embedding_model_download_info(
    client: Option<Arc<dyn EmbeddingClient>>,
    model: &str,
) -> Result<EmbeddingModelDownloadInfoSummary, AppError> {
    let response = embedding_model_operation(
        client,
        EmbeddingModelOperation::DownloadInfo,
        embedding_model_payload(model),
    )
    .await?;
    response_payload(response, "embedding").map_err(service_app_error)
}

pub(crate) async fn delete_embedding_model(
    client: Option<Arc<dyn EmbeddingClient>>,
    model: &str,
) -> Result<(), AppError> {
    let response = embedding_model_operation(
        client,
        EmbeddingModelOperation::Delete,
        embedding_model_payload(model),
    )
    .await?;
    response_payload(response, "embedding").map_err(service_app_error)
}

pub(crate) async fn test_embedding_config_via_sidecar(
    client: Option<Arc<dyn EmbeddingClient>>,
    model: &EmbeddingProviderConfig,
) -> Result<(), AppError> {
    let client = required_embedding_client(client)?;
    let payload = embedding_connectivity_test_payload(model)?;
    let expected_dimension = payload.dimension;
    let request = global_typed_provider_service_request(EMBEDDING_CAPABILITY_VERSION, payload)
        .map_err(service_app_error)?;
    let response = client
        .embed(request)
        .await
        .map_err(service_error_app_error)?;
    let output: EmbedTextsOutput =
        typed_response_payload(response, "embedding").map_err(service_app_error)?;
    validate_embedding_connectivity_output(&output, expected_dimension)
}

pub(crate) async fn download_embedding_model_events(
    client: Option<Arc<dyn EmbeddingClient>>,
    model: &str,
    progress_tx: Option<UnboundedSender<DownloadProgress>>,
    cancel_token: CancellationToken,
) -> Result<LocalEmbeddingModelSummary, AppError> {
    let client = required_embedding_client(client)?;
    let payload = embedding_model_payload(model);
    let request =
        global_typed_service_request_without_secrets(EMBEDDING_CAPABILITY_VERSION, payload)
            .map_err(service_app_error)?;
    let (event_tx, mut event_rx) =
        tokio::sync::mpsc::channel(a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY);
    let mut request_future = Box::pin(client.download_model_events(request, event_tx));
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
                if let Some(model) = handle_embedding_download_event(event, progress_tx.as_ref())? {
                    return Ok(model);
                }
            }
        }
    }
}

fn required_embedding_client(
    client: Option<Arc<dyn EmbeddingClient>>,
) -> Result<std::sync::Arc<dyn EmbeddingClient>, AppError> {
    client.ok_or_else(|| AppError::ConfigError(missing_sidecar_capability_message("embedding")))
}

pub(super) fn handle_embedding_download_event(
    event: CapabilityTaskEvent,
    progress_tx: Option<&UnboundedSender<DownloadProgress>>,
) -> Result<Option<LocalEmbeddingModelSummary>, AppError> {
    match event.event_kind {
        CapabilityTaskEventKind::Progress => {
            let payload = event.payload.ok_or_else(|| {
                AppError::ConfigError("embedding sidecar progress 事件缺少 payload".to_string())
            })?;
            let progress: DownloadProgress = serde_json::from_value(payload).map_err(|error| {
                AppError::ConfigError(format!(
                    "embedding sidecar progress payload 解析失败: {error}"
                ))
            })?;
            if let Some(progress_tx) = progress_tx {
                let _ = progress_tx.send(progress);
            }
            Ok(None)
        }
        CapabilityTaskEventKind::Completed => {
            let payload = event.payload.ok_or_else(|| {
                AppError::ConfigError("embedding sidecar completed 事件缺少 payload".to_string())
            })?;
            let model = serde_json::from_value(payload).map_err(|error| {
                AppError::ConfigError(format!("embedding sidecar completed 模型解析失败: {error}"))
            })?;
            Ok(Some(model))
        }
        CapabilityTaskEventKind::Failed => {
            let error = event.error.ok_or_else(|| {
                AppError::ConfigError("embedding sidecar failed 事件缺少 error".to_string())
            })?;
            Err(service_error_app_error(error))
        }
        other => Err(AppError::ConfigError(format!(
            "embedding sidecar 下载事件类型不符合合同: {other:?}"
        ))),
    }
}
