use a4n_api_contract::{
    image::GeneratedImageSummary,
    image_generation::{GenerateGalgameImageInput, GenerateSceneImageInput},
};
use a4n_canvas_contract::GenerateCanvasImageInput;
use a4n_orchestrator::media::ImageGenerationProgress;

use super::{
    AppState, ModelRequestPreview, RuntimeImageGenerationHostPort, model_request_preview,
    preview::prepared_image_generation_preview_step,
};

pub async fn preview_scene_image_request(
    state: &AppState,
    user_id: i64,
    input: GenerateSceneImageInput,
) -> Result<ModelRequestPreview, String> {
    let project_id = input.project_id;
    let host = RuntimeImageGenerationHostPort::new(state);
    let prepared = state
        .image_generation_application()
        .prepare_scene_request(&host, user_id, input)
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

pub(crate) async fn preview_canvas_image_request(
    state: &AppState,
    user_id: i64,
    input: GenerateCanvasImageInput,
) -> Result<ModelRequestPreview, String> {
    let project_id = input.project_id;
    let host = RuntimeImageGenerationHostPort::new(state);
    let prepared = state
        .image_generation_application()
        .prepare_canvas_request(&host, user_id, input)
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
) -> Result<GeneratedImageSummary, String> {
    let progress = ImageGenerationProgress::none();
    let host = RuntimeImageGenerationHostPort::new(state);
    state
        .image_generation_application()
        .generate_galgame_image(&host, user_id, input, &progress)
        .await
        .map_err(|error| error.to_string())
}
