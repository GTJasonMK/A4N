use std::collections::HashMap;

use serde_json::Value;

use super::{
    FRAME_SEGMENT_PLAN_CONTRACT_VERSION, NewChapterVideoPrompt, PromptOutputLanguage, SourceSpan,
    VideoPromptGenerationTrace, VideoSegmentPlan, VideoShootingScriptProjection,
};
use a4n_core::video_execution::VideoSegmentExecutionRecipeV1;

pub(in crate::video) struct PreparedVideoPromptInputs {
    pub(in crate::video) source_metadata: Value,
    pub(in crate::video) source_spans: Vec<SourceSpan>,
    pub(in crate::video) regenerate_existing: bool,
    pub(in crate::video) prompt_batch_size: usize,
    pub(in crate::video) target_shot_number: Option<i32>,
    pub(in crate::video) target_narrative_unit_index: Option<u32>,
    pub(in crate::video) segment_plan_prompt_id: Option<i64>,
    pub(in crate::video) segment_recipe: Option<VideoSegmentExecutionRecipeV1>,
    pub(in crate::video) style: String,
    pub(in crate::video) prompt_output_language: PromptOutputLanguage,
    pub(in crate::video) duration_secs: i32,
    pub(in crate::video) aspect_ratio: String,
    pub(in crate::video) reference_image_ids: Vec<i64>,
    pub(in crate::video) video_library_image_ids: Vec<i64>,
    pub(in crate::video) reference_ids: Value,
    pub(in crate::video) generation_asset_fingerprint: Value,
    pub(in crate::video) project_context: Option<Value>,
    pub(in crate::video) available_assets: Value,
    pub(in crate::video) vars: HashMap<String, Value>,
    pub(in crate::video) workflow_run_id: Option<i64>,
}

pub(super) struct PreparedSegmentPlan {
    pub(super) segment_stage_outputs: Value,
    pub(super) segment_trace: Option<VideoPromptGenerationTrace>,
    pub(super) segment_plan: Vec<VideoSegmentPlan>,
    pub(super) shooting_script_projection: VideoShootingScriptProjection,
    pub(super) source_shot_order: Vec<String>,
    pub(super) source_complete: bool,
}

pub(crate) struct ReusableSegmentPlan {
    pub(crate) segment_plan: Vec<VideoSegmentPlan>,
    pub(super) draft: NewChapterVideoPrompt,
}

pub(super) struct LoadedSharedSegmentPlan {
    pub(super) segment_plan: Vec<VideoSegmentPlan>,
    pub(super) execution_identity: Value,
}

pub(super) const SHARED_SEGMENT_PLAN_PROMPT_TYPE: &str = "frame_segment_plan";
pub(super) const SHARED_SEGMENT_PLAN_TITLE: &str = "视频片段规划";
pub(super) const SHARED_SEGMENT_PLAN_CONTEXT: &str = "视频分段规划";
pub(super) const SHARED_SEGMENT_PLAN_PROGRESS: &str = "正在规划视频片段";
pub(super) const SHARED_SEGMENT_PLAN_CONTRACT_VERSION: &str = FRAME_SEGMENT_PLAN_CONTRACT_VERSION;
