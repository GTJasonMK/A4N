use a4n_orchestrator::media::GenerateBatchImagesInput;

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
        .orchestrator()
        .prepare_batch_images_request_validated(&host, user_id, input)
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
