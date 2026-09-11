use crate::{AppState, actions::app_error_message};
pub use a4n_domain::LlmMaxTokensConfig;
use a4n_error::AppError;

pub async fn get_llm_max_tokens_config(state: &AppState) -> Result<LlmMaxTokensConfig, String> {
    state
        .config_application()
        .llm_max_tokens()
        .await
        .map_err(app_error_message)
}

pub async fn update_llm_max_tokens_config(
    state: &AppState,
    caller_user_id: i64,
    input: LlmMaxTokensConfig,
) -> Result<LlmMaxTokensConfig, AppError> {
    state
        .config_application()
        .update_llm_max_tokens(caller_user_id, input)
        .await
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
            .config_application()
            .llm_max_tokens()
            .await
            .map_err(app_error_message),
    }
}
