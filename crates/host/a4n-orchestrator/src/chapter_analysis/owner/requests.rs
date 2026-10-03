use a4n_chapter_analysis::CHAPTER_ANALYSIS_STAGE_FACTS;
use a4n_chapter_analysis_host_contract::ChapterAnalysisRequestStep;
use a4n_domain::chapter::ChapterRecord;
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;

mod context;
mod stages;
mod support;

pub use stages::{
    build_chapter_character_state_request, build_chapter_fact_extraction_request,
    build_chapter_foreshadowing_events_request, build_chapter_summary_synthesis_request,
    chapter_foreshadowing_resolution_context,
};
pub use support::{chapter_analysis_root_field_progress, json_value_shape_summary};

pub async fn build_chapter_analysis_initial_request(
    chapter: &ChapterRecord,
    prompt_renderer: &dyn PromptRenderer,
) -> Result<ChapterAnalysisRequestStep, AppError> {
    Ok(ChapterAnalysisRequestStep {
        key: CHAPTER_ANALYSIS_STAGE_FACTS.to_string(),
        title: "章节分析 1/4：事实候选抽取".to_string(),
        prompt_name: CHAPTER_ANALYSIS_STAGE_FACTS.to_string(),
        request: build_chapter_fact_extraction_request(chapter, prompt_renderer).await?,
    })
}
