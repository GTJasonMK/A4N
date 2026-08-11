use a4n_core::{
    error::AppError, models::chapter::chapter, source_spans::source_spans_for_chapters,
};
use a4n_prompt::{chapter_analysis::ChapterAnalysisRequestStep, rendering::PromptRenderer};

mod placeholders;
use placeholders::facts_preview_placeholder;
mod steps;
use steps::build_preview_steps;

pub async fn build_chapter_analysis_preview_requests(
    ch: &chapter::Model,
    prompt_renderer: &dyn PromptRenderer,
    pending_foreshadowing: &serde_json::Value,
) -> Result<Vec<ChapterAnalysisRequestStep>, AppError> {
    let source_spans = source_spans_for_chapters(std::slice::from_ref(ch));
    let facts_placeholder = facts_preview_placeholder();
    build_preview_steps(
        ch,
        prompt_renderer,
        &source_spans,
        &facts_placeholder,
        pending_foreshadowing,
    )
    .await
}
