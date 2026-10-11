use std::sync::Arc;

use a4n_error::AppError;
use a4n_llm_client::{LlmClient, LlmGatewayRequest};
#[cfg(feature = "embedded-llm-provider")]
use a4n_llm_provider_contract::provider::LlmProvider;
use a4n_llm_provider_contract::provider::{ChatRequest, LlmTraceContext, ModelCapabilities};
use a4n_provider_contract::LlmProviderConfig;

use super::contracts::{LlmProbeOutput, ResolvedLlmModel};

pub(super) fn validated_timeout_secs(m: &LlmProviderConfig) -> Result<u64, AppError> {
    if m.timeout_secs <= 0 {
        return Err(AppError::ConfigError(format!(
            "LLM 配置 {} 的 timeout_secs 必须大于 0",
            m.name
        )));
    }
    Ok(m.timeout_secs as u64)
}

pub(super) fn model_capabilities(m: &LlmProviderConfig) -> Result<ModelCapabilities, AppError> {
    let max_context_tokens = usize::try_from(m.max_context_tokens).map_err(|_| {
        AppError::ConfigError(format!(
            "LLM 配置 {} 的 max_context_tokens 必须大于 0",
            m.name
        ))
    })?;
    let max_output_tokens = usize::try_from(m.max_output_tokens).map_err(|_| {
        AppError::ConfigError(format!(
            "LLM 配置 {} 的 max_output_tokens 必须大于 0",
            m.name
        ))
    })?;
    if max_context_tokens == 0 || max_output_tokens == 0 || max_output_tokens >= max_context_tokens
    {
        return Err(AppError::ConfigError(format!(
            "LLM 配置 {} 的模型能力无效：max_context_tokens={}, max_output_tokens={}",
            m.name, max_context_tokens, max_output_tokens
        )));
    }

    Ok(ModelCapabilities {
        max_context_tokens,
        max_output_tokens,
    })
}

#[cfg(feature = "embedded-llm-provider")]
pub(super) fn make_provider(m: &LlmProviderConfig) -> Result<Box<dyn LlmProvider>, AppError> {
    let timeout_secs = validated_timeout_secs(m)?;
    let capabilities = model_capabilities(m)?;
    a4n_llm_http::pick_llm_provider(
        &m.provider,
        m.base_url.clone(),
        m.model.clone(),
        m.api_key.clone(),
        timeout_secs,
        m.extra_body_json.clone(),
        capabilities,
    )
}

pub(super) fn sidecar_chat_request(
    resolved_model: ResolvedLlmModel,
    mut request: ChatRequest,
    trace_context: &LlmTraceContext,
    operation: &str,
) -> Result<LlmGatewayRequest, AppError> {
    request.stream_include_usage = true;
    let payload = resolved_model.sidecar_payload(request);
    let sidecar_context = crate::sidecar_clients::sidecar_context_for_llm_trace(trace_context)
        .map_err(|err| AppError::ConfigError(format!("{operation} 请求构造失败：{err}")))?;
    crate::sidecar_clients::typed_service_request_with_context_provider_secret_policy(
        sidecar_context,
        a4n_service_protocol::LLM_CHAT_CAPABILITY_VERSION,
        payload,
    )
    .map_err(|err| AppError::ConfigError(format!("{operation} 请求构造失败：{err}")))
}

pub(super) fn require_sidecar_business_trace<'a>(
    trace_context: Option<&'a LlmTraceContext>,
    operation: &str,
) -> Result<&'a LlmTraceContext, AppError> {
    let trace_context = trace_context.ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "{operation} 缺少 trace_context；业务 LLM sidecar 请求必须显式携带 project/root project 和 workflow/generation trace"
        ))
    })?;
    trace_context.validate_business_trace().map_err(|error| {
        AppError::InvalidArgument(format!("{operation} trace_context 无效：{error}"))
    })?;
    Ok(trace_context)
}

pub(super) async fn probe_llm_via_sidecar(
    client: Arc<dyn LlmClient>,
    resolved_model: ResolvedLlmModel,
    request: ChatRequest,
) -> Result<LlmProbeOutput, AppError> {
    let payload = resolved_model.probe_payload(request);
    let request = crate::sidecar_clients::global_typed_provider_service_request(
        a4n_service_protocol::LLM_CHAT_CAPABILITY_VERSION,
        payload,
    )
    .map_err(|err| AppError::ConfigError(format!("LLM sidecar probe 请求构造失败：{err}")))?;
    let response = client.probe(request).await.map_err(llm_sidecar_error)?;
    crate::sidecar_clients::typed_response_payload(response, "llm")
        .map_err(|err| AppError::LlmError(format!("LLM sidecar probe 响应失败：{err}")))
}

pub(super) fn llm_sidecar_error(error: a4n_service_protocol::ServiceError) -> AppError {
    match error.code {
        a4n_service_protocol::ServiceErrorCode::ConfigurationError => {
            AppError::ConfigError(error.message)
        }
        a4n_service_protocol::ServiceErrorCode::Cancelled => AppError::Cancelled,
        a4n_service_protocol::ServiceErrorCode::ServiceUnavailable
        | a4n_service_protocol::ServiceErrorCode::VersionMismatch => {
            AppError::Timeout(format!("LLM sidecar 不可用：{}", error.message))
        }
        _ => AppError::LlmError(format!("LLM sidecar 调用失败：{}", error.message)),
    }
}
