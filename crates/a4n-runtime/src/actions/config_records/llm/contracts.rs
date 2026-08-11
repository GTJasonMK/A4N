pub use a4n_api_contract::config::{CreateLlmInput, LlmConfigSummary, UpdateLlmInput};
use a4n_core::models::ai::llm_config;

pub(super) fn llm_config_summary(config: &llm_config::Model) -> LlmConfigSummary {
    LlmConfigSummary {
        id: config.id,
        name: config.name.clone(),
        provider: config.provider.clone(),
        base_url: config.base_url.clone(),
        has_api_key: config
            .api_key_encrypted
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty()),
        model: config.model.clone(),
        max_concurrent: config.max_concurrent,
        timeout_secs: config.timeout_secs,
        max_context_tokens: config.max_context_tokens,
        max_output_tokens: config.max_output_tokens,
        extra_body_json: config.extra_body_json.clone(),
        is_default: config.is_default,
    }
}

pub(super) fn positive_model_output_cap(value: i32) -> Result<usize, String> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "模型最大输出必须大于 0".to_string())
}

pub(super) fn positive_model_context_cap(value: i32) -> Result<usize, String> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "模型上下文窗口必须大于 0".to_string())
}
