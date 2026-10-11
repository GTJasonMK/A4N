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

pub(crate) struct ImageGenerationPreviewInput<'a> {
    pub(crate) runtime_profile: RuntimeProfile,
    pub(crate) client: Option<Arc<dyn ImageClient>>,
    pub(crate) collaboration: &'a dyn a4n_api_contract::collaboration::ProjectAccessApplicationPort,
    pub(crate) project_id: i64,
    pub(crate) key: String,
    pub(crate) title: String,
    pub(crate) prompt_name: Option<String>,
    pub(crate) note: Option<String>,
    pub(crate) config: &'a ImageProviderConfig,
    pub(crate) request: ImageGenerationRequest,
}

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
    let mut step = image_generation_preview_step(ImageGenerationPreviewInput {
        runtime_profile: state.runtime_profile(),
        client: state.capabilities().image.clone(),
        collaboration: state.project_access_application(),
        project_id,
        key: key.into(),
        title: title.into(),
        prompt_name: prepared.prompt_name,
        note: (!notes.is_empty()).then(|| notes.join("\n")),
        config: &prepared.config,
        request: prepared.request,
    })
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
    input: ImageGenerationPreviewInput<'_>,
) -> Result<ModelRequestPreviewStep, String> {
    let ImageGenerationPreviewInput {
        runtime_profile,
        client,
        collaboration,
        project_id,
        key,
        title,
        prompt_name,
        note,
        config,
        request,
    } = input;
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
