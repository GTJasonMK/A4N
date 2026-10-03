use a4n_chapter_analysis::CHAPTER_ANALYSIS_STAGE_FACTS;
use a4n_domain::chapter::ChapterRecord;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use serde_json::json;

use a4n_prompt::rendering::PromptRenderer;

use crate::chapter_analysis::owner::{
    prompt::chapter_display_title, requests::context::build_chapter_analysis_stage_request,
};

pub async fn build_chapter_fact_extraction_request(
    ch: &ChapterRecord,
    prompt_renderer: &dyn PromptRenderer,
) -> Result<ChatRequest, AppError> {
    let chapter_title = chapter_display_title(ch);
    let input = json!({
        "chapter_title": chapter_title,
        "content": ch.content,
    });
    build_chapter_analysis_stage_request(
        ch,
        prompt_renderer,
        CHAPTER_ANALYSIS_STAGE_FACTS,
        "从章节正文抽取角色、地点、物品、关键事件、伏笔候选和有来源的世界事实",
        input,
        json!({
            "format": "json_object",
            "required_fields": ["characters", "locations", "items", "event_candidates", "foreshadowing_candidates", "world_facts"],
            "source_span_rule": "所有 source_span_ids 和 original_text_span_id 必须原样复制 chapter_analysis_source.content 中实际出现的十进制来源短标记，不要输出原始 ID"
        }),
    )
    .await
}
