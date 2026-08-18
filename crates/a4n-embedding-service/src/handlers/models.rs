use std::path::PathBuf;

use a4n_core::config::env::optional_non_empty_env_path;
use a4n_core::error::AppError;
use a4n_domain::embedding::{
    EmbeddingModelDownloadInfoSummary, LocalEmbeddingModelSummary,
};
use a4n_embedding_fastembed::{
    delete_fastembed_model, fastembed_model_download_info, list_fastembed_local_models,
};
use a4n_service_protocol::{
    EMBEDDING_CACHE_DIR_ENV, EMBEDDING_SERVICE_NAME, EmbeddingModelPayload, ServiceRequest,
    ServiceResponse,
};
use axum::Json;

use crate::protocol::service_error;

pub(crate) async fn list_models(
    Json(request): Json<ServiceRequest<()>>,
) -> Json<ServiceResponse<Vec<LocalEmbeddingModelSummary>>> {
    if let Err(error) = request.validate_known_service(EMBEDDING_SERVICE_NAME) {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let result = resolve_cache_dir().and_then(|cache_dir| list_fastembed_local_models(&cache_dir));
    Json(ServiceResponse::from_result(trace_id, result, service_error))
}

pub(crate) async fn model_download_info(
    Json(request): Json<ServiceRequest<EmbeddingModelPayload>>,
) -> Json<ServiceResponse<EmbeddingModelDownloadInfoSummary>> {
    if let Err(error) = request.validate_known_service(EMBEDDING_SERVICE_NAME) {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let result = async {
        let model = required_model(request.payload)?;
        let cache_dir = resolve_cache_dir()?;
        let info = fastembed_model_download_info(&model, cache_dir).await?;
        Ok(info)
    }
    .await;
    Json(ServiceResponse::from_result(trace_id, result, service_error))
}

pub(crate) async fn delete_model(
    Json(request): Json<ServiceRequest<EmbeddingModelPayload>>,
) -> Json<ServiceResponse<()>> {
    if let Err(error) = request.validate_known_service(EMBEDDING_SERVICE_NAME) {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let result = required_model(request.payload).and_then(|model| {
        resolve_cache_dir().and_then(|cache_dir| delete_fastembed_model(&model, &cache_dir))
    });
    Json(ServiceResponse::from_result(trace_id, result, service_error))
}

pub(crate) fn required_model(payload: EmbeddingModelPayload) -> Result<String, AppError> {
    let model = payload.model.trim();
    if model.is_empty() {
        return Err(AppError::InvalidArgument(
            "embedding 模型操作必须提供模型名".to_string(),
        ));
    }
    Ok(model.to_string())
}

pub(crate) fn resolve_cache_dir() -> Result<PathBuf, AppError> {
    optional_non_empty_env_path(EMBEDDING_CACHE_DIR_ENV)?.ok_or_else(|| {
        AppError::ConfigError(format!("{EMBEDDING_CACHE_DIR_ENV} 未设置"))
    })
}
