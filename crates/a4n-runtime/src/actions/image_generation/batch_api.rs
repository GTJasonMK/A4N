use a4n_api_contract::image_generation::GenerateBatchImagesInput;

use super::{
    AppState, ModelRequestPreview, RuntimeImageGenerationHostPort, model_request_preview,
    preview::prepared_image_generation_preview_step,
};

pub async fn preview_batch_images_request(
    state: &AppState,
    user_id: i64,
    input: GenerateBatchImagesInput,
) -> Result<ModelRequestPreview, String> {
    let project_id = input.project_id;
    let host = RuntimeImageGenerationHostPort::new(state);
    let requests = state
        .image_generation_application()
        .prepare_batch_request(&host, user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    let total = requests.len();
    let mut steps = Vec::with_capacity(total);
    for (index, request) in requests.into_iter().enumerate() {
        steps.push(
            prepared_image_generation_preview_step(
                state,
                project_id,
                format!("batch_image_{}", index + 1),
                format!("批量图片 {}/{} provider 请求", index + 1, total),
                request,
                true,
            )
            .await?,
        );
    }
    Ok(model_request_preview(
        "batch_image_generation",
        "批量图片模型请求预览",
        steps,
    ))
}
