use a4n_core::error::AppError;
use a4n_image_providers::image_provider_request_body_preview;
use a4n_model_discovery::{
    ModelDiscoveryRequest, ModelDiscoveryResult, discover_models_from_request,
};
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::Json;
use serde_json::{Value, json};

use crate::contracts::ImageProviderPayload;
use crate::protocol::{result_response, validate_image_request, validate_model_discovery_request};

pub(crate) async fn request_preview(
    Json(request): Json<ServiceRequest<ImageProviderPayload>>,
) -> Json<ServiceResponse<Value>> {
    if let Err(error) = validate_image_request(&request) {
        return Json(request.error_response(error));
    }
    Json(result_response(
        &request,
        image_request_preview(&request.payload),
    ))
}

pub(crate) async fn discover_models(
    Json(request): Json<ServiceRequest<ModelDiscoveryRequest>>,
) -> Json<ServiceResponse<ModelDiscoveryResult>> {
    if let Err(error) = validate_model_discovery_request(&request) {
        return Json(request.error_response(error));
    }
    let result = discover_models_from_request(request.payload.clone())
        .await
        .map_err(AppError::ConfigError);
    Json(result_response(&request, result))
}

fn image_request_preview(payload: &ImageProviderPayload) -> Result<Value, AppError> {
    image_provider_request_body_preview(&payload.backend, &payload.request)
        .map(|body| json!({ "request": body }))
}
