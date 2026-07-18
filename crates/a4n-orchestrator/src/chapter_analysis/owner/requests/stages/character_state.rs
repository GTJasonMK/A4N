use a4n_core::{
    chapter_analysis::CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE, error::AppError,
    models::chapter::chapter,
};
use a4n_llm::provider::ChatRequest;
use serde_json::{Value, json};

use a4n_prompt::rendering::PromptRenderer;

use crate::chapter_analysis::owner::{
    prompt::required_chapter_title, requests::context::build_chapter_analysis_stage_request,
};

pub async fn build_chapter_character_state_request(
    ch: &chapter::Model,
    prompt_renderer: &dyn PromptRenderer,
    facts: &Value,
) -> Result<ChatRequest, AppError> {
    let chapter_title = required_chapter_title(ch)?;
    build_chapter_analysis_stage_request(
        ch,
        prompt_renderer,
        CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE,
        "基于章节正文和事实候选生成角色章节结束状态",
        json!({
            "chapter_title": chapter_title,
            "content": ch.content,
            "characters": facts.get("characters").cloned().unwrap_or_else(|| json!([])),
            "event_candidates": facts.get("event_candidates").cloned().unwrap_or_else(|| json!([])),
        }),
        json!({
            "format": "json_object",
            "required_fields": ["character_states"]
        }),
    )
    .await
}
