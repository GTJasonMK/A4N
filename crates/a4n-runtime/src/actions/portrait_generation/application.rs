use crate::{AppState, actions::access, prompt_renderer::RuntimePromptRenderer};
use a4n_orchestrator::portrait::{
    GeneratePortraitInput, GeneratePortraitPromptInput, PortraitSummary, UploadPortraitInput,
};
use tokio_util::sync::CancellationToken;

use super::host_port::RuntimePortraitGenerationHostPort;

pub async fn generate_portrait_prompt(
    state: &AppState,
    user_id: i64,
    input: GeneratePortraitPromptInput,
) -> Result<String, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let variables =
        crate::actions::template_variables::parse_template_variables(input.variables.as_ref())?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    state
        .character()
        .generate_portrait_prompt_validated(input, &prompt_renderer, &variables)
        .await
        .map_err(|error| error.to_string())
}

pub(crate) async fn generate_portrait_inline(
    state: &AppState,
    user_id: i64,
    input: GeneratePortraitInput,
) -> Result<PortraitSummary, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let host_port = RuntimePortraitGenerationHostPort::new(state);
    state
        .character()
        .generate_portrait_validated(
            input,
            &prompt_renderer,
            &host_port,
            None,
            CancellationToken::new(),
        )
        .await
        .map_err(|error| error.to_string())
}

pub async fn upload_portrait(
    state: &AppState,
    user_id: i64,
    input: UploadPortraitInput,
) -> Result<PortraitSummary, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let host_port = RuntimePortraitGenerationHostPort::new(state);
    state
        .character()
        .upload_portrait_validated(input, &host_port)
        .await
        .map_err(|error| error.to_string())
}
