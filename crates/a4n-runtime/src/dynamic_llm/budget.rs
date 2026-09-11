use a4n_domain::LlmMaxTokensConfig;
use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_provider_contract::LlmProviderConfig;

#[derive(Debug, Clone)]
pub(crate) struct EffectiveLlmPreviewBudget {
    pub(crate) model_name: String,
    pub(crate) max_context_tokens: usize,
    pub(crate) max_output_tokens: usize,
    pub(crate) output_token_reserve: usize,
    pub(crate) output_token_source: String,
}

pub(crate) fn resolve_effective_llm_preview_budget_for_config(
    model: LlmProviderConfig,
    request: &ChatRequest,
    max_tokens_config: Option<&LlmMaxTokensConfig>,
) -> Result<EffectiveLlmPreviewBudget, AppError> {
    let max_context_tokens = usize::try_from(model.max_context_tokens).map_err(|_| {
        AppError::ConfigError(format!(
            "LLM 配置 {} 的 max_context_tokens 必须大于 0",
            model.name
        ))
    })?;
    let max_output_tokens = usize::try_from(model.max_output_tokens).map_err(|_| {
        AppError::ConfigError(format!(
            "LLM 配置 {} 的 max_output_tokens 必须大于 0",
            model.name
        ))
    })?;
    if max_context_tokens == 0 || max_output_tokens == 0 || max_output_tokens >= max_context_tokens
    {
        return Err(AppError::ConfigError(format!(
            "LLM 配置 {} 的模型能力无效：max_context_tokens={}, max_output_tokens={}",
            model.name, max_context_tokens, max_output_tokens
        )));
    }

    let (output_token_reserve, output_token_source) = match request.max_tokens_profile.as_deref() {
        Some(profile) => {
            let config = max_tokens_config
                .ok_or_else(|| AppError::ConfigError("LLM max_tokens 配置未加载".to_string()))?;
            let configured = config.for_profile(profile).ok_or_else(|| {
                AppError::ConfigError(format!("未知的 LLM max_tokens 场景配置: {profile}"))
            })?;
            let resolved = request
                .max_tokens
                .map_or(configured, |minimum| configured.max(minimum));
            (resolved, format!("profile={profile}"))
        }
        None => match request.max_tokens {
            Some(max_tokens) => (max_tokens, "request.max_tokens".to_string()),
            None => (max_output_tokens, "model.max_output_tokens".to_string()),
        },
    };
    ensure_max_tokens_within_model_cap(
        Some(output_token_reserve),
        request.max_tokens_profile.as_deref(),
        max_output_tokens,
        model.model.as_str(),
    )?;

    Ok(EffectiveLlmPreviewBudget {
        model_name: model.model,
        max_context_tokens,
        max_output_tokens,
        output_token_reserve,
        output_token_source,
    })
}

pub(super) fn ensure_max_tokens_within_model_cap(
    max_tokens: Option<usize>,
    profile: Option<&str>,
    model_max_output_tokens: usize,
    model_name: &str,
) -> Result<(), AppError> {
    let Some(max_tokens) = max_tokens else {
        return Ok(());
    };
    if max_tokens <= model_max_output_tokens {
        return Ok(());
    }
    let source = profile
        .map(|profile| format!("profile={profile}"))
        .unwrap_or_else(|| "显式 max_tokens".to_string());
    Err(AppError::ConfigError(format!(
        "LLM 请求输出上限超过模型能力：model={}, {}, max_tokens={}, model_max_output_tokens={}",
        model_name, source, max_tokens, model_max_output_tokens
    )))
}
