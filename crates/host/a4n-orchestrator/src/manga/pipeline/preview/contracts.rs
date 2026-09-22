use std::collections::HashMap;

use a4n_llm_provider_contract::provider::ChatRequest;
use serde_json::Value;

use a4n_db::shooting_script::ParsedChapterShootingScript;
use a4n_domain::source_spans::SourceSpan;
use a4n_prompt::rendering::PromptRenderer;

use super::super::super::OrchestratorContext;

#[derive(Debug, Clone)]
pub struct MangaPipelineRequestPreviewStep {
    pub key: String,
    pub title: String,
    pub prompt_name: Option<String>,
    pub state: String,
    pub note: Option<String>,
    pub request: Option<ChatRequest>,
}

pub(super) struct MangaPreviewCallInput<'a> {
    pub(super) service: &'a OrchestratorContext,
    pub(super) prompt_name: &'a str,
    pub(super) workflow_run_id: Option<i64>,
    pub(super) call_key: String,
    pub(super) call_order: i32,
    pub(super) restore_from_checkpoint: bool,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) project_id: i64,
    pub(super) variables: &'a HashMap<String, Value>,
    pub(super) user_content: a4n_llm_context::LlmContextManifest,
    pub(super) temperature: f32,
}

pub(super) enum MangaPreviewCall {
    Ready(Box<ChatRequest>),
    Restored { parsed_output: Value },
}

pub(super) struct MangaPreviewContext<'a> {
    pub(super) service: &'a OrchestratorContext,
    pub(super) chapter_id: i64,
    pub(super) project_id: i64,
    pub(super) shooting_script: ParsedChapterShootingScript,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) workflow_run_id: Option<i64>,
    pub(super) resume_from_checkpoint: bool,
    pub(super) chapter_content: String,
    pub(super) source_spans: Vec<SourceSpan>,
    pub(super) variables: HashMap<String, Value>,
    pub(super) reference_assets: Value,
}

pub(super) struct MangaEarlyExtraction {
    pub(super) events_json: Value,
    pub(super) characters_json: Value,
    pub(super) speech_json: Value,
}

pub(super) struct MangaExtractionPreview {
    pub(super) characters_json: Value,
    pub(super) events_json: Value,
    pub(super) speech_json: Value,
    pub(super) visual_json: Value,
    pub(super) items_json: Value,
    pub(super) comic_synthesis_json: Value,
}
