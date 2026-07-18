use a4n_core::error::AppError;
use a4n_model_discovery::{
    ModelDiscoveryRequest, ModelDiscoveryResult, discover_models_from_request,
};
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::Json;

use crate::protocol::{
    error_response, result_response, validate_provider_secret_policy, validate_service_request,
};

pub(crate) async fn discover_models(
    Json(request): Json<ServiceRequest<ModelDiscoveryRequest>>,
) -> Json<ServiceResponse<ModelDiscoveryResult>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    if let Err(error) =
        validate_provider_secret_policy(request.payload.api_key.as_deref(), &request.secret_policy)
    {
        return Json(error_response(&request, error));
    }
    let result = discover_models_from_request(request.payload.clone())
        .await
        .map_err(AppError::ConfigError);
    Json(result_response(&request, result))
}
