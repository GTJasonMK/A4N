use std::collections::HashMap;

use serde_json::Value;

use super::{
    FRAME_SEGMENT_PLAN_CONTRACT_VERSION, NewChapterVideoPrompt, PromptOutputLanguage,
    PromptWorkflowMode, SourceSpan, VideoPromptGenerationTrace, VideoSegmentPlan,
};

pub(super) struct PreparedVideoPromptInputs {
    pub(super) source_metadata: Value,
    pub(super) source_spans: Vec<SourceSpan>,
    pub(super) regenerate_existing: bool,
    pub(super) prompt_batch_size: usize,
    pub(super) style: String,
    pub(super) prompt_output_language: PromptOutputLanguage,
    pub(super) duration_secs: i32,
    pub(super) aspect_ratio: String,
    pub(super) reference_image_ids: Vec<i64>,
    pub(super) video_library_image_ids: Vec<i64>,
    pub(super) reference_ids: Value,
    pub(super) generation_asset_fingerprint: Value,
    pub(super) project_context: Option<Value>,
    pub(super) available_assets: Value,
    pub(super) vars: HashMap<String, Value>,
    pub(super) workflow_run_id: Option<i64>,
}

pub(super) struct PreparedSegmentPlan {
    pub(super) segment_stage_outputs: Value,
    pub(super) segment_trace: Option<VideoPromptGenerationTrace>,
    pub(super) segment_json: Value,
    pub(super) segment_plan: Vec<VideoSegmentPlan>,
    pub(super) execution_identity: Value,
}

pub(crate) struct ReusableSegmentPlan {
    pub(crate) segment_json: Value,
    pub(crate) segment_plan: Vec<VideoSegmentPlan>,
    pub(super) draft: NewChapterVideoPrompt,
    pub(crate) execution_identity: Value,
}

pub(super) const SHARED_SEGMENT_PLAN_WORKFLOW_MODE: PromptWorkflowMode =
    PromptWorkflowMode::BoundaryFrames;
pub(super) const SHARED_SEGMENT_PLAN_PROMPT_TYPE: &str = "frame_segment_plan";
pub(super) const SHARED_SEGMENT_PLAN_TITLE: &str = "视频片段规划";
pub(super) const SHARED_SEGMENT_PLAN_CONTEXT: &str = "视频分段规划";
pub(super) const SHARED_SEGMENT_PLAN_PROGRESS: &str = "正在规划视频片段";
pub(super) const SHARED_SEGMENT_PLAN_CONTRACT_VERSION: &str = FRAME_SEGMENT_PLAN_CONTRACT_VERSION;
