use super::super::{
    CountOutput, PromptEntrySummary, PromptNamePayload, UpdatePromptInput,
    client::{prompt_client, prompt_service_request, prompt_sidecar_error},
    projections::to_entry_summary,
    queries::active_prompt_names_for_state,
};
use crate::{AppState, actions::app_error_message};
use a4n_prompt::engine::PromptEntry;
use std::collections::HashSet;

pub async fn prompt_load_defaults(state: &AppState) -> Result<usize, String> {
    let existing_prompt_names = active_prompt_names_for_state(state).await?;
    state
        .orchestrator
        .preflight_project_override_keys_for_retired_cleanup()
        .await
        .map_err(app_error_message)?;
    let loaded = match prompt_client(state)? {
        Some(client) => {
            let response = client
                .load_defaults(prompt_service_request(None, None, serde_json::json!({}))?)
                .await
                .map_err(prompt_sidecar_error)?;
            let output: CountOutput = crate::sidecar_clients::response_payload(response, "prompt")?;
            output.count
        }
        None => state
            .engine
            .load_defaults()
            .await
            .map_err(|error| error.to_string())?,
    };
    let active_prompt_names = active_prompt_names_for_state(state).await?;
    let retired_prompt_names: HashSet<String> = existing_prompt_names
        .difference(&active_prompt_names)
        .cloned()
        .collect();
    if !retired_prompt_names.is_empty()
        && let Err(error) = state
            .orchestrator
            .delete_project_overrides_for_prompt_names(&retired_prompt_names)
            .await
    {
        return Err(format!(
            "提示词默认模板已重载，但退役提示词项目覆盖清理失败: {}",
            app_error_message(error)
        ));
    }
    Ok(loaded)
}

pub async fn prompt_update(
    state: &AppState,
    input: UpdatePromptInput,
) -> Result<PromptEntrySummary, String> {
    if let Some(client) = prompt_client(state)? {
        let response = client
            .update(prompt_service_request(None, None, input)?)
            .await
            .map_err(prompt_sidecar_error)?;
        let entry: PromptEntry = crate::sidecar_clients::response_payload(response, "prompt")?;
        return Ok(to_entry_summary(&entry));
    }
    let entry = state
        .engine
        .update(&input.name, input.content, &input.author)
        .await
        .map_err(|error| error.to_string())?;
    Ok(to_entry_summary(&entry))
}

pub async fn prompt_reset(state: &AppState, name: &str) -> Result<(), String> {
    if let Some(client) = prompt_client(state)? {
        let response = client
            .reset(prompt_service_request(
                None,
                None,
                PromptNamePayload {
                    name: name.to_string(),
                },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        let _: serde_json::Value = crate::sidecar_clients::response_payload(response, "prompt")?;
        return Ok(());
    }
    state
        .engine
        .reset(name)
        .await
        .map_err(|error| error.to_string())
}
