use crate::{
    AppState,
    actions::{app_error_message, config_records::shared::require_config_admin},
};
pub use a4n_core::config::app_config::LlmMaxTokensConfig;
use a4n_core::error::AppError;

pub async fn get_llm_max_tokens_config(state: &AppState) -> Result<LlmMaxTokensConfig, String> {
    state
        .orchestrator()
        .llm_max_tokens_config()
        .await
        .map_err(app_error_message)
}

pub async fn update_llm_max_tokens_config(
    state: &AppState,
    caller_user_id: i64,
    input: LlmMaxTokensConfig,
) -> Result<LlmMaxTokensConfig, AppError> {
    require_config_admin(state, caller_user_id).await?;
    state
        .orchestrator()
        .save_llm_max_tokens_config_validated(input.clone())
        .await?;

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
