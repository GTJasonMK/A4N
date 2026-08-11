use a4n_orchestrator::media::{
    GenerateCanvasImageInput, GenerateGalgameImageInput, SceneImageGenerationInput,
};

use super::{
    AppState, DEFAULT_IMAGE_GENERATION_PROGRESS, GeneratedImageSummary, ImageGenerationProgress,
    ModelRequestPreview, RuntimeImageGenerationHostPort, model_request_preview,
    preview::prepared_image_generation_preview_step,
};

pub async fn preview_scene_image_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    prompt: String,
    config_id: Option<i64>,
    chapter_id: Option<i64>,
) -> Result<ModelRequestPreview, String> {
    let host = RuntimeImageGenerationHostPort::new(state);
    let prepared = state
        .orchestrator()
        .prepare_scene_image_request_validated(
            &host,
            user_id,
            SceneImageGenerationInput {
                project_id,
                prompt,
                config_id,
                chapter_id,
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(model_request_preview(
        "image_scene_generation",
        "场景图模型请求预览",
        vec![
            prepared_image_generation_preview_step(
                state,
                project_id,
                "scene_image",
                "场景图 provider 请求",
                prepared,
                false,
            )
            .await?,
        ],
    ))
}

pub async fn preview_canvas_image_request(
    state: &AppState,
    user_id: i64,
    input: GenerateCanvasImageInput,
) -> Result<ModelRequestPreview, String> {
    let project_id = input.project_id;
    let host = RuntimeImageGenerationHostPort::new(state);
    let prepared = state
        .orchestrator()
        .prepare_canvas_image_request_validated(&host, user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(model_request_preview(
        "canvas_image_generation",
        "画布图片模型请求预览",
        vec![
            prepared_image_generation_preview_step(
                state,
                project_id,
                "canvas_image",
                "画布图片 provider 请求",
                prepared,
                true,
            )
            .await?,
        ],
    ))
}

pub(crate) async fn generate_galgame_image(
    state: &AppState,
    user_id: i64,
    input: GenerateGalgameImageInput,
    progress: Option<&ImageGenerationProgress<'_>>,
) -> Result<GeneratedImageSummary, String> {
    let progress = progress.unwrap_or(&DEFAULT_IMAGE_GENERATION_PROGRESS);
    let host = RuntimeImageGenerationHostPort::new(state);
    state
        .orchestrator()
        .generate_galgame_image_validated(&host, user_id, input, progress.inner())
        .await
        .map_err(|error| error.to_string())
}
