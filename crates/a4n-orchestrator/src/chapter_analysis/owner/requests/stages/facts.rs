use a4n_core::{
    chapter_analysis::CHAPTER_ANALYSIS_STAGE_FACTS, error::AppError, models::chapter::chapter,
    source_spans::SourceSpan,
};
use a4n_llm::provider::ChatRequest;
use serde_json::json;

use a4n_prompt::rendering::PromptRenderer;

use crate::chapter_analysis::owner::{
    prompt::chapter_display_title, requests::context::build_chapter_analysis_stage_request,
};

pub async fn build_chapter_fact_extraction_request(
    ch: &chapter::Model,
    prompt_renderer: &dyn PromptRenderer,
    source_spans: &[SourceSpan],
) -> Result<ChatRequest, AppError> {
    let chapter_title = chapter_display_title(ch);
    let input = json!({
        "chapter_title": chapter_title,
        "content": ch.content,
        "source_spans": a4n_core::source_spans::source_span_prompt_values(source_spans),
    });
    build_chapter_analysis_stage_request(
        ch,
        prompt_renderer,
        CHAPTER_ANALYSIS_STAGE_FACTS,
        "从章节正文抽取角色、地点、物品、关键事件和伏笔候选",
        input,
        json!({
            "format": "json_object",
            "required_fields": ["characters", "locations", "items", "event_candidates", "foreshadowing_candidates"],
            "source_span_rule": "所有 source_span_ids 和 original_text_span_id 必须来自稳定上下文 chapter_analysis_source.source_spans[].id"
        }),
    )
    .await
}
