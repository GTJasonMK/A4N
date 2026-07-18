use std::collections::HashMap;

use a4n_llm::provider::LlmProvider;
use tokio_util::sync::CancellationToken;

use a4n_prompt::rendering::PromptRenderer;

mod checkpoint;
mod stream;

pub struct ChapterGenerationStreamInput<'a, F>
where
    F: FnMut(&str, u32),
{
    pub project_id: i64,
    pub chapter_number: i32,
    pub chapter_outline: &'a str,
    pub context_before: Option<&'a str>,
    pub llm: &'a dyn LlmProvider,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, serde_json::Value>,
    pub workflow_run_id: i64,
    pub cancel_token: Option<CancellationToken>,
    pub on_chunk: F,
}
