use a4n_core::error::AppError;
use a4n_model_discovery::{
    ModelDiscoveryRequest, ModelDiscoveryResult, discover_models_from_request,
};
use a4n_service_protocol::{LLM_SERVICE_NAME, ServiceRequest, ServiceResponse};
use axum::Json;

use crate::protocol::{error_response, service_error};

pub(crate) async fn discover_models(
    Json(request): Json<ServiceRequest<ModelDiscoveryRequest>>,
) -> Json<ServiceResponse<ModelDiscoveryResult>> {
    if let Err(error) = request
        .validate_known_provider_request(LLM_SERVICE_NAME, request.payload.api_key.as_deref())
    {
        return Json(error_response(&request, error));
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
