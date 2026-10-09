use std::collections::HashMap;

use a4n_domain::media::GeneratedImageRecord;
use a4n_llm_provider_contract::provider::ChatRequest;
use serde_json::Value;

use crate::media_authoring::PreparedAdaptation;
use a4n_domain::source_spans::SourceSpan;
use a4n_prompt::rendering::PromptRenderer;

use super::super::super::OrchestratorContext;
use crate::source_markers::SourceMarkerMap;

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
    pub(super) adaptation: PreparedAdaptation,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) workflow_run_id: Option<i64>,
    pub(super) resume_from_checkpoint: bool,
    pub(super) stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
    pub(super) run_stage: Option<a4n_domain::manga::MangaPipelineArtifactStage>,
    pub(super) page_number: Option<u32>,
    pub(super) chapter_content: String,
    pub(super) source_spans: Vec<SourceSpan>,
    pub(super) source_markers: SourceMarkerMap,
    pub(super) variables: HashMap<String, Value>,
    pub(super) reference_assets: Value,
    pub(super) available_reference_images: Vec<GeneratedImageRecord>,
}
