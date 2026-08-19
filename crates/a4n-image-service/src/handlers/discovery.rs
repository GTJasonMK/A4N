use a4n_core::error::AppError;
use a4n_image_providers::image_provider_request_body_preview;
use a4n_model_discovery::{
    ModelDiscoveryRequest, ModelDiscoveryResult, discover_models_from_request,
};
use a4n_service_protocol::{IMAGE_SERVICE_NAME, ServiceRequest, ServiceResponse};
use axum::Json;
use serde_json::Value;

use crate::contracts::ImageProviderPayload;
use crate::protocol::{result_response, service_error, validate_image_request};

pub(crate) async fn request_preview(
    Json(request): Json<ServiceRequest<ImageProviderPayload>>,
) -> Json<ServiceResponse<Value>> {
    if let Err(error) = validate_image_request(&request) {
        return Json(request.error_response(error));
    }
    Json(result_response(
        &request,
        image_provider_request_body_preview(&request.payload.backend, &request.payload.request),
    ))
}

pub(crate) async fn discover_models(
    Json(request): Json<ServiceRequest<ModelDiscoveryRequest>>,
) -> Json<ServiceResponse<ModelDiscoveryResult>> {
    if let Err(error) = request
        .validate_known_provider_request(IMAGE_SERVICE_NAME, request.payload.api_key.as_deref())
    {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let result = discover_models_from_request(request.payload)
        .await
        .map_err(AppError::ConfigError);
    Json(ServiceResponse::from_result(
        trace_id,
        result,
        service_error,
    ))
}
