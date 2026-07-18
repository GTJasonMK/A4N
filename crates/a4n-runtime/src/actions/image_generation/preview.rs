use super::*;

pub(super) async fn image_generation_preview_step(
    state: &AppState,
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
        config.provider_options_json.as_ref(),
    )
}
