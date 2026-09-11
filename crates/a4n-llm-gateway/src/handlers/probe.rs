use a4n_error::AppError;
use a4n_llm::gateway::{LlmProbeOutput, LlmProbePayload};
use a4n_llm::provider::{ChatRequest, LlmProvider, collect_chat_stream};
use a4n_service_protocol::{LLM_SERVICE_NAME, ServiceRequest, ServiceResponse};
use axum::Json;

use super::provider_from_config;
use crate::protocol::{error_response, service_error};

pub(crate) async fn probe(
    Json(request): Json<ServiceRequest<LlmProbePayload>>,
) -> Json<ServiceResponse<LlmProbeOutput>> {
    if let Err(error) = request.validate_known_provider_request(
        LLM_SERVICE_NAME,
        request.payload.provider.api_key.as_deref(),
    ) {
        return Json(error_response(&request, error));
    }
    let trace_id = request.trace_context.trace_id;
    let result = async {
        let LlmProbePayload { provider, request } = request.payload;
        let provider = provider_from_config(provider)?;
        probe_existing_provider(provider.as_ref(), request).await
    }
    .await;
    Json(ServiceResponse::from_result(
        trace_id,
        result,
        service_error,
    ))
}

pub(crate) async fn probe_existing_provider(
    provider: &dyn LlmProvider,
    request: ChatRequest,
) -> Result<LlmProbeOutput, AppError> {
    // 探测只收集最小真实 stream，不调用完整响应 chat()。
    let response = collect_chat_stream(provider, request).await?;
    LlmProbeOutput::from_chat_response(response)
}
