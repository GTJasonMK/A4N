use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use a4n_domain::shooting_script::{
    ShootingCameraAngleV1, ShootingCameraHeightV1, ShootingCameraMovementV1, ShootingCoverageV1,
    ShootingDialogueCueV1, ShootingDirectingStatementV1, ShootingEditingModeV1, ShootingShotSizeV1,
    ShootingSoundDesignV1, ShootingSpatialContinuityV1, ShootingTransitionContractV1,
    ShootingTransitionTypeV1, ShootingViewpointV1,
};

pub const VIDEO_EXECUTION_CONTRACT_EPOCH: u32 = 5;
pub const VIDEO_EXECUTION_PROJECTION_V2: &str = "video_execution_projection_v2";
pub const VIDEO_SEGMENT_PLAN_V2: &str = "video_segment_plan_v2";
pub const VIDEO_EXECUTION_UNIT_V2: &str = "video_execution_unit_v2";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoShootingScriptIdentityV1 {
    pub record_id: i64,
    pub revision: i32,
    pub source_revision_key: String,
    pub source_content_sha256: String,
    pub script_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoExecutionProjectionV2 {
    pub schema_version: String,
    pub contract_epoch: u32,
    pub shooting_script_identity: VideoShootingScriptIdentityV1,
    pub directing_statement: ShootingDirectingStatementV1,
    pub scenes: Vec<VideoExecutionSceneV2>,
    pub planning_quality: VideoExecutionQualityV2,
}

impl VideoExecutionProjectionV2 {
    pub fn shots(&self) -> impl Iterator<Item = &VideoExecutionShotV2> {
        self.scenes.iter().flat_map(|scene| scene.shots.iter())
    }

    pub fn shot(&self, source_shot_id: &str) -> Option<&VideoExecutionShotV2> {
        self.shots()
            .find(|shot| shot.source_shot_id == source_shot_id)
    }

    pub fn shot_count(&self) -> usize {
        self.scenes.iter().map(|scene| scene.shots.len()).sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoExecutionSceneV2 {
    pub scene_index: u32,
    pub scene_identity: String,
    pub sequence_index: u32,
    pub narrative_unit_index: u32,
    pub timeline_position: u32,
    pub intercut_group_id: Option<String>,
    pub editing_mode: ShootingEditingModeV1,
    pub transition_contracts: Vec<ShootingTransitionContractV1>,
    pub title: String,
    pub slugline: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub participants: Vec<String>,
    pub focal_character: Option<String>,
    pub source_span_ids: Vec<String>,
    pub dramatic_question: String,
    pub scene_objective: String,
    pub scene_obstacle: String,
    pub relationship_before: String,
    pub relationship_after: String,
    pub audience_knowledge_before: Vec<String>,
    pub audience_knowledge_after: Vec<String>,
    pub withheld_information: Vec<String>,
    pub viewpoint: ShootingViewpointV1,
    pub spatial_continuity: ShootingSpatialContinuityV1,
    pub sound_strategy: String,
    pub entry_state: String,
    pub exit_state: String,
    pub shots: Vec<VideoExecutionShotV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoExecutionShotV2 {
    pub source_shot_id: String,
    pub shot_index: u32,
    pub scene_index: u32,
    pub narrative_unit_index: u32,
    pub timeline_position: u32,
    pub title: String,
    pub source_span_ids: Vec<String>,
    pub source_span_range: VideoSourceSpanRangeV1,
    pub coverage: ShootingCoverageV1,
    pub narrative_purpose: String,
    pub subject: String,
    pub participants: Vec<String>,
    pub focal_character: Option<String>,
    pub viewpoint: ShootingViewpointV1,
    pub editing_mode: ShootingEditingModeV1,
    pub intercut_group_id: Option<String>,
    pub transition_contracts: Vec<ShootingTransitionContractV1>,
    pub audience_knowledge_before: Vec<String>,
    pub audience_knowledge_after: Vec<String>,
    pub withheld_information: Vec<String>,
    pub relationship_before: String,
    pub relationship_after: String,
    pub shot_size: ShootingShotSizeV1,
    pub camera_angle: ShootingCameraAngleV1,
    pub camera_height: ShootingCameraHeightV1,
    pub lens_intent: String,
    pub camera_movement: ShootingCameraMovementV1,
    pub composition: String,
    pub spatial_anchors: Vec<String>,
    pub blocking_start: String,
    pub blocking_movement: String,
    pub blocking_end: String,
    pub dialogue_cues: Vec<ShootingDialogueCueV1>,
    pub sound: ShootingSoundDesignV1,
    pub camera_start_state: String,
    pub camera_end_state: String,
    pub start_state: String,
    pub end_state: String,
    pub transition_in: String,
    pub transition_out: String,
    pub continuity_requirements: Vec<String>,
    pub action_beats: Vec<VideoExecutionBeatV2>,
    pub estimated_natural_seconds: f64,
    pub segment_duration_secs: i32,
    pub recommended_segment_count: i32,
    pub minimum_readable_segment_count: i32,
    pub action_load: VideoActionLoadV1,
    pub risk_flags: Vec<VideoExecutionRiskV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoExecutionBeatV2 {
    pub beat_id: String,
    pub source_span_ids: Vec<String>,
    pub actor: Option<String>,
    pub objective: String,
    pub obstacle: String,
    pub visible_action: String,
    pub reaction: Option<String>,
    pub expression: Option<String>,
    pub gaze: Option<String>,
    pub pose_or_movement: Option<String>,
    pub dialogue_intent: Option<String>,
    pub audience_question: String,
    pub answer_or_delay: String,
    pub knowledge_delta: Vec<String>,
    pub relationship_delta: Option<String>,
    pub spatial_change: Option<String>,
    pub entry_state: String,
    pub exit_state: String,
    pub required: bool,
    pub natural_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoSourceSpanRangeV1 {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VideoActionLoadV1 {
    Light,
    Medium,
    Heavy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoExecutionQualityV2 {
    pub total_estimated_seconds: f64,
    pub total_recommended_segments: i32,
    pub total_minimum_readable_segments: i32,
    pub risk_flags: Vec<VideoExecutionRiskV1>,
    pub directing_audit: VideoDirectingAuditV2,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoDirectingAuditV2 {
    pub total_directing_beats: i32,
    pub must_show_beats: i32,
    pub directing_order_preserved: bool,
    pub viewpoint_metadata_complete: bool,
    pub audience_causality_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoExecutionRiskV1 {
    pub level: VideoExecutionRiskLevelV1,
    pub code: String,
    pub source_shot_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VideoExecutionRiskLevelV1 {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoSegmentPlanV2 {
    pub schema_version: String,
    pub segment_number: i32,
    pub source_shot_id: String,
    pub segment_index_in_shot: i32,
    pub shot_segment_count: i32,
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
    pub transition_to_next: Option<VideoSegmentTransitionV1>,
    pub source_span_ids: Vec<String>,
    pub duration_secs: i32,
    pub aspect_ratio: String,
    pub reference_image_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoSpatialAnchorV1 {
    pub anchor: String,
    pub usage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoEmotionActionGoalV1 {
    pub character: String,
    pub emotion: String,
    pub visible_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoIntentV1 {
    pub hook: String,
    pub audience_effect: String,
    pub rhythm: String,
    pub visual_priority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoEmotionalArcV1 {
    pub start: String,
    pub turn: String,
    pub end: String,
    pub function: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoPerformancePlanV2 {
    pub movement_path: String,
    pub body_performance: String,
    pub gaze_and_hands: String,
    pub pause_or_reaction: String,
    pub camera_relationship: String,
    pub time_blocks: Vec<VideoPerformanceTimeBlockV1>,
    pub action_script: Vec<VideoPerformanceActionV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoPerformanceTimeBlockV1 {
    pub start_sec: f64,
    pub end_sec: f64,
    pub camera: String,
    pub subject_action: String,
    pub continuity_target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoPerformanceActionV1 {
    pub character: String,
    pub body: String,
    pub movement: String,
    pub hands: String,
    pub gaze: String,
    pub timing: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoMaterialCoverageV1 {
    pub required_entities: Vec<String>,
    pub missing_or_weak_references: Vec<String>,
    pub risk_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoSegmentTransitionV1 {
    pub transition_type: ShootingTransitionTypeV1,
    pub editing_mode: ShootingEditingModeV1,
    pub preserved_action: Option<String>,
    pub preserved_sound: Option<String>,
    pub required_cut_point: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoExecutionUnitV2 {
    pub schema_version: String,
    pub unit_number: i32,
    pub duration_secs: i32,
    pub start_frame_number: Option<i32>,
    pub end_frame_number: Option<i32>,
    pub storyboard_number: Option<i32>,
    pub kind: VideoExecutionUnitKindV2,
    pub prompt_segment: VideoSegmentPlanV2,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "unit_type", rename_all = "snake_case")]
pub enum VideoExecutionUnitKindV2 {
    Content(VideoContentUnitV2),
    ActionBridge(VideoActionBridgeUnitV1),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoContentUnitV2 {
    pub source_segment_number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoActionBridgeUnitV1 {
    pub from_segment_number: i32,
    pub to_segment_number: i32,
    pub preserved_action: String,
    pub preserved_sound: Option<String>,
    pub required_cut_point: Option<String>,
}

impl VideoExecutionUnitV2 {
    pub fn source_segment_numbers(&self) -> Vec<i32> {
        match &self.kind {
            VideoExecutionUnitKindV2::Content(content) => vec![content.source_segment_number],
            VideoExecutionUnitKindV2::ActionBridge(bridge) => {
                vec![bridge.from_segment_number, bridge.to_segment_number]
            }
        }
    }

    pub fn is_action_bridge(&self) -> bool {
        matches!(self.kind, VideoExecutionUnitKindV2::ActionBridge(_))
    }
}
