use std::path::PathBuf;

use a4n_core::config::env::optional_non_empty_env_path;
use a4n_core::error::AppError;
use a4n_embedding_fastembed::{
    delete_fastembed_model, fastembed_model_download_info, list_fastembed_local_models,
};
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::Json;

use crate::contracts::ModelPayload;
use crate::protocol::{error_response, result_response, validate_service_request};

pub(crate) async fn list_models(
    Json(request): Json<ServiceRequest<ModelPayload>>,
) -> Json<ServiceResponse<serde_json::Value>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = request.payload.validate_operation("list").and_then(|_| {
        let cache_dir = resolve_cache_dir()?;
        list_fastembed_local_models(&cache_dir).and_then(|models| {
            serde_json::to_value(models).map_err(a4n_core::error::AppError::from)
        })
    });
    Json(result_response(&request, result))
}

pub(crate) async fn model_download_info(
    Json(request): Json<ServiceRequest<ModelPayload>>,
) -> Json<ServiceResponse<serde_json::Value>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    if let Err(error) = request.payload.validate_operation("download_info") {
        return Json(result_response(&request, Err(error)));
    }
    let result = async {
        let model = request.payload.required_model()?.to_string();
        let cache_dir = resolve_cache_dir()?;
        let info = fastembed_model_download_info(&model, cache_dir).await?;
        serde_json::to_value(info).map_err(a4n_core::error::AppError::from)
    }
    .await;
    Json(result_response(&request, result))
}

pub(crate) async fn delete_model(
    Json(request): Json<ServiceRequest<ModelPayload>>,
) -> Json<ServiceResponse<serde_json::Value>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    if let Err(error) = request.payload.validate_operation("delete") {
        return Json(result_response(&request, Err(error)));
    }
    let model = match request.payload.required_model() {
        Ok(model) => model,
        Err(error) => return Json(result_response(&request, Err(error))),
    };
    let result = resolve_cache_dir().and_then(|cache_dir| {
        delete_fastembed_model(model, &cache_dir).map(|_| serde_json::Value::Null)
    });
    Json(result_response(&request, result))
}

pub(crate) fn resolve_cache_dir() -> Result<PathBuf, AppError> {
    Ok(optional_non_empty_env_path("A4N_EMBEDDING_CACHE_DIR")?
        .unwrap_or_else(|| PathBuf::from("data/cache/embedding-models")))
}
