use a4n_core::{error::AppError, models::chapter::chapter, source_spans::SourceSpan};
use serde_json::Value;

use a4n_prompt::{chapter_analysis::ChapterAnalysisRequestStep, rendering::PromptRenderer};

mod derived;
mod facts;
use derived::build_derived_preview_steps;
use facts::build_facts_preview_step;

pub(super) async fn build_preview_steps(
    ch: &chapter::Model,
    prompt_renderer: &dyn PromptRenderer,
    source_spans: &[SourceSpan],
    facts_placeholder: &Value,
) -> Result<Vec<ChapterAnalysisRequestStep>, AppError> {
    let mut steps = vec![build_facts_preview_step(ch, prompt_renderer, source_spans).await?];
    steps.extend(build_derived_preview_steps(ch, prompt_renderer, facts_placeholder).await?);
    Ok(steps)
}
