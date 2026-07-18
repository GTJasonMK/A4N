use std::collections::HashSet;

use crate::AppState;

use super::super::{
    PromptValidateOutput, PromptValidatePayload, PromptVersionDiffOutput, PromptVersionDiffPayload,
    ValidateVarsInput, VersionDiffInput,
    client::{prompt_client, prompt_service_request, prompt_sidecar_error},
};

pub async fn prompt_version_diff(
    state: &AppState,
    input: VersionDiffInput,
) -> Result<String, String> {
    if let Some(client) = prompt_client(state)? {
        let response = client
            .diff(prompt_service_request(
                None,
                None,
                PromptVersionDiffPayload {
                    name: input.name,
                    version_a: input.version_a,
                    version_b: input.version_b,
                },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        let output: PromptVersionDiffOutput =
            crate::sidecar_clients::response_payload(response, "prompt")?;
        return Ok(output.diff);
    }
    state
        .engine
        .diff(&input.name, &input.version_a, &input.version_b)
        .map_err(|err| err.to_string())
}

pub async fn prompt_validate_variables(
    state: &AppState,
    input: ValidateVarsInput,
) -> Result<Vec<String>, String> {
    if let Some(client) = prompt_client(state)? {
        let response = client
            .validate(prompt_service_request(
                None,
                None,
                PromptValidatePayload {
                    template_name: input.template_name,
                    variables: input.variables,
                },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        let output: PromptValidateOutput =
            crate::sidecar_clients::response_payload(response, "prompt")?;
        return Ok(output.missing);
    }
    let variables: HashSet<String> = input.variables.into_iter().collect();
    state
        .engine
        .validate_variables(&input.template_name, &variables)
        .map_err(|err| err.to_string())
}
