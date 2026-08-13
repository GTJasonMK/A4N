use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use a4n_core::video_execution::{
    VideoEmotionActionGoalV1, VideoEmotionalArcV1, VideoIntentV1, VideoMaterialCoverageV1,
    VideoPerformancePlanV2, VideoSpatialAnchorV1,
};
pub(in crate::video) use a4n_core::video_execution::{
    VideoExecutionProjectionV2 as VideoShootingScriptProjection,
    VideoExecutionShotV2 as VideoCinematicShot, VideoSegmentPlanV2 as VideoSegmentPlan,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(in crate::video) struct VideoPromptShot {
    pub shot_number: i32,
    pub title: String,
    pub scene_summary: String,
    pub self_contained_context: Value,
    pub start_state: Value,
    pub end_state: Value,
    pub transition_in: Value,
    pub visual_prompt: String,
    pub motion_prompt: String,
    pub negative_prompt: String,
    pub blocking_plan: Value,
    pub lighting_plan: Value,
    pub emotion_action_beats: Value,
    pub dialogue_beats: Value,
    pub cinematography_plan: Value,
    pub temporal_beats: Value,
    pub reference_binding: Value,
    pub continuity_check: Value,
    pub duration_secs: i32,
    pub aspect_ratio: String,
    pub reference_image_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(in crate::video) struct VideoFramePrompt {
    pub frame_number: i32,
    pub title: String,
    pub scene_summary: String,
    pub storyboard_panels: Vec<StoryboardPanel>,
    pub frame_prompt: String,
    pub blocking_state: Value,
    pub lighting_state: Value,
    pub negative_prompt: String,
    pub duration_secs: i32,
    pub aspect_ratio: String,
    pub reference_image_ids: Vec<i64>,
    pub segment_numbers: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::video) struct StoryboardPanel {
    pub panel_index: i32,
    pub moment: String,
    pub visual_goal: String,
    pub composition: String,
    pub continuity_note: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::video) struct VideoPromptShotInput {
    pub title: String,
    pub scene_summary: String,
    pub visual_prompt: String,
    pub motion_prompt: String,
    pub negative_prompt: String,
    pub dialogue_cues: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::video) struct InsertedSegmentDraft {
    pub title: String,
    pub scene_summary: String,
    pub visual_prompt: String,
    pub motion_prompt: String,
    pub negative_prompt: String,
}

impl InsertedSegmentDraft {
    pub(in crate::video) fn validate(
        self,
        prompt_output_language: a4n_core::prompt_language::PromptOutputLanguage,
    ) -> Result<Self, a4n_core::error::AppError> {
        super::validation::validate_inserted_segment_draft(self, prompt_output_language)
    }
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::video) struct VideoSegmentPlanInput {
    pub source_shot_id: String,
    pub covered_beat_ids: Vec<String>,
    pub title: String,
    pub scene_summary: String,
    pub blocking_summary: String,
    pub spatial_anchors: Vec<VideoSpatialAnchorV1>,
    pub emotion_action_goals: Vec<VideoEmotionActionGoalV1>,
    pub video_intent: VideoIntentV1,
    pub emotional_arc: VideoEmotionalArcV1,
    pub performance_plan: VideoPerformancePlanV2,
    pub material_coverage: VideoMaterialCoverageV1,
}

impl From<&VideoSegmentPlan> for VideoSegmentPlanInput {
    fn from(segment: &VideoSegmentPlan) -> Self {
        Self {
            source_shot_id: segment.source_shot_id.clone(),
            covered_beat_ids: segment.covered_beat_ids.clone(),
            title: segment.title.clone(),
            scene_summary: segment.scene_summary.clone(),
            blocking_summary: segment.blocking_summary.clone(),
            spatial_anchors: segment.spatial_anchors.clone(),
            emotion_action_goals: segment.emotion_action_goals.clone(),
            video_intent: segment.video_intent.clone(),
            emotional_arc: segment.emotional_arc.clone(),
            performance_plan: segment.performance_plan.clone(),
            material_coverage: segment.material_coverage.clone(),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::video) struct StaticVideoFramePromptInput {
    pub title: String,
    pub scene_summary: String,
    pub frame_prompt: String,
    pub blocking_state: Value,
    pub lighting_state: Value,
    pub negative_prompt: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::video) struct StoryboardVideoFramePromptInput {
    pub title: String,
    pub scene_summary: String,
    pub storyboard_panels: Vec<StoryboardPanel>,
    pub frame_prompt: String,
    pub blocking_state: Value,
    pub lighting_state: Value,
    pub negative_prompt: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::video) struct VideoPromptResponse {
    pub(in crate::video) shots: Vec<VideoPromptShotInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::video) struct VideoSegmentPlanResponse {
    pub(in crate::video) segments: Vec<VideoSegmentPlanInput>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::video) struct StoredVideoSegmentPlanResponse {
    pub(in crate::video) segments: Vec<VideoSegmentPlan>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::video) struct StaticVideoFramePromptResponse {
    pub(in crate::video) frames: Vec<StaticVideoFramePromptInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::video) struct StoryboardVideoFramePromptResponse {
    pub(in crate::video) frames: Vec<StoryboardVideoFramePromptInput>,
}

pub(in crate::video) struct VideoFramePromptDefaults<'a> {
    pub frame_number: i32,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub segment_numbers: Vec<i32>,
    pub reference_image_ids: Vec<i64>,
}

pub(in crate::video) struct VideoSegmentPlanDefaults<'a> {
    pub(in crate::video) shooting_script_projection: &'a VideoShootingScriptProjection,
    pub(in crate::video) duration_secs: i32,
    pub(in crate::video) aspect_ratio: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::video) struct VideoPromptTargetedRepairResponse {
    pub schema_version: String,
    pub repairs: VideoPromptTargetedRepairFields,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::video) struct VideoPromptTargetedRepairFields {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialogue_cues: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_prompt: Option<String>,
}
