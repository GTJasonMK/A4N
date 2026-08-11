use a4n_orchestrator::media::PageImageGenerationInput;

use super::{
    AppState, ModelRequestPreview, RuntimeImageGenerationHostPort, model_request_preview,
    preview::prepared_image_generation_preview_step,
};

pub async fn preview_page_image_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    page_number: i32,
) -> Result<ModelRequestPreview, String> {
    let host = RuntimeImageGenerationHostPort::new(state);
    let prepared = state
        .orchestrator()
        .prepare_page_image_request_validated(
            &host,
            user_id,
            PageImageGenerationInput {
                project_id,
                chapter_number,
                page_number,
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(model_request_preview(
        "manga_page_image_generation",
        "整页漫画图片模型请求预览",
        vec![
            prepared_image_generation_preview_step(
                state,
                project_id,
                "page_image",
                format!("第 {chapter_number} 章第 {page_number} 页 provider 请求"),
                prepared,
                false,
            )
            .await?,
        ],
    ))
}
