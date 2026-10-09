use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, ModelRequestPreviewStep,
    chat_request_step_for_state, image_provider_request_step, model_request_preview,
    provider_request_step,
};
use crate::state::AppState;
use a4n_image_contract::ImageGenerationRequest;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_provider_contract::ImageProviderConfig;

pub(in crate::actions::portrait_generation) fn portrait_preview(
    workflow: impl Into<String>,
    title: impl Into<String>,
    steps: Vec<ModelRequestPreviewStep>,
) -> Result<ModelRequestPreview, String> {
    Ok(model_request_preview(workflow, title, steps))
}

pub(in crate::actions::portrait_generation) async fn chat_preview_step(
    state: &AppState,
    key: &str,
    title: &str,
    prompt_name: Option<&str>,
    request: ChatRequest,
) -> Result<ModelRequestPreviewStep, String> {
    chat_request_step_for_state(
        state,
        ChatRequestStepInput {
            key: key.to_string(),
            title: title.to_string(),
            prompt_name: prompt_name.map(str::to_string),
            prompt_assembly: None,
            state: "ready".to_string(),
            note: None,
            request,
        },
    )
    .await
}

pub(super) async fn provider_preview_step(
    state: &AppState,
    project_id: i64,
    key: &str,
    title: &str,
    config: &ImageProviderConfig,
    mut request: ImageGenerationRequest,
) -> Result<ModelRequestPreviewStep, String> {
    crate::actions::image_generation::assemble_image_generation_request(
        state,
        project_id,
        &mut request,
    )
    .await?;
    if state.runtime_profile().uses_sidecars() {
        let root_project_id = crate::actions::sidecar_root_project_id(state, project_id).await?;
        let provider_request = crate::sidecar_clients::preview_image_provider_request(
            state.capabilities().image.clone(),
            config,
            request,
            Some(project_id),
            root_project_id,
        )
        .await?;
        let mut step = provider_request_step(
            key.to_string(),
            title.to_string(),
            "image_provider_request",
            Some("image_generation_request".to_string()),
            "ready",
            None,
            &provider_request,
        )?;
        crate::actions::request_preview::attach_prompt_assembly_metadata(
            state,
            Some(project_id),
            "image_generation.provider",
            "image_generation_request",
            &mut step,
        )
        .await?;
        return Ok(step);
    }
    let mut step = image_provider_request_step(
        key.to_string(),
        title.to_string(),
        None,
        "ready",
        None,
        &config.backend,
        request,
    )?;
    step.prompt_name = Some("image_generation_request".to_string());
    crate::actions::request_preview::attach_prompt_assembly_metadata(
        state,
        Some(project_id),
        "image_generation.provider",
        "image_generation_request",
        &mut step,
    )
    .await?;
    Ok(step)
}
