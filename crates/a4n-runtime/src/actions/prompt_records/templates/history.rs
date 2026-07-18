use super::super::{
    PromptNamePayload, PromptVersionHistoryOutput, PromptVersionSummary,
    client::{prompt_client, prompt_service_request, prompt_sidecar_error},
    projections::to_version_summary,
};
use crate::AppState;

pub async fn prompt_version_history(
    state: &AppState,
    name: &str,
) -> Result<Vec<PromptVersionSummary>, String> {
    if let Some(client) = prompt_client(state)? {
        let response = client
            .version_history(prompt_service_request(
                None,
                None,
                PromptNamePayload {
                    name: name.to_string(),
                },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        let output: PromptVersionHistoryOutput =
            crate::sidecar_clients::response_payload(response, "prompt")?;
        return Ok(output.versions.iter().map(to_version_summary).collect());
    }
    let versions = state
        .engine
        .version_history(name)
        .map_err(|error| error.to_string())?;
    Ok(versions.iter().map(to_version_summary).collect())
}
