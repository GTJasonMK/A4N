use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, ModelRequestPreviewStep,
    chat_request_step_for_state, image_provider_request_step, model_request_preview,
    provider_request_step,
};
use crate::state::AppState;
use a4n_image_contract::ImageGenerationRequest;
use a4n_llm::provider::ChatRequest;
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
    request: ImageGenerationRequest,
) -> Result<ModelRequestPreviewStep, String> {
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
        return provider_request_step(
            key.to_string(),
            title.to_string(),
            "image_provider_request",
            None,
            "ready",
            None,
            &provider_request,
        );
    }
    image_provider_request_step(
        key.to_string(),
        title.to_string(),
        None,
        "ready",
        None,
        &config.backend,
        request,
    )
}
