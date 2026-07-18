use super::super::{
    AppState, ImageGenerationRequest, MANGA_PAGE_PROMPT_TYPE, PageImageRequestContext,
    collect_bound_reference_images, ensure_manga_page_panels_match_current_source,
    ensure_manga_source_can_write, image_backend_supports_reference_images, resolve_image_config,
};
use crate::actions::app_error_message;

pub async fn build_page_image_request(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    page_number: i32,
) -> Result<PageImageRequestContext, String> {
    let chapter = state
        .orchestrator
        .chapter_manga_source_by_number(project_id, chapter_number, true)
        .await
        .map_err(|error| error.to_string())?;
    ensure_manga_source_can_write(&chapter, "生成漫画页图片").map_err(|error| error.to_string())?;
    if chapter.content.trim().is_empty() {
        return Err("章节正文为空，无法生成漫画页图片".into());
    }
    let page_prompt = state
        .orchestrator
        .manga_page_prompt_for_image_generation(
            project_id,
            chapter.chapter_id,
            page_number,
            MANGA_PAGE_PROMPT_TYPE,
        )
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| {
            format!(
                "第 {chapter_number} 章第 {page_number} 页没有整页漫画提示词，请先运行漫画流水线"
            )
        })?;
    ensure_manga_page_panels_match_current_source(
        std::slice::from_ref(&page_prompt),
        &chapter.content,
    )?;
    let panel_id = format!("page{page_number}");
    let reference_bindings = page_prompt
        .layout
        .as_ref()
        .and_then(|layout| layout.get("reference_bindings"))
        .cloned();
    let reference_images = collect_bound_reference_images(state, page_prompt.layout.as_ref())?;
    let reference_image_count = reference_images.len();
    let config = resolve_image_config(state, None).await?;
    let reference_image_mode = if reference_images.is_empty() {
        "none".to_string()
    } else if image_backend_supports_reference_images(&config.backend) {
        "image_input".to_string()
    } else {
        return Err(format!(
            "当前图片后端 {} 不支持真实参考图输入，请切换到 openai-compatible-image-edit、openai-chat-image 或 cliproxy-image，或解除本页参考图绑定",
            config.backend
        ));
    };
    let image_prompt = page_prompt.image_prompt.trim();
    if image_prompt.is_empty() {
        return Err(format!(
            "第 {chapter_number} 章第 {page_number} 页整页漫画提示词为空，请先重新生成或补全提示词"
        ));
    }
    Ok(PageImageRequestContext {
        request: ImageGenerationRequest {
            prompt: image_prompt.to_string(),
            width: config.default_width,
            height: config.default_height,
            model: config.model.clone(),
            reference_images,
        },
        config,
        chapter,
        page_prompt,
        panel_id,
        reference_bindings,
        reference_image_count,
        reference_image_mode,
    })
}
