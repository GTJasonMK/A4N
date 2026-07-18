use super::{prompt_client_required, prompt_service_request, prompt_sidecar_error};
use crate::AppState;

use super::super::{CountOutput, PromptHydrateOverride, PromptHydrateOverridesPayload};

pub(in crate::actions::prompt_records) async fn hydrate_prompt_sidecar_project_overrides(
    state: &AppState,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<(), String> {
    let Some(project_id) = project_id else {
        return Ok(());
    };
    let client = prompt_client_required(state)?;
    let active_prompt_names = super::super::queries::active_prompt_names_for_state(state).await?;
    let overrides = state
        .orchestrator
        .project_override_summaries(project_id, &active_prompt_names)
        .await
        .map_err(crate::actions::app_error_message)?
        .into_iter()
        .map(|item| {
            let content = item.content.as_str().ok_or_else(|| {
                format!(
                    "提示词覆盖记录 value 必须是字符串: project_id={}, prompt={}",
                    item.project_id, item.prompt_name
                )
            })?;
            Ok(PromptHydrateOverride {
                project_id: item.project_id,
                name: item.prompt_name,
                content: content.to_string(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let response = client
        .hydrate_project_overrides(prompt_service_request(
            Some(project_id),
            root_project_id,
            PromptHydrateOverridesPayload { overrides },
        )?)
        .await
        .map_err(prompt_sidecar_error)?;
    let _: CountOutput = crate::sidecar_clients::response_payload(response, "prompt")?;
    Ok(())
}
