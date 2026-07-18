use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use a4n_prompt::chapter_generation::build_chapter_generation_request;
use a4n_prompt::rendering::PromptRenderer;

pub(super) async fn build_stream_chapter_request(
    project_id: i64,
    chapter_number: i32,
    chapter_outline: &str,
    context_before: Option<&str>,
    prompt_renderer: &dyn PromptRenderer,
    variables: &HashMap<String, Value>,
) -> Result<ChatRequest, AppError> {
    let prompt_content = prompt_renderer
        .render_prompt("writing", variables, Some(project_id))
        .await?;
    build_chapter_generation_request(
        project_id,
        chapter_number,
        prompt_content,
        chapter_outline,
        context_before,
    )
}
