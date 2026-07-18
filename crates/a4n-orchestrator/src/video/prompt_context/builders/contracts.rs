use serde_json::{Map, Value};

use a4n_core::prompt_language::PromptOutputLanguage;
use a4n_prompt::rendering::PromptRenderer;

#[derive(Clone, Copy)]
pub(super) enum VideoPromptContractMode {
    Api,
    Chat,
}

pub(super) struct VideoPromptCommonConstraintInput<'a> {
    pub(super) rules: &'a Map<String, Value>,
    pub(super) mode: VideoPromptContractMode,
    pub(super) style: &'a str,
    pub(super) prompt_output_language: PromptOutputLanguage,
    pub(super) duration_secs: i32,
    pub(super) aspect_ratio: &'a str,
    pub(super) reference_ids: &'a Value,
}

#[derive(Clone, Copy)]
pub(in crate::video) struct VideoPlanningPromptContext<'a> {
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub project_id: i64,
    pub chapter_summary: Option<&'a str>,
    pub chapter_analysis: Option<&'a Value>,
    pub project_context: Option<&'a Value>,
    pub style: &'a str,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
}

#[derive(Clone, Copy)]
pub(in crate::video) struct FinalVideoPromptContext<'a> {
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub project_id: i64,
    pub chapter_summary: Option<&'a str>,
    pub chapter_analysis: Option<&'a Value>,
    pub segment_plan: &'a Value,
    pub frame_prompts: &'a Value,
    pub current_segment: &'a Value,
    pub previous_completed_shot: Option<&'a Value>,
    pub available_assets: &'a Value,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub reference_ids: &'a Value,
}

#[derive(Clone, Copy)]
pub(in crate::video) struct FramePromptContext<'a> {
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub project_id: i64,
    pub chapter_summary: Option<&'a str>,
    pub chapter_analysis: Option<&'a Value>,
    pub segment_plan: &'a Value,
    pub available_assets: &'a Value,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub aspect_ratio: &'a str,
    pub reference_ids: &'a Value,
}
