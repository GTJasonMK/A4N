use a4n_orchestrator::media::BatchImageGenerationInput;
use serde_json::Value;

use super::{
    AppState, ModelRequestPreview, RuntimeImageGenerationHostPort, model_request_preview,
    preview::prepared_image_generation_preview_step,
};

pub async fn preview_batch_images_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    requests_json: Value,
) -> Result<ModelRequestPreview, String> {
    let host = RuntimeImageGenerationHostPort::new(state);
    let requests = state
        .media()
        .prepare_batch_images_request_validated(
            &host,
            user_id,
            BatchImageGenerationInput {
                project_id,
                requests_json,
            },
        )
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
