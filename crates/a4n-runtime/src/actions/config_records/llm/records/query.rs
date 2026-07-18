use super::super::contracts::{LlmConfigSummary, llm_config_summary};
use crate::{AppState, actions::app_error_message};

pub async fn list_llm_config(state: &AppState) -> Result<Vec<LlmConfigSummary>, String> {
    let configs = state
        .orchestrator
        .list_llm_config_records()
        .await
        .map_err(app_error_message)?;
    Ok(configs.iter().map(llm_config_summary).collect())
}
