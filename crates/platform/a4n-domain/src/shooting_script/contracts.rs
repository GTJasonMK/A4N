//! 拍摄剧本唯一结构化事实合同。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::chapter_visual_plan::{VisualSegmentProjectionV3, VisualUserAdditionV1};

pub const CHAPTER_SHOOTING_SCRIPT_V1: &str = "chapter_shooting_script_v1";

#[derive(Debug, Clone)]
pub struct EditStoredChapterShootingScriptInput {
    pub record_id: i64,
    pub expected_revision: i32,
    pub script: ChapterShootingScriptV1,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum ChapterShootingScriptSchemaVersion {
    #[serde(rename = "chapter_shooting_script_v1")]
    V1,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChapterShootingScriptV1 {
    pub schema_version: ChapterShootingScriptSchemaVersion,
    #[serde(default)]
    pub visual_plan_source: Option<ShootingVisualPlanSourceV1>,
    #[serde(default)]
    pub user_additions: Vec<VisualUserAdditionV1>,
    pub directing_statement: ShootingDirectingStatementV1,
    pub scenes: Vec<ShootingSceneV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShootingVisualPlanSourceV1 {
    pub plan_id: i64,
    pub plan_version: i32,
    pub plan_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShootingDirectingStatementV1 {
    pub central_dramatic_question: String,
    pub protagonist_throughline: String,
    pub audience_experience: String,
    pub viewpoint_strategy: String,
    pub visual_strategy: String,
    pub spatial_strategy: String,
    pub sound_strategy: String,
    pub rhythm_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShootingSceneV1 {
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
    #[serde(default)]
    pub visual_narrative_unit_id: Option<String>,
    #[serde(default)]
    pub user_addition_ids: Vec<String>,
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
    pub shots: Vec<ShootingShotV1>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShootingEditingModeV1 {
    Continuity,
    Intercut,
    Montage,
    Flashback,
    ParallelAction,
    SoundBridge,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShootingTransitionTypeV1 {
    Continuous,
    HardCut,
    MatchCut,
    WhipPan,
    ActionWipe,
    SoundBridge,
    Dissolve,
    Fade,
    FocusTransition,
    Morph,
}

impl ShootingTransitionTypeV1 {
    pub const VALUES: &'static [&'static str] = &[
        "continuous",
        "hard_cut",
        "match_cut",
        "whip_pan",
        "action_wipe",
        "sound_bridge",
        "dissolve",
        "fade",
        "focus_transition",
        "morph",
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Continuous => "continuous",
            Self::HardCut => "hard_cut",
            Self::MatchCut => "match_cut",
            Self::WhipPan => "whip_pan",
            Self::ActionWipe => "action_wipe",
            Self::SoundBridge => "sound_bridge",
            Self::Dissolve => "dissolve",
            Self::Fade => "fade",
            Self::FocusTransition => "focus_transition",
            Self::Morph => "morph",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShootingTransitionContractV1 {
    pub to_narrative_unit_index: u32,
    pub transition_type: ShootingTransitionTypeV1,
    pub preserved_action: Option<String>,
    pub preserved_sound: Option<String>,
    pub preserved_knowledge: Vec<String>,
    pub required_cut_point: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShootingViewpointV1 {
    pub mode: ShootingViewpointModeV1,
    pub anchor_character: Option<String>,
    pub information_strategy: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShootingViewpointModeV1 {
    DirectSubjective,
    ModifiedSubjective,
    LimitedThirdPerson,
    OmniscientObjective,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShootingSpatialContinuityV1 {
    pub continuity_intent: ShootingContinuityIntentV1,
    pub primary_axis: Option<String>,
    pub screen_direction: Option<String>,
    pub reorientation_strategy: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShootingContinuityIntentV1 {
    Transparent,
    DeliberateDisorientation,
    OpenAmbiguity,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShootingShotV1 {
    pub shot_index: u32,
    pub shot_id: String,
    pub title: String,
    pub source_span_ids: Vec<String>,
    #[serde(default)]
    pub visual_shot_id: Option<String>,
    #[serde(default)]
    pub planned_start_ms: Option<u64>,
    #[serde(default)]
    pub planned_end_ms: Option<u64>,
    #[serde(default)]
    pub user_addition_ids: Vec<String>,
    #[serde(default)]
    pub visual_constraints: Option<ShootingVisualConstraintsV3>,
    pub coverage: ShootingCoverageV1,
    pub narrative_purpose: String,
    pub subject: String,
    pub shot_size: ShootingShotSizeV1,
    pub camera_angle: ShootingCameraAngleV1,
    pub camera_height: ShootingCameraHeightV1,
    pub lens_intent: String,
    pub camera_movement: ShootingCameraMovementV1,
    pub composition: String,
    pub spatial_anchors: Vec<String>,
    pub blocking: ShootingBlockingV1,
    pub performance_beats: Vec<ShootingPerformanceBeatV1>,
    pub dialogue_cues: Vec<ShootingDialogueCueV1>,
    pub sound: ShootingSoundDesignV1,
    pub camera_start_state: String,
    pub camera_end_state: String,
    pub start_state: String,
    pub end_state: String,
    pub transition_in: String,
    pub transition_out: String,
    pub continuity_requirements: Vec<String>,
    pub estimated_natural_seconds: f64,
}

pub type ShootingVisualConstraintsV3 = VisualSegmentProjectionV3;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShootingBlockingV1 {
    pub start: String,
    pub movement: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShootingPerformanceBeatV1 {
    pub beat_index: u32,
    pub beat_id: String,
    pub source_span_ids: Vec<String>,
    #[serde(default)]
    pub user_addition_ids: Vec<String>,
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
    pub estimated_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShootingDialogueCueV1 {
    pub beat_id: String,
    pub speaker: String,
    pub line_or_intent: String,
    pub listener_reaction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShootingSoundDesignV1 {
    pub dialogue: Vec<String>,
    pub environment: Vec<String>,
    pub effects: Vec<String>,
    pub music: Vec<String>,
    pub silence: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShootingCoverageV1 {
    MustShow,
    ShouldShow,
    Optional,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShootingShotSizeV1 {
    ExtremeWide,
    Wide,
    MediumWide,
    Medium,
    MediumCloseUp,
    CloseUp,
    ExtremeCloseUp,
    Insert,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShootingCameraAngleV1 {
    EyeLevel,
    High,
    Low,
    Overhead,
    Dutch,
    Subjective,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShootingCameraHeightV1 {
    Ground,
    Low,
    Eye,
    High,
    Overhead,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShootingCameraMovementV1 {
    Locked,
    Pan,
    Tilt,
    PushIn,
    PullOut,
    Track,
    Dolly,
    Crane,
    Handheld,
    Steadicam,
    WhipPan,
}
