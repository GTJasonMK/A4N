use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::shooting_script::{
    ShootingCameraAngleV1, ShootingCameraHeightV1, ShootingCoverageV1,
    ShootingDirectingStatementV1, ShootingEditingModeV1, ShootingShotSizeV1,
    ShootingTransitionTypeV1,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChapterVisualPlanStatus {
    Draft,
    Confirmed,
    Superseded,
    Stale,
}

impl ChapterVisualPlanStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Confirmed => "confirmed",
            Self::Superseded => "superseded",
            Self::Stale => "stale",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChapterVisualPlanAuthoringMode {
    Collaborative,
    Manual,
}

impl ChapterVisualPlanAuthoringMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Collaborative => "collaborative",
            Self::Manual => "manual",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum ChapterVisualPlanSchemaVersion {
    #[serde(rename = "chapter_visual_plan_v2")]
    V2,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChapterVisualPlanV1 {
    pub frame: VisualFrameV1,
    pub directing_statement: ShootingDirectingStatementV1,
    pub entities: Vec<VisualEntityV1>,
    pub user_additions: Vec<VisualUserAdditionV1>,
    pub narrative_units: Vec<VisualNarrativeUnitV1>,
    pub transitions: Vec<VisualTransitionV1>,
    pub source_dispositions: Vec<SourceSpanDispositionV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualFrameV1 {
    pub width: u32,
    pub height: u32,
    pub duration_ms: u64,
    pub background: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisualEntityKindV1 {
    Character,
    Object,
    Environment,
    Text,
    Effect,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualEntityV1 {
    pub id: String,
    pub label: String,
    pub kind: VisualEntityKindV1,
    pub description: String,
    pub source_span_ids: Vec<String>,
    pub user_addition_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisualUserAdditionKindV1 {
    Entity,
    Dialogue,
    Action,
    Location,
    StoryFact,
    VisualMotif,
    Sound,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualUserAdditionV1 {
    pub id: String,
    pub kind: VisualUserAdditionKindV1,
    pub description: String,
    pub rationale: String,
    pub introduced_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VisualNarrativeUnitV1 {
    pub id: String,
    pub sequence_index: u32,
    pub timeline_position: u32,
    pub title: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub editing_mode: ShootingEditingModeV1,
    pub intercut_group_id: Option<String>,
    pub narrative_purpose: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub participant_entity_ids: Vec<String>,
    pub focal_entity_id: Option<String>,
    pub source_span_ids: Vec<String>,
    pub user_addition_ids: Vec<String>,
    pub shots: Vec<VisualShotPlanV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VisualShotPlanV1 {
    pub id: String,
    pub title: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub source_span_ids: Vec<String>,
    pub user_addition_ids: Vec<String>,
    pub narrative_purpose: String,
    pub coverage: ShootingCoverageV1,
    pub shot_size: Option<ShootingShotSizeV1>,
    pub camera_angle: Option<ShootingCameraAngleV1>,
    pub camera_height: Option<ShootingCameraHeightV1>,
    pub lens_intent: Option<String>,
    pub composition: Option<String>,
    pub keyframes: Vec<VisualKeyframeV1>,
    pub entity_events: Vec<VisualEntityEventV1>,
    pub camera_events: Vec<VisualCameraEventV1>,
    pub sound_events: Vec<VisualSoundEventV1>,
    pub technique_events: Vec<VisualTechniqueEventV1>,
    pub continuity_notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisualInterpolationV1 {
    Hold,
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VisualKeyframeV1 {
    pub id: String,
    pub at_ms: u64,
    pub label: String,
    pub interpolation_to_next: VisualInterpolationV1,
    pub camera: VisualCameraStateV1,
    pub entity_states: Vec<VisualEntityStateV1>,
    pub relations: Vec<VisualRelationV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VisualRectV1 {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisualFacingV1 {
    Left,
    Right,
    Front,
    Back,
    ProfileLeft,
    ProfileRight,
    Unspecified,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VisualEntityStateV1 {
    pub entity_id: String,
    pub visible: bool,
    pub bounds: VisualRectV1,
    pub rotation_degrees: f32,
    pub opacity: f32,
    pub z_index: i32,
    pub facing: VisualFacingV1,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub action: Option<String>,
    pub gaze_target_entity_id: Option<String>,
    pub dialogue: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VisualCameraStateV1 {
    pub center_x: f32,
    pub center_y: f32,
    pub zoom: f32,
    pub rotation_degrees: f32,
    pub focus_entity_id: Option<String>,
    pub framing_note: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisualRelationKindV1 {
    LeftOf,
    RightOf,
    Above,
    Below,
    InFrontOf,
    Behind,
    Inside,
    Overlapping,
    Facing,
    LookingAt,
    Touching,
    Holding,
    Following,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualRelationV1 {
    pub id: String,
    pub kind: VisualRelationKindV1,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisualEntityEventKindV1 {
    Enter,
    Exit,
    Move,
    Action,
    Dialogue,
    Expression,
    Gaze,
    Pose,
    Interaction,
    Transform,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualEntityEventV1 {
    pub id: String,
    pub kind: VisualEntityEventKindV1,
    pub start_ms: u64,
    pub end_ms: u64,
    pub entity_id: String,
    pub target_entity_id: Option<String>,
    pub description: String,
    pub source_span_ids: Vec<String>,
    pub user_addition_ids: Vec<String>,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisualCameraEventKindV1 {
    Cut,
    Pan,
    Tilt,
    PushIn,
    PullOut,
    Track,
    Dolly,
    Crane,
    Handheld,
    WhipPan,
    Zoom,
    Focus,
    Shake,
    Orbit,
    Reframe,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualCameraEventV1 {
    pub id: String,
    pub kind: VisualCameraEventKindV1,
    pub start_ms: u64,
    pub end_ms: u64,
    pub target_entity_id: Option<String>,
    pub description: String,
    pub source_span_ids: Vec<String>,
    pub user_addition_ids: Vec<String>,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisualSoundEventKindV1 {
    Dialogue,
    VoiceOver,
    Narration,
    Ambient,
    Effect,
    Music,
    Silence,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualSoundEventV1 {
    pub id: String,
    pub kind: VisualSoundEventKindV1,
    pub start_ms: u64,
    pub end_ms: u64,
    pub speaker_entity_id: Option<String>,
    pub content: String,
    pub source_span_ids: Vec<String>,
    pub user_addition_ids: Vec<String>,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisualTechniqueKindV1 {
    Montage,
    SplitScreen,
    SlowMotion,
    FastMotion,
    FreezeFrame,
    Flashback,
    Dream,
    Subjective,
    Symbolic,
    Silhouette,
    Reflection,
    Overlay,
    TextOnScreen,
    Lighting,
    ColorGrade,
    MatchAction,
    Offscreen,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualTechniqueEventV1 {
    pub id: String,
    pub kind: VisualTechniqueKindV1,
    pub start_ms: u64,
    pub end_ms: u64,
    pub description: String,
    pub source_span_ids: Vec<String>,
    pub user_addition_ids: Vec<String>,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualTransitionV1 {
    pub id: String,
    pub from_shot_id: String,
    pub to_shot_id: String,
    pub at_ms: u64,
    pub duration_ms: u64,
    pub kind: ShootingTransitionTypeV1,
    pub preserved_action: Option<String>,
    pub preserved_sound: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceSpanDispositionKindV1 {
    Adapt,
    Omit,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceSpanDispositionV1 {
    pub source_span_id: String,
    pub disposition: SourceSpanDispositionKindV1,
    pub narrative_unit_id: Option<String>,
    pub shot_ids: Vec<String>,
    pub reason: Option<String>,
}
