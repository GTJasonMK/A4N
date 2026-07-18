use super::positive_model_output_cap;
use crate::{AppState, actions::app_error_message};
use a4n_core::config::app_config::LlmMaxTokensConfig;

pub async fn get_llm_max_tokens_config(state: &AppState) -> Result<LlmMaxTokensConfig, String> {
    state
        .orchestrator
        .llm_max_tokens_config()
        .await
        .map_err(app_error_message)
}

pub async fn update_llm_max_tokens_config(
    state: &AppState,
    input: LlmMaxTokensConfig,
) -> Result<LlmMaxTokensConfig, String> {
    input.validate()?;
    for (label, model_cap) in persisted_llm_default_model_output_caps(state).await? {
        input
            .validate_against_model_output_cap(model_cap)
            .map_err(|err| format!("{label}：{err}"))?;
    }
    state
        .orchestrator
        .save_llm_max_tokens_config(&input)
        .await
        .map_err(app_error_message)?;

    Ok(input)
}

pub(super) async fn load_effective_llm_max_tokens_config(
    state: &AppState,
) -> Result<LlmMaxTokensConfig, String> {
    match crate::runtime_config::current_llm_max_tokens() {
        Some(config) => {
            config.validate()?;
            Ok(config)
        }
        None => state
            .orchestrator
            .llm_max_tokens_config()
            .await
            .map_err(app_error_message),
    }
}

pub(super) async fn load_persisted_llm_max_tokens_config(
    state: &AppState,
) -> Result<LlmMaxTokensConfig, String> {
    state
        .orchestrator
        .llm_max_tokens_config()
        .await
        .map_err(app_error_message)
}

async fn persisted_llm_default_model_output_caps(
    state: &AppState,
) -> Result<Vec<(String, usize)>, String> {
    let mut caps = Vec::new();
    for model in state
        .orchestrator
        .default_llm_output_cap_rows()
        .await
        .map_err(app_error_message)?
    {
        caps.push((
            format!("默认 LLM 配置 #{} {}", model.id, model.name),
            positive_model_output_cap(model.max_output_tokens)?,
        ));
    }
    Ok(caps)
}
