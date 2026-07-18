use std::collections::HashMap;

use a4n_core::{models::chapter::chapter, prompt_language::PromptOutputLanguage};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::duration::VideoDurationCapability;

pub(super) const IMPORT_VIDEO_PROMPT_TYPE: &str = "video_prompt";
pub const FRAME_SEGMENT_PLAN_CONTRACT_VERSION: &str = "video_execution_contract_v5";
pub const BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION: &str = "boundary_frame_shared_chain_v3";
pub const FIRST_FRAME_CHAIN_CONTRACT_VERSION: &str = "first_frame_chain_execution_v4";
pub const CHAT_VIDEO_CONTRACT_VERSION: &str = "chat_video_execution_v4";
pub const STORYBOARD_PROMPT_CONTRACT_VERSION: &str = "storyboard_execution_unit_v2";
pub const STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION: &str = "storyboard_execution_video_v2";
pub(super) const FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT: usize = 1;

pub(super) type VideoPromptProgress<'a> =
    &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), a4n_core::error::AppError> + Send);
pub(super) type VideoPromptChunk<'a> = &'a mut (dyn FnMut(&str, u32) + Send);

#[derive(Debug, Clone, PartialEq)]
pub struct ChapterVideoSource {
    pub content: String,
    pub summary: Option<String>,
    pub analysis: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectChapterVideoSource {
    pub chapter: chapter::Model,
    pub source: ChapterVideoSource,
}

#[derive(Debug, Clone)]
pub struct ChapterVideoPromptOptions {
    pub style: String,
    pub prompt_output_language: PromptOutputLanguage,
    pub prompt_batch_size: i32,
    pub duration_secs: i32,
    pub duration_secs_options: Vec<i32>,
    pub aspect_ratio: String,
    pub reference_image_ids: Vec<i64>,
    pub video_library_image_ids: Vec<i64>,
    pub duration_capability: Option<VideoDurationCapability>,
    pub regenerate_existing: bool,
    pub workflow_run_id: Option<i64>,
}

pub struct ChapterVideoPromptStageInput<'a, 'source> {
    pub project_id: i64,
    pub chapter_id: i64,
    pub workflow_mode: &'a str,
    pub source: ChapterVideoSourceContext<'source>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub options: ChapterVideoPromptOptions,
    pub cancel_token: Option<&'a CancellationToken>,
}

#[derive(Debug, Clone, Copy)]
pub struct ChapterVideoSourceContext<'a> {
    pub content: &'a str,
    pub summary: Option<&'a str>,
    pub analysis: Option<&'a Value>,
}
