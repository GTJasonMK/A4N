use a4n_error::AppError;
use a4n_llm::provider::{ChatRequest, ChatResponse, LlmProvider, UsageInfo, collect_chat_stream};
use a4n_llm_sidecar_contract::LlmProbePayload;
use a4n_llm_sidecar_contract::{LlmProbeOutput, LlmProbeUsage};
use a4n_service_protocol::{LLM_SERVICE_NAME, ServiceRequest, ServiceResponse};
use axum::Json;

use super::provider_from_config;
use crate::protocol::{error_response, service_error};

pub(crate) async fn probe(
    Json(request): Json<ServiceRequest<LlmProbePayload<ChatRequest>>>,
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
    probe_output_from_chat_response(response)
}

fn probe_output_from_chat_response(response: ChatResponse) -> Result<LlmProbeOutput, AppError> {
    let content = response.message.content.trim();
    if content.is_empty() {
        return Err(AppError::LlmError("LLM 连接探测未收到有效响应".to_string()));
    }
    Ok(LlmProbeOutput {
        response_preview: content.chars().take(200).collect(),
        usage: response.usage.map(probe_usage),
        finish_reason: response.finish_reason,
    })
}

fn probe_usage(usage: UsageInfo) -> LlmProbeUsage {
    LlmProbeUsage {
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        total_tokens: usage.total_tokens,
        prompt_cache_hit_tokens: usage.prompt_cache_hit_tokens,
        prompt_cache_miss_tokens: usage.prompt_cache_miss_tokens,
        prompt_cache_write_tokens: usage.prompt_cache_write_tokens,
    }
}
