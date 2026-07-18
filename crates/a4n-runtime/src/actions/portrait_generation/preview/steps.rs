use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, ModelRequestPreviewStep, blocked_step,
    chat_request_step_for_state, image_provider_request_step, model_request_preview,
    provider_request_step,
};
use crate::state::AppState;
use a4n_core::models::ai::image_gen_config;
use a4n_image::ImageGenerationRequest;
use a4n_llm::provider::ChatRequest;

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
    config: &image_gen_config::Model,
    request: ImageGenerationRequest,
) -> Result<ModelRequestPreviewStep, String> {
    if state.runtime_profile.uses_sidecars() {
        let root_project_id = crate::actions::sidecar_root_project_id(state, project_id).await?;
        let provider_request = crate::sidecar_clients::preview_image_provider_request(
            state.capabilities(),
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
        None,
    )
}

pub(super) fn blocked_preview_step(
    key: &str,
    title: &str,
    kind: &str,
    note: &str,
) -> ModelRequestPreviewStep {
    blocked_step(
        key.to_string(),
        title.to_string(),
        kind.to_string(),
        None,
        note.to_string(),
    )
}
