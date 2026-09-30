use a4n_chapter_analysis::CHAPTER_ANALYSIS_STAGE_SUMMARY;
use a4n_domain::chapter::ChapterRecord;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use serde_json::{Value, json};

use a4n_prompt::rendering::PromptRenderer;

use crate::chapter_analysis::owner::{
    prompt::chapter_display_title, requests::context::build_chapter_analysis_stage_request,
};

pub async fn build_chapter_summary_synthesis_request(
    ch: &ChapterRecord,
    prompt_renderer: &dyn PromptRenderer,
    facts: &Value,
) -> Result<ChatRequest, AppError> {
    let chapter_title = chapter_display_title(ch);
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
