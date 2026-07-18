use crate::{
    AppState,
    actions::{access, template_variables},
};

use super::{
    AbRenderInput, AbRenderResult, PromptRenderAbOutput, PromptRenderOutput, PromptRenderPayload,
    RenderInput,
    client::{
        hydrate_prompt_sidecar_project_overrides, prompt_client, prompt_service_request,
        prompt_sidecar_error,
    },
};

pub async fn prompt_render(
    state: &AppState,
    user_id: i64,
    input: RenderInput,
) -> Result<String, String> {
    if let Some(project_id) = input.project_id {
        access::verify_project_editor(state, user_id, project_id)
            .await
            .map_err(|err| err.to_string())?;
    }
    let variables = template_variables::parse_template_variables(input.variables.as_ref())?;
    render_prompt_template(state, &input.template_name, variables, input.project_id).await
}

pub(crate) async fn render_prompt_template(
    state: &AppState,
    template_name: &str,
    variables: std::collections::HashMap<String, serde_json::Value>,
    project_id: Option<i64>,
) -> Result<String, String> {
    if let Some(client) = prompt_client(state)? {
        let root_project_id = match project_id {
            Some(project_id) => crate::actions::sidecar_root_project_id(state, project_id).await?,
            None => None,
        };
        hydrate_prompt_sidecar_project_overrides(state, project_id, root_project_id).await?;
        let response = client
            .render(prompt_service_request(
                project_id,
                root_project_id,
                PromptRenderPayload {
                    template_name: template_name.to_string(),
                    variables,
                    project_id,
                },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        let output: PromptRenderOutput =
            crate::sidecar_clients::response_payload(response, "prompt")?;
        return Ok(output.rendered);
    }
    state
        .engine
        .render(template_name, &variables, project_id)
        .map_err(|err| err.to_string())
}

pub async fn prompt_render_ab(
    state: &AppState,
    user_id: i64,
    input: AbRenderInput,
) -> Result<AbRenderResult, String> {
    if let Some(project_id) = input.project_id {
        access::verify_project_editor(state, user_id, project_id)
            .await
            .map_err(|err| err.to_string())?;
    }
    let variables = template_variables::parse_template_variables(input.variables.as_ref())?;
    if let Some(client) = prompt_client(state)? {
        let root_project_id = match input.project_id {
            Some(project_id) => crate::actions::sidecar_root_project_id(state, project_id).await?,
            None => None,
        };
        hydrate_prompt_sidecar_project_overrides(state, input.project_id, root_project_id).await?;
        let response = client
            .render_ab(prompt_service_request(
                input.project_id,
                root_project_id,
                PromptRenderPayload {
                    template_name: input.template_name,
                    variables,
                    project_id: input.project_id,
                },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        let output: PromptRenderAbOutput =
            crate::sidecar_clients::response_payload(response, "prompt")?;
        return Ok(AbRenderResult {
            version_a: output.version_a,
            version_b: output.version_b,
        });
    }
    let (version_a, version_b) = state
        .engine
        .render_ab(&input.template_name, &variables, input.project_id)
        .map_err(|err| err.to_string())?;
    Ok(AbRenderResult {
        version_a,
        version_b,
    })
}
