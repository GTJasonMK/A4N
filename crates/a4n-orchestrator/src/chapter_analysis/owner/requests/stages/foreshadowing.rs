use a4n_core::{
    chapter_analysis::CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS, error::AppError,
    models::chapter::chapter,
};
use a4n_llm::provider::ChatRequest;
use serde_json::{Value, json};

use a4n_prompt::rendering::PromptRenderer;

use crate::chapter_analysis::owner::{
    prompt::required_chapter_title, requests::context::build_chapter_analysis_stage_request,
};

pub async fn build_chapter_foreshadowing_events_request(
    ch: &chapter::Model,
    prompt_renderer: &dyn PromptRenderer,
    facts: &Value,
) -> Result<ChatRequest, AppError> {
    let chapter_title = required_chapter_title(ch)?;
    build_chapter_analysis_stage_request(
        ch,
        prompt_renderer,
        CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS,
        "从事实候选中筛选最终伏笔和关键事件",
        json!({
            "chapter_title": chapter_title,
            "facts": {
                "event_candidates": facts.get("event_candidates").cloned().unwrap_or_else(|| json!([])),
                "foreshadowing_candidates": facts.get("foreshadowing_candidates").cloned().unwrap_or_else(|| json!([])),
            }
        }),
        json!({
            "format": "json_object",
            "required_fields": ["foreshadowing", "key_events"],
            "source_span_rule": "只能使用 facts 中已有的 source_span_ids 和 original_text_span_id"
        }),
    )
    .await
}
