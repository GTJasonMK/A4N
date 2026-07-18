use a4n_prompt::engine::PromptEntry;

use super::{prompt_client_required, prompt_service_request, prompt_sidecar_error};
use crate::AppState;

use super::super::{PromptListOutput, PromptNamePayload, PromptProjectOverridePayload};

pub(in crate::actions::prompt_records) async fn sidecar_prompt_list(
    state: &AppState,
) -> Result<Vec<PromptEntry>, String> {
    let response = prompt_client_required(state)?
        .list(prompt_service_request(None, None, serde_json::json!({}))?)
        .await
        .map_err(prompt_sidecar_error)?;
    let output: PromptListOutput = crate::sidecar_clients::response_payload(response, "prompt")?;
    Ok(output.prompts)
}

pub(in crate::actions::prompt_records) async fn sidecar_get_prompt(
    state: &AppState,
    name: &str,
) -> Result<PromptEntry, String> {
    let response = prompt_client_required(state)?
        .get(prompt_service_request(
            None,
            None,
            PromptNamePayload {
                name: name.to_string(),
            },
        )?)
        .await
        .map_err(prompt_sidecar_error)?;
    crate::sidecar_clients::response_payload(response, "prompt")
}

pub(in crate::actions::prompt_records) async fn sidecar_set_project_override(
    state: &AppState,
    project_id: i64,
    name: &str,
    content: String,
) -> Result<(), String> {
    let root_project_id = crate::actions::sidecar_root_project_id(state, project_id).await?;
    let response = prompt_client_required(state)?
        .set_project_override(prompt_service_request(
            Some(project_id),
            root_project_id,
            PromptProjectOverridePayload {
                project_id,
                name: name.to_string(),
                content: Some(content),
            },
        )?)
        .await
        .map_err(prompt_sidecar_error)?;
    let _: serde_json::Value = crate::sidecar_clients::response_payload(response, "prompt")?;
    Ok(())
}

pub(in crate::actions::prompt_records) async fn sidecar_remove_project_override(
    state: &AppState,
    project_id: i64,
    name: &str,
) -> Result<(), String> {
    let root_project_id = crate::actions::sidecar_root_project_id(state, project_id).await?;
    let response = prompt_client_required(state)?
        .remove_project_override(prompt_service_request(
            Some(project_id),
            root_project_id,
            PromptProjectOverridePayload {
                project_id,
                name: name.to_string(),
                content: None,
            },
        )?)
        .await
        .map_err(prompt_sidecar_error)?;
    let _: serde_json::Value = crate::sidecar_clients::response_payload(response, "prompt")?;
    Ok(())
}
