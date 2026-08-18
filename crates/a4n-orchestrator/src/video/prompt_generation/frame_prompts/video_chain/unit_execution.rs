use std::collections::{HashMap, HashSet};

use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::super::{
    ChapterVideoSourceContext, FinalVideoPromptContext, FramePromptGenerationSpec,
    GeneratedVideoPromptShot, OrchestratorContext, PromptOutputLanguage,
    VideoPromptGenerationTrace, VideoSegmentPlan,
};

mod draft;
mod prompts;
mod workflow;

pub(super) use self::workflow::execute_video_prompt_unit;

pub(super) struct VideoPromptUnitExecutionInput<'a, 'source> {
    pub(super) service: &'a OrchestratorContext,
    pub(super) spec: &'a FramePromptGenerationSpec,
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) source: &'a ChapterVideoSourceContext<'source>,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) vars: &'a HashMap<String, Value>,
    pub(super) workflow_run_id: Option<i64>,
    pub(super) cancel_token: Option<&'a CancellationToken>,
    pub(super) segment: &'a VideoSegmentPlan,
    pub(super) segment_by_number: &'a HashMap<i32, &'a VideoSegmentPlan>,
    pub(super) video_segment_json: &'a Value,
    pub(super) frame_json: &'a Value,
    pub(super) previous_completed_shot: Option<&'a GeneratedVideoPromptShot>,
    pub(super) available_assets: &'a Value,
    pub(super) style: &'a str,
    pub(super) prompt_output_language: PromptOutputLanguage,
    pub(super) duration_secs: i32,
    pub(super) aspect_ratio: &'a str,
    pub(super) reference_ids: &'a Value,
    pub(super) allowed_reference_ids: &'a HashSet<i64>,
    pub(super) is_chat_video: bool,
    pub(super) segment_index: usize,
    pub(super) current_index: usize,
    pub(super) total_segments: usize,
    pub(super) generation_number: usize,
    pub(super) generation_limit: usize,
}

pub(super) struct ExecutedVideoPromptUnit {
    pub(super) video_prompt: GeneratedVideoPromptShot,
    pub(super) generation_trace: VideoPromptGenerationTrace,
}

#[derive(Clone, Copy)]
pub(super) struct VideoPromptUnitCheckpointContext<'a, 'source> {
    pub(super) service: &'a OrchestratorContext,
    pub(super) spec: &'a FramePromptGenerationSpec,
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) source: &'a ChapterVideoSourceContext<'source>,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) vars: &'a HashMap<String, Value>,
    pub(super) workflow_run_id: Option<i64>,
    pub(super) cancel_token: Option<&'a CancellationToken>,
    pub(super) segment_by_number: &'a HashMap<i32, &'a VideoSegmentPlan>,
    pub(super) final_prompt_context: FinalVideoPromptContext<'a>,
    pub(super) shot_number: i32,
    pub(super) prompt_output_language: PromptOutputLanguage,
    pub(super) allowed_reference_ids: &'a HashSet<i64>,
    pub(super) is_chat_video: bool,
}
