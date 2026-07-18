use a4n_core::{
    chapter_analysis::CHAPTER_ANALYSIS_STAGE_FACTS, error::AppError, models::chapter::chapter,
    source_spans::SourceSpan,
};
use a4n_prompt::{chapter_analysis::ChapterAnalysisRequestStep, rendering::PromptRenderer};

use crate::chapter_analysis::owner::requests::stages::build_chapter_fact_extraction_request;

pub(super) async fn build_facts_preview_step(
    ch: &chapter::Model,
    prompt_renderer: &dyn PromptRenderer,
    source_spans: &[SourceSpan],
) -> Result<ChapterAnalysisRequestStep, AppError> {
    Ok(ChapterAnalysisRequestStep {
        key: CHAPTER_ANALYSIS_STAGE_FACTS.to_string(),
        title: "章节分析 1/4：事实候选抽取".to_string(),
        prompt_name: CHAPTER_ANALYSIS_STAGE_FACTS.to_string(),
        request: build_chapter_fact_extraction_request(ch, prompt_renderer, source_spans).await?,
    })
}
