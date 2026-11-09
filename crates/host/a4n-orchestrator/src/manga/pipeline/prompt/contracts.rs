use std::collections::HashMap;

use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatRequest, UsageInfo};
use serde::Serialize;
use serde_json::Value;

use a4n_prompt::rendering::PromptRenderer;

pub(in crate::manga) const MANGA_PIPELINE_WORKFLOW: &str = "manga_pipeline";
pub(in crate::manga) const MANGA_PIPELINE_PROMPT_NAMES: &[&str] = &[
    "manga_preprocessing_facts",
    "manga_preprocessing_speech",
    "manga_preprocessing_visuals",
    "manga_preprocessing_synthesis",
    "manga_page_slots",
    "manga_page_adaptation_plan",
    "manga_page_storyboard",
    "manga_page_reference_discovery",
    "manga_page_drawing_prompt",
];

pub(in crate::manga) fn manga_pipeline_prompt_assembly_call_key(
    prompt_name: &str,
) -> Result<&'static str, AppError> {
    match prompt_name {
        "manga_preprocessing_facts"
        | "manga_preprocessing_speech"
        | "manga_preprocessing_visuals"
        | "manga_preprocessing_synthesis" => Ok("OrchestratorContext.run_manga_preprocessing"),
        "manga_page_slots" => Ok("MangaPipeline.generate_page_prompts"),
        "manga_page_adaptation_plan"
        | "manga_page_storyboard"
        | "manga_page_reference_discovery"
        | "manga_page_drawing_prompt" => Ok("MangaPipeline.process_page"),
        _ => Err(AppError::InvalidArgument(format!(
            "漫画流水线提示词未注册装配调用：{prompt_name}"
        ))),
    }
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::manga) struct MangaPromptGenerationTrace {
    pub(in crate::manga) restored_from_checkpoint: bool,
    pub(in crate::manga) prompt_name: String,
    pub(in crate::manga) request_json: Value,
    pub(in crate::manga) messages_json: Value,
    pub(in crate::manga) response_text: String,
    pub(in crate::manga) workflow_run_id: i64,
    pub(in crate::manga) call_key: String,
    pub(in crate::manga) call_order: i32,
    pub(in crate::manga) input_fingerprint_json: Value,
    pub(in crate::manga) usage: Option<UsageInfo>,
    pub(in crate::manga) finish_reason: Option<String>,
}

pub(in crate::manga) struct MangaPreparedLlmCall {
    pub(in crate::manga) request: ChatRequest,
    pub(in crate::manga) request_json: Value,
    pub(in crate::manga) messages_json: Value,
    pub(in crate::manga) call_key: String,
    pub(in crate::manga) call_order: i32,
    pub(in crate::manga) input_fingerprint_json: Value,
}

pub(in crate::manga) struct MangaRestoredLlmCheckpoint {
    pub(in crate::manga) response_text: String,
    pub(in crate::manga) parsed_output: Value,
    pub(in crate::manga) usage: Option<UsageInfo>,
    pub(in crate::manga) finish_reason: Option<String>,
}

pub(in crate::manga) struct MangaLlmCallInput<'a> {
    pub(in crate::manga) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::manga) prompt_name: &'a str,
    pub(in crate::manga) project_id: i64,
    pub(in crate::manga) workflow_run_id: Option<i64>,
    pub(in crate::manga) call_key: String,
    pub(in crate::manga) call_order: i32,
    pub(in crate::manga) variables: &'a HashMap<String, Value>,
    pub(in crate::manga) user_content: a4n_llm_context::LlmContextManifest,
    pub(in crate::manga) temperature: f32,
}
