use std::collections::HashMap;

use a4n_core::config::app_config::llm_token_profiles;
use a4n_llm::provider::{ChatRequest, UsageInfo};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use a4n_core::source_spans::SourceSpan;
use a4n_llm::context::LlmContextManifest;
use a4n_prompt::rendering::PromptRenderer;

pub(in crate::manga) const MANGA_PIPELINE_MAX_TOKENS_PROFILE: &str = llm_token_profiles::EXTRA_LONG;
pub(in crate::manga) const MANGA_PIPELINE_WORKFLOW: &str = "manga_pipeline";
pub(in crate::manga) const MANGA_PIPELINE_PROMPT_NAMES: &[&str] = &[
    "chapter_screenplay_skeleton",
    "chapter_screenplay_unit_adaptation",
    "chapter_shooting_design_unit",
    "manga_character_binding",
    "manga_speech_selection",
    "manga_visual_anchor_extraction",
    "manga_item_extraction",
    "manga_chapter_comic_synthesis",
    "manga_page_planning",
    "manga_page_drawing_prompt",
];
pub(in crate::manga) const MANGA_PREVIOUS_OUTPUT_SHOOTING_SCRIPT_EVENTS: &str =
    "manga_previous_output:shooting_script_events";
pub(in crate::manga) const MANGA_PREVIOUS_OUTPUT_CHARACTER_BINDING: &str =
    "manga_previous_output:character_binding";
pub(in crate::manga) const MANGA_PREVIOUS_OUTPUT_SPEECH_SELECTION: &str =
    "manga_previous_output:speech_selection";
pub(in crate::manga) const MANGA_PREVIOUS_OUTPUT_VISUAL_ANCHOR_EXTRACTION: &str =
    "manga_previous_output:visual_anchor_extraction";
pub(in crate::manga) const MANGA_PREVIOUS_OUTPUT_ITEM_EXTRACTION: &str =
    "manga_previous_output:item_extraction";

#[derive(Debug, Clone, Copy)]
pub(in crate::manga) struct MangaContextPreviousOutput<'a> {
    pub(in crate::manga) pack_kind: &'a str,
    pub(in crate::manga) prompt_name: &'a str,
    pub(in crate::manga) parsed_output: &'a Value,
    pub(in crate::manga) raw_output: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(in crate::manga) struct MangaPromptGenerationTrace {
    pub(in crate::manga) prompt_name: String,
    pub(in crate::manga) request_json: Value,
    pub(in crate::manga) messages_json: Value,
    pub(in crate::manga) response_text: String,
    pub(in crate::manga) workflow_run_id: Option<i64>,
    pub(in crate::manga) call_key: Option<String>,
    pub(in crate::manga) call_order: Option<i32>,
    pub(in crate::manga) input_fingerprint_json: Option<Value>,
    pub(in crate::manga) usage: Option<UsageInfo>,
    pub(in crate::manga) finish_reason: Option<String>,
    pub(in crate::manga) context_manifest_summary_json: Option<Value>,
    #[serde(skip)]
    pub(in crate::manga) restored_parsed_output: Option<Value>,
    #[serde(skip)]
    pub(in crate::manga) repair_request: Option<ChatRequest>,
}

pub(in crate::manga) struct MangaPreparedLlmCall {
    pub(in crate::manga) request: ChatRequest,
    pub(in crate::manga) request_json: Value,
    pub(in crate::manga) messages_json: Value,
    pub(in crate::manga) call_key: String,
    pub(in crate::manga) call_order: i32,
    pub(in crate::manga) input_fingerprint_json: Value,
    pub(in crate::manga) context_manifest_summary_json: Option<Value>,
}

pub(in crate::manga) struct MangaRestoredLlmCheckpoint {
    pub(in crate::manga) response_text: String,
    pub(in crate::manga) parsed_output: Value,
    pub(in crate::manga) usage: Option<UsageInfo>,
    pub(in crate::manga) finish_reason: Option<String>,
}

pub(in crate::manga) struct MangaContextManifestInput<'a> {
    pub(in crate::manga) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::manga) project_id: i64,
    pub(in crate::manga) variables: &'a HashMap<String, Value>,
    pub(in crate::manga) chapter_content: &'a str,
    pub(in crate::manga) source_spans: &'a [SourceSpan],
    pub(in crate::manga) prompt_names: &'a [&'a str],
    pub(in crate::manga) previous_outputs: &'a [MangaContextPreviousOutput<'a>],
    pub(in crate::manga) reference_assets: Option<&'a Value>,
    pub(in crate::manga) dynamic_user_content: String,
}

pub(in crate::manga) struct MangaLlmCallInput<'a> {
    pub(in crate::manga) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::manga) prompt_name: &'a str,
    pub(in crate::manga) project_id: i64,
    pub(in crate::manga) workflow_run_id: Option<i64>,
    pub(in crate::manga) call_key: String,
    pub(in crate::manga) call_order: i32,
    pub(in crate::manga) variables: &'a HashMap<String, Value>,
    pub(in crate::manga) manifest: LlmContextManifest,
    pub(in crate::manga) temperature: f32,
}
