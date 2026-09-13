use a4n_api_contract::image_generation::{
    ObjectProfileImageInput, ObjectProfileImagePromptOutput, SceneProfileImageInput,
    SceneProfileImagePromptOutput,
};

use super::{
    AppState, ModelRequestPreview, RuntimeImageGenerationHostPort, model_request_preview,
    preview::prepared_image_generation_preview_step,
};

pub async fn generate_object_profile_image_prompt(
    state: &AppState,
    user_id: i64,
    input: ObjectProfileImageInput,
) -> Result<ObjectProfileImagePromptOutput, String> {
    state
        .image_profile_prompt_application()
        .object_profile_prompt(user_id, &input)
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
        .image_generation_application()
        .prepare_object_profile_request(&host, user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    let project_id = prepared.project_id;
    let subject_name = prepared
        .subject_name
        .as_deref()
        .ok_or_else(|| "物体素材图请求缺少物体名称".to_string())?;
    let title = format!("{subject_name} 素材图请求预览");
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
        .image_profile_prompt_application()
        .scene_profile_prompt(user_id, &input)
        .await
        .map_err(|error| error.to_string())
}
