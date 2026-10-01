use a4n_chapter_analysis::CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE;
use a4n_domain::chapter::ChapterRecord;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use serde_json::{Value, json};

use a4n_prompt::rendering::PromptRenderer;

use super::required_fact_field;
use crate::chapter_analysis::owner::{
    prompt::chapter_display_title, requests::context::build_chapter_analysis_stage_request,
};

pub async fn build_chapter_character_state_request(
    ch: &ChapterRecord,
    prompt_renderer: &dyn PromptRenderer,
    facts: &Value,
) -> Result<ChatRequest, AppError> {
    let chapter_title = chapter_display_title(ch);
    let characters = required_fact_field(facts, "characters")?;
    let event_candidates = required_fact_field(facts, "event_candidates")?;
    build_chapter_analysis_stage_request(
        ch,
        prompt_renderer,
        CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE,
        "基于章节正文和事实候选生成角色章节结束状态",
        json!({
            "chapter_title": chapter_title,
            "content": ch.content,
            "characters": characters,
            "event_candidates": event_candidates,
        }),
        json!({
            "format": "json_object",
            "required_fields": ["character_states"]
        }),
    )
    .await
}
