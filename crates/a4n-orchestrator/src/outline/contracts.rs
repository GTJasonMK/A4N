use std::collections::HashMap;

use a4n_api_contract::outline::OutlineDeleteTarget;
use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

pub type ProgressReporter<'a> =
    Option<&'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync)>;

pub struct ChapterOutlineGenerationInput<'a, 'progress> {
    pub project_id: i64,
    pub part_id: i64,
    pub start_chapter: i32,
    pub count: i32,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub progress: ProgressReporter<'progress>,
    pub workflow_run_id: Option<i64>,
}

pub struct ChapterOutlineCountGenerationInput<'a, 'progress> {
    pub project_id: i64,
    pub start_chapter: i32,
    pub count: i32,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub progress: ProgressReporter<'progress>,
    pub workflow_run_id: Option<i64>,
}

pub struct ChapterOutlineRegenerationInput<'a, 'progress> {
    pub project_id: i64,
    pub outline_id: i64,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub progress: ProgressReporter<'progress>,
    pub workflow_run_id: Option<i64>,
}

pub struct BeatMappingApplyInput<'a, 'progress> {
    pub project_id: i64,
    pub part_id: i64,
    pub beat_sheet: &'a str,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub progress: ProgressReporter<'progress>,
    pub workflow_run_id: Option<i64>,
}

pub struct PartOutlineGenerationInput<'a, 'progress> {
    pub project_id: i64,
    pub part_count: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub progress: ProgressReporter<'progress>,
    pub workflow_run_id: Option<i64>,
}

pub struct PartOutlineRegenerationInput<'a, 'progress> {
    pub project_id: i64,
    pub expected_parts: &'a [OutlineDeleteTarget],
    pub chapters_per_part: Option<i32>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub progress: ProgressReporter<'progress>,
    pub workflow_run_id: Option<i64>,
}

pub struct PartOutlineContinuationInput<'a> {
    pub project_id: i64,
    pub part_number: i32,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub workflow_run_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct OutlineRequestPreviewStep {
    pub key: String,
    pub title: String,
    pub prompt_name: String,
    pub state: String,
    pub note: Option<String>,
    pub request: Option<ChatRequest>,
}
