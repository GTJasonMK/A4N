use std::sync::Arc;

use crate::state::RuntimeProfile;
use crate::{
    AppState,
    actions::request_preview::{
        ModelRequestPreviewStep, image_provider_request_step, provider_request_step,
    },
};
use serde_json::json;

pub(crate) async fn assemble_image_generation_request(
    state: &AppState,
    project_id: i64,
    request: &mut ImageGenerationRequest,
) -> Result<(), String> {
    request.prompt = a4n_api_contract::prompt::PromptRenderPort::render_prompt_for_call(
        state,
        "image_generation.provider",
        "image_generation_request",
        &std::collections::HashMap::from([("prompt".to_string(), json!(request.prompt))]),
        Some(project_id),
    )
    .await
    .map_err(crate::actions::app_error_message)?;
    Ok(())
}
use a4n_image_client::ImageClient;
use a4n_image_contract::ImageGenerationRequest;
use a4n_image_host_contract::PreparedImageGenerationRequest;
use a4n_provider_contract::ImageProviderConfig;

pub(super) async fn prepared_image_generation_preview_step(
    state: &AppState,
    project_id: i64,
    key: impl Into<String>,
    title: impl Into<String>,
    mut prepared: PreparedImageGenerationRequest,
    include_image_type_note: bool,
) -> Result<ModelRequestPreviewStep, String> {
    assemble_image_generation_request(state, project_id, &mut prepared.request).await?;
    let mut notes = prepared.preview_notes;
    if include_image_type_note {
        notes.insert(0, format!("image_type: {}", prepared.image_type));
    }
    let mut step = image_generation_preview_step(
        state.runtime_profile(),
        state.capabilities().image.clone(),
        state.project_access_application(),
        project_id,
        key,
        title,
        prepared.prompt_name,
        (!notes.is_empty()).then(|| notes.join("\n")),
        &prepared.config,
        prepared.request,
    )
    .await?;
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

pub(crate) async fn image_generation_preview_step(
    runtime_profile: RuntimeProfile,
    client: Option<Arc<dyn ImageClient>>,
    collaboration: &dyn a4n_api_contract::collaboration::ProjectAccessApplicationPort,
    project_id: i64,
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: Option<String>,
    note: Option<String>,
    config: &ImageProviderConfig,
    request: ImageGenerationRequest,
) -> Result<ModelRequestPreviewStep, String> {
    let key = key.into();
    let title = title.into();
    if runtime_profile.uses_sidecars() {
        let root_project_id = collaboration
            .collaboration_root(project_id)
            .await
            .map(|root| Some(root.id))
            .map_err(crate::actions::app_error_message)?;
        let provider_request = crate::sidecar_clients::preview_image_provider_request(
            client,
            config,
            request,
            Some(project_id),
            root_project_id,
        )
        .await?;
        return provider_request_step(
            key,
            title,
            "image_provider_request",
            prompt_name,
            "ready",
            note,
            &provider_request,
        );
    }
    image_provider_request_step(
        key,
        title,
        prompt_name,
        "ready",
        note,
        &config.backend,
        request,
    )
}
