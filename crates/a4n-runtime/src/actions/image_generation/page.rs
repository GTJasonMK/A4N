use a4n_api_contract::image_generation::GeneratePageImageInput;

use super::{
    AppState, ModelRequestPreview, RuntimeImageGenerationHostPort, model_request_preview,
    preview::prepared_image_generation_preview_step,
};

pub async fn preview_page_image_request(
    state: &AppState,
    user_id: i64,
    input: GeneratePageImageInput,
) -> Result<ModelRequestPreview, String> {
    let project_id = input.project_id;
    let chapter_number = input.chapter_number;
    let page_number = input.page_number;
    let host = RuntimeImageGenerationHostPort::new(state);
    let prepared = state
        .image_generation_application()
        .prepare_page_request(&host, user_id, input)
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
