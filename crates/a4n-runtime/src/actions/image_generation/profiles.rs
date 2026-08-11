use super::{
    AppState, ModelRequestPreview, ObjectProfileImageInput, ObjectProfileImagePromptOutput,
    RuntimeImageGenerationHostPort, SceneProfileImageInput, SceneProfileImagePromptOutput,
    model_request_preview, preview::prepared_image_generation_preview_step,
};

pub async fn generate_object_profile_image_prompt(
    state: &AppState,
    user_id: i64,
    input: ObjectProfileImageInput,
) -> Result<ObjectProfileImagePromptOutput, String> {
    state
        .orchestrator()
        .generate_object_profile_image_prompt_validated(user_id, &input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn preview_object_profile_image_request(
    state: &AppState,
    user_id: i64,
    input: ObjectProfileImageInput,
) -> Result<ModelRequestPreview, String> {
    let host = RuntimeImageGenerationHostPort::new(state);
    let prepared = state
        .orchestrator()
        .prepare_object_profile_image_request_validated(&host, user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    let project_id = prepared.project_id;
    let title = format!(
        "{} 素材图请求预览",
        prepared.subject_name.as_deref().unwrap_or("物体")
    );
    Ok(model_request_preview(
        "object_profile_image_generation",
        title,
        vec![
            prepared_image_generation_preview_step(
                state,
                project_id,
                "object_profile_image",
                "物体素材图 provider 请求",
                prepared,
                true,
            )
            .await?,
        ],
    ))
}

pub async fn generate_scene_profile_image_prompt(
    state: &AppState,
    user_id: i64,
    input: SceneProfileImageInput,
) -> Result<SceneProfileImagePromptOutput, String> {
    state
        .orchestrator()
        .generate_scene_profile_image_prompt_validated(user_id, &input)
        .await
        .map_err(|error| error.to_string())
}
