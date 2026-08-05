use super::*;
use crate::state::{CollaborationRuntimeFacade, RuntimeProfile};
use a4n_capabilities::A4nCapabilities;
use a4n_orchestrator::media::PreparedImageGenerationRequest;

pub(super) async fn prepared_image_generation_preview_step(
    state: &AppState,
    project_id: i64,
    key: impl Into<String>,
    title: impl Into<String>,
    prepared: PreparedImageGenerationRequest,
    include_image_type_note: bool,
) -> Result<ModelRequestPreviewStep, String> {
    let mut notes = prepared.preview_notes;
    if include_image_type_note {
        notes.insert(0, format!("image_type: {}", prepared.image_type));
    }
    image_generation_preview_step(
        state.runtime_profile(),
        state.capabilities(),
        &state.collaboration(),
        project_id,
        key,
        title,
        prepared.prompt_name,
        (!notes.is_empty()).then(|| notes.join("\n")),
        &prepared.config,
        prepared.request,
    )
    .await
}

pub(super) async fn image_generation_preview_step(
    runtime_profile: RuntimeProfile,
    capabilities: &A4nCapabilities,
    collaboration: &CollaborationRuntimeFacade<'_>,
    project_id: i64,
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: Option<String>,
    note: Option<String>,
    config: &image_gen_config::Model,
    request: ImageGenerationRequest,
) -> Result<ModelRequestPreviewStep, String> {
    let key = key.into();
    let title = title.into();
    if runtime_profile.uses_sidecars() {
        let root_project_id = collaboration
            .collaboration_root_project(project_id)
            .await
            .map(|root| Some(root.id))
            .map_err(crate::actions::app_error_message)?;
        let provider_request = crate::sidecar_clients::preview_image_provider_request(
            capabilities,
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
