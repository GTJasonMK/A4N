use a4n_core::error::AppError;
use a4n_llm::provider::{ChatResponse, LlmProvider};
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::Json;

use super::provider_from_config;
use crate::contracts::LlmChatPayload;
use crate::protocol::{
    error_response, result_response, validate_provider_secret_policy, validate_service_request,
};

pub(crate) async fn chat(
    Json(request): Json<ServiceRequest<LlmChatPayload>>,
) -> Json<ServiceResponse<ChatResponse>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    if let Err(error) = validate_provider_secret_policy(
        request.payload.provider.api_key.as_deref(),
        &request.secret_policy,
    ) {
        return Json(error_response(&request, error));
    }
    let result = chat_with_provider(request.payload.clone()).await;
    Json(result_response(&request, result))
}

async fn chat_with_provider(payload: LlmChatPayload) -> Result<ChatResponse, AppError> {
    provider_from_config(payload.provider)?
        .chat(payload.request)
        .await
}
