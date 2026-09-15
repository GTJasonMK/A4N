use super::super::contracts::LlmConfigSummary;
use crate::AppState;

pub async fn list_llm_config(state: &AppState) -> Result<Vec<LlmConfigSummary>, String> {
    state
        .config_read_application()
        .list_llm()
        .await
        .map_err(|error| error.to_string())
}
