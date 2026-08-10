use a4n_core::error::AppError;
use a4n_llm::gateway::{LlmGatewayPayload, LlmProbeOutput};
use a4n_llm::provider::{ChatRequest, LlmProvider, collect_chat_stream};
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::Json;

use super::provider_from_config;
use crate::protocol::{
    error_response, result_response, validate_provider_secret_policy, validate_service_request,
};

pub(crate) async fn probe(
    Json(request): Json<ServiceRequest<LlmGatewayPayload>>,
) -> Json<ServiceResponse<LlmProbeOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    if let Err(error) = validate_provider_secret_policy(
        request.payload.provider.api_key.as_deref(),
        &request.secret_policy,
    ) {
        return Json(error_response(&request, error));
    }
    let result = probe_with_provider(request.payload.clone()).await;
    Json(result_response(&request, result))
}

async fn probe_with_provider(payload: LlmGatewayPayload) -> Result<LlmProbeOutput, AppError> {
    let provider = provider_from_config(payload.provider)?;
    probe_existing_provider(provider.as_ref(), payload.request).await
}

pub(crate) async fn probe_existing_provider(
    provider: &dyn LlmProvider,
    request: ChatRequest,
) -> Result<LlmProbeOutput, AppError> {
    // 探测只收集最小真实 stream，不调用完整响应 chat()。
    let response = collect_chat_stream(provider, request).await?;
    LlmProbeOutput::from_chat_response(response)
}
