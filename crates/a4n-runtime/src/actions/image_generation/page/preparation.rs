use a4n_orchestrator::manga::MANGA_PAGE_PROMPT_TYPE;
use serde_json::{Value, json};

use crate::AppState;

use super::super::contracts::PageImageRequestContext;
use super::super::generation_io::image_generation_request_trace_json;
use super::super::request::build_page_image_request;
use super::super::source_binding::image_source_metadata;

pub(super) struct PreparedPageImageGeneration {
    pub(super) context: PageImageRequestContext,
    pub(super) request_json: Value,
    pub(super) metadata: Value,
}

pub(super) async fn prepare_page_image_generation(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    page_number: i32,
) -> Result<PreparedPageImageGeneration, String> {
    let context = build_page_image_request(state, project_id, chapter_number, page_number).await?;
    let request_json = image_generation_request_trace_json(&context.config, &context.request);
    let metadata = json!({
        "config_id": context.config.id,
        "backend": context.config.backend.clone(),
        "chapter_number": chapter_number,
        "page_number": page_number,
        "panel_id": context.panel_id.clone(),
        "prompt_type": MANGA_PAGE_PROMPT_TYPE,
        "page_prompt_id": context.page_prompt.id,
        "generation_mode": "direct_page",
        "reference_bindings": context.reference_bindings.clone(),
        "reference_image_count": context.reference_image_count,
        "reference_image_mode": context.reference_image_mode.clone(),
        "source": image_source_metadata(&context.chapter.content),
    });
    Ok(PreparedPageImageGeneration {
        context,
        request_json,
        metadata,
    })
}
