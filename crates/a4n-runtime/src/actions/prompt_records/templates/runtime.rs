use super::super::{
    PromptBatchUpdate, PromptEntrySummary, PromptUpdateManyPayload,
    client::{
        prompt_client, prompt_service_request, prompt_sidecar_error, sidecar_get_prompt,
        sidecar_prompt_list,
    },
    projections::to_entry_summary,
};
use crate::AppState;
use a4n_prompt::engine::PromptEntry;

pub async fn prompt_get_runtime_entry(state: &AppState, name: &str) -> Result<PromptEntry, String> {
    match prompt_client(state)? {
        Some(_) => sidecar_get_prompt(state, name).await,
        None => state.engine.get(name).map_err(|error| error.to_string()),
    }
}

pub async fn prompt_update_many_runtime(
    state: &AppState,
    updates: Vec<(String, String)>,
    author: &str,
) -> Result<Vec<PromptEntry>, String> {
    if let Some(client) = prompt_client(state)? {
        let payload = PromptUpdateManyPayload {
            updates: updates
                .into_iter()
                .map(|(name, content)| PromptBatchUpdate { name, content })
                .collect(),
            author: author.to_string(),
        };
        let response = client
            .update_many(prompt_service_request(None, None, payload)?)
            .await
            .map_err(prompt_sidecar_error)?;
        return crate::sidecar_clients::response_payload(response, "prompt");
    }
    state
        .engine
        .update_many(updates, author)
        .await
        .map_err(|error| error.to_string())
}

pub async fn prompt_list(state: &AppState) -> Result<Vec<PromptEntrySummary>, String> {
    let entries = prompt_list_runtime_entries(state).await?;
    Ok(entries.iter().map(to_entry_summary).collect())
}

pub(crate) async fn prompt_list_runtime_entries(
    state: &AppState,
) -> Result<Vec<PromptEntry>, String> {
    match prompt_client(state)? {
        Some(_) => sidecar_prompt_list(state).await,
        None => Ok(state.engine.list_all()),
    }
}
