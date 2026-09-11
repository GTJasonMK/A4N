use std::collections::HashMap;

use a4n_db_models::models::chapter::part_outline;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use crate::outline::ProgressReporter;
use crate::outline::context::ChapterOutlineGenerationContext;
use a4n_prompt::rendering::PromptRenderer;

#[derive(Debug, Clone)]
pub(super) struct GeneratedChapterOutline {
    pub chapter_json: Value,
    pub request: ChatRequest,
    pub response_content: String,
    pub finish_reason: Option<String>,
}

pub(super) struct ChapterOutlineSequenceInput<'a, 'progress> {
    pub project_id: i64,
    pub start_chapter: i32,
    pub count: i32,
    pub focus_part: Option<&'a part_outline::Model>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub progress: ProgressReporter<'progress>,
    pub workflow_run_id: Option<i64>,
}

pub(super) struct SingleChapterOutlineJsonInput<'a> {
    pub project_id: i64,
    pub context: &'a ChapterOutlineGenerationContext,
    pub focus_part: Option<&'a part_outline::Model>,
    pub chapter_number: i32,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub workflow_run_id: Option<i64>,
}

pub(super) struct ChapterOutlineRequestInput<'a> {
    pub project_id: i64,
    pub context: &'a ChapterOutlineGenerationContext,
    pub focus_part: Option<&'a part_outline::Model>,
    pub start_chapter: i32,
    pub count: i32,
    pub end_chapter: i32,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
}

pub(super) struct ContextAwareChapterOutlineRequestInput<'a> {
    pub project_id: i64,
    pub context: &'a ChapterOutlineGenerationContext,
    pub prompt_content: String,
    pub user_content: String,
    pub count: i32,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub temperature: Option<f32>,
    pub workflow: &'a str,
    pub step_key: String,
}
