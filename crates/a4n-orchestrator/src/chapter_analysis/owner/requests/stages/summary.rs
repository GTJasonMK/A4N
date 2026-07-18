use a4n_core::{
    chapter_analysis::CHAPTER_ANALYSIS_STAGE_SUMMARY, error::AppError, models::chapter::chapter,
};
use a4n_llm::provider::ChatRequest;
use serde_json::{Value, json};

use a4n_prompt::rendering::PromptRenderer;

use crate::chapter_analysis::owner::{
    prompt::required_chapter_title, requests::context::build_chapter_analysis_stage_request,
};

pub async fn build_chapter_summary_synthesis_request(
    ch: &chapter::Model,
    prompt_renderer: &dyn PromptRenderer,
    facts: &Value,
) -> Result<ChatRequest, AppError> {
    let chapter_title = required_chapter_title(ch)?;
    build_chapter_analysis_stage_request(
        ch,
        prompt_renderer,
        CHAPTER_ANALYSIS_STAGE_SUMMARY,
        "基于章节正文和事实候选生成摘要、标签、基调和时间标记",
        json!({
            "chapter_title": chapter_title,
            "content": ch.content,
            "facts": facts,
        }),
        json!({
            "format": "json_object",
            "required_fields": ["metadata_additions", "summaries"]
        }),
    )
    .await
}
