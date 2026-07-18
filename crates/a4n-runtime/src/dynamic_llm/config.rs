use super::*;

pub(super) fn validated_timeout_secs(m: &llm_config::Model) -> Result<u64, AppError> {
    let timeout_secs = u64::try_from(m.timeout_secs).map_err(|_| {
        AppError::ConfigError(format!("LLM 配置 {} 的 timeout_secs 必须大于 0", m.name))
    })?;
    if timeout_secs == 0 {
        return Err(AppError::ConfigError(format!(
            "LLM 配置 {} 的 timeout_secs 必须大于 0",
            m.name
        )));
    }
    Ok(timeout_secs)
}

pub(super) fn model_capabilities(m: &llm_config::Model) -> Result<ModelCapabilities, AppError> {
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
        function_calling: true,
        vision: true,
        max_context_tokens,
        max_output_tokens,
    })
}

#[cfg(feature = "embedded-llm-provider")]
pub(super) fn make_provider(m: &llm_config::Model) -> Result<HttpLlmProvider, AppError> {
    let timeout_secs = validated_timeout_secs(m)?;
    let capabilities = model_capabilities(m)?;
    HttpLlmProvider::new_with_timeout_options_and_capabilities(
        m.base_url.clone(),
        m.model.clone(),
        m.api_key_encrypted.clone(),
        timeout_secs,
        m.request_options_json.clone(),
        capabilities,
    )
}

pub(super) async fn chat_via_sidecar(
    client: Arc<dyn LlmClient>,
    resolved_model: ResolvedLlmModel,
    request: ChatRequest,
    trace_context: &LlmTraceContext,
) -> Result<ChatResponse, AppError> {
    let payload = resolved_model.sidecar_payload(request);
    let sidecar_context = crate::sidecar_clients::sidecar_context_for_llm_trace(trace_context)
        .map_err(|err| AppError::ConfigError(format!("LLM sidecar 请求构造失败：{err}")))?;
    let request = crate::sidecar_clients::service_request_with_context_provider_secret_policy(
        sidecar_context,
        "llm",
        a4n_service_protocol::LLM_CHAT_CAPABILITY_VERSION,
        payload,
    )
    .map_err(|err| AppError::ConfigError(format!("LLM sidecar 请求构造失败：{err}")))?;
    let response = client.chat(request).await.map_err(llm_sidecar_error)?;
    crate::sidecar_clients::response_payload(response, "llm")
        .map_err(|err| AppError::LlmError(format!("LLM sidecar 响应失败：{err}")))
}

pub(super) fn require_sidecar_business_trace<'a>(
    trace_context: Option<&'a LlmTraceContext>,
    operation: &str,
) -> Result<&'a LlmTraceContext, AppError> {
    trace_context.ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "{operation} 缺少 trace_context；业务 LLM sidecar 请求必须显式携带 project/root project 和 workflow/generation trace"
        ))
    })
}

pub(super) async fn probe_llm_via_sidecar(
    client: Arc<dyn LlmClient>,
    resolved_model: ResolvedLlmModel,
    request: ChatRequest,
) -> Result<LlmProbeOutput, AppError> {
    let payload = resolved_model.probe_payload(request);
    let request = crate::sidecar_clients::global_provider_service_request(
        "llm",
        a4n_service_protocol::LLM_CHAT_CAPABILITY_VERSION,
        payload,
    )
    .map_err(|err| AppError::ConfigError(format!("LLM sidecar probe 请求构造失败：{err}")))?;
    let response = client.probe(request).await.map_err(llm_sidecar_error)?;
    crate::sidecar_clients::response_payload(response, "llm")
        .map_err(|err| AppError::LlmError(format!("LLM sidecar probe 响应失败：{err}")))
}

#[cfg(feature = "embedded-llm-provider")]
pub(super) async fn chat_embedded(
    resolved_model: ResolvedLlmModel,
    request: ChatRequest,
) -> Result<ChatResponse, AppError> {
    let provider = make_provider(&resolved_model.model)?;
    provider.chat(request).await
}

#[cfg(not(feature = "embedded-llm-provider"))]
pub(super) async fn chat_embedded(
    _resolved_model: ResolvedLlmModel,
    _request: ChatRequest,
) -> Result<ChatResponse, AppError> {
    Err(AppError::ConfigError(
        "当前 runtime 未编译 embedded LLM provider；请使用 A4N_RUNTIME_PROFILE=sidecar 并启动 a4n-llm-gateway，或以 --features embedded-capabilities 显式构建 full profile".to_string(),
    ))
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

pub(super) fn estimate_llm_tokens(text: &str) -> usize {
    let chinese_chars = text.chars().filter(|c| u32::from(*c) > 0x2E80).count();
    let other_words = if chinese_chars == text.len() {
        0
    } else {
        text.split_whitespace().count()
    };
    ((chinese_chars as f32 * 0.5) + (other_words as f32 * 1.3)).ceil() as usize
}
