use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_provider_contract::LlmProviderConfig;

use super::config::model_capabilities;

#[derive(Debug, Clone)]
pub(crate) struct EffectiveLlmPreviewBudget {
    pub(crate) model_name: String,
    pub(crate) max_context_tokens: usize,
    pub(crate) max_output_tokens: usize,
    pub(crate) output_token_reserve: usize,
    pub(crate) output_token_source: &'static str,
}

pub(crate) fn resolve_effective_llm_preview_budget_for_config(
    model: LlmProviderConfig,
    request: &ChatRequest,
) -> Result<EffectiveLlmPreviewBudget, AppError> {
    let capabilities = model_capabilities(&model)?;
    let output_token_source = if request.max_tokens.is_some() {
        "request.max_tokens"
    } else {
        "model.max_output_tokens"
    };
    let output_token_reserve = requested_max_tokens(
        request.max_tokens,
        capabilities.max_output_tokens,
        model.model.as_str(),
    )?;

    Ok(EffectiveLlmPreviewBudget {
        model_name: model.model,
        max_context_tokens: capabilities.max_context_tokens,
        max_output_tokens: capabilities.max_output_tokens,
        output_token_reserve,
        output_token_source,
    })
}

pub(crate) fn requested_max_tokens(
    requested: Option<usize>,
    model_max_output_tokens: usize,
    model_name: &str,
) -> Result<usize, AppError> {
    let max_tokens = requested.unwrap_or(model_max_output_tokens);
    if max_tokens == 0 {
        return Err(AppError::InvalidArgument(
            "LLM 请求 max_tokens 必须大于 0".to_string(),
        ));
    }
    if max_tokens > model_max_output_tokens {
        return Err(AppError::ConfigError(format!(
            "LLM 请求输出上限超过模型能力：model={}, max_tokens={}, model_max_output_tokens={}",
            model_name, max_tokens, model_max_output_tokens
        )));
    }
    Ok(max_tokens)
}
