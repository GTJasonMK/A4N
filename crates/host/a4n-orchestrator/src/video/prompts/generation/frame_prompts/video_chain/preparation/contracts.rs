use std::collections::{HashMap, HashSet};

use a4n_domain::prompt_language::PromptOutputLanguage;
use serde_json::Value;

use super::super::super::{
    ChapterVideoSourceContext, FramePromptGenerationSpec, OrchestratorContext, VideoFramePrompt,
    VideoPromptProgress, VideoSegmentPlan, VideoUnitPlan,
};

pub(super) struct VideoChainFrameSelectionInput<'a> {
    pub(super) service: &'a OrchestratorContext,
    pub(super) spec: &'a FramePromptGenerationSpec,
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) segment_plan: &'a [VideoSegmentPlan],
    pub(super) target_shot_number: Option<i32>,
    pub(super) duration_secs: i32,
    pub(super) execution_identity: &'a Value,
    pub(super) prompt_output_language: PromptOutputLanguage,
    pub(super) available_assets: &'a Value,
    pub(super) reference_image_ids: &'a [i64],
    pub(super) include_system_frame_images: bool,
}

pub(super) struct SelectedVideoChainFrames {
    pub(super) frame_prompts: Vec<VideoFramePrompt>,
    pub(super) prompt_video_units: Vec<VideoUnitPlan>,
    pub(super) all_video_units: Vec<VideoUnitPlan>,
    pub(super) frame_image_ids_by_number: HashMap<i32, i64>,
}

pub(super) struct VideoChainAssetFinalizationInput<'a, 'source, 'progress> {
    pub(super) service: &'a OrchestratorContext,
    pub(super) spec: &'a FramePromptGenerationSpec,
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) source: &'a ChapterVideoSourceContext<'source>,
    pub(super) reference_image_ids: Vec<i64>,
    pub(super) video_library_image_ids: Vec<i64>,
    pub(super) vars: HashMap<String, Value>,
    pub(super) selected_frames: SelectedVideoChainFrames,
    pub(super) progress: &'a mut Option<VideoPromptProgress<'progress>>,
}

pub(super) struct FinalizedVideoChainAssets {
    pub(super) reference_ids: Value,
    pub(super) available_assets: Value,
    pub(super) generation_asset_fingerprint: Value,
    pub(super) vars: HashMap<String, Value>,
    pub(super) allowed_reference_ids: HashSet<i64>,
    pub(super) all_video_units: Vec<VideoUnitPlan>,
    pub(super) frame_json: Value,
    pub(super) frame_by_number: HashMap<i32, VideoFramePrompt>,
    pub(super) video_segment_plan: Vec<VideoSegmentPlan>,
    pub(super) video_segment_json: Value,
    pub(super) video_unit_by_number: HashMap<i32, VideoUnitPlan>,
}
