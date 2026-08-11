use crate::{AppState, actions::app_error_message};
pub use a4n_core::config::app_config::LlmMaxTokensConfig;

pub async fn get_llm_max_tokens_config(state: &AppState) -> Result<LlmMaxTokensConfig, String> {
    state
        .orchestrator()
        .llm_max_tokens_config()
        .await
        .map_err(app_error_message)
}

pub async fn update_llm_max_tokens_config(
    state: &AppState,
    input: LlmMaxTokensConfig,
) -> Result<LlmMaxTokensConfig, String> {
    state
        .orchestrator()
        .save_llm_max_tokens_config_validated(input.clone())
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
            .orchestrator()
            .llm_max_tokens_config()
            .await
            .map_err(app_error_message),
    }
}
