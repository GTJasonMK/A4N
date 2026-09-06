use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::chapter_visual_plan::{
    ChapterVisualPlanSchemaVersion, VisualCameraEventKindV1, VisualCameraStateV1,
    VisualEntityEventKindV1, VisualEntityKindV1, VisualEntityStateV1, VisualInterpolationV1,
    VisualRelationV1, VisualSoundEventKindV1, VisualTechniqueKindV1,
};
use crate::shooting_script::{
    ShootingCameraAngleV1, ShootingCameraHeightV1, ShootingCoverageV1, ShootingEditingModeV1,
    ShootingShotSizeV1, ShootingTransitionTypeV1,
};

pub const CHAPTER_VISUAL_PLAN_V2: &str = "chapter_visual_plan_v2";

/// V2 是用户编辑的唯一视觉事实。正文只作为生成和投影时的只读上下文。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChapterVisualPlanV2 {
    pub schema_version: ChapterVisualPlanSchemaVersion,
    pub frame: VisualFrameV2,
    pub entities: Vec<VisualEntityV2>,
    pub scenes: Vec<VisualSceneV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualFrameV2 {
    pub width: u32,
    pub height: u32,
    pub background: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualEntityV2 {
    pub id: String,
    pub label: String,
    pub kind: VisualEntityKindV1,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VisualSceneV2 {
    pub id: String,
    pub title: String,
    pub narrative_purpose: String,
    pub editing_mode: ShootingEditingModeV1,
    pub intercut_group_id: Option<String>,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub participant_entity_ids: Vec<String>,
    pub focal_entity_id: Option<String>,
    pub shots: Vec<VisualShotV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VisualShotV2 {
    pub id: String,
    pub title: String,
    pub duration_ms: u64,
    pub narrative_purpose: String,
    pub coverage: ShootingCoverageV1,
    pub shot_size: Option<ShootingShotSizeV1>,
    pub camera_angle: Option<ShootingCameraAngleV1>,
    pub camera_height: Option<ShootingCameraHeightV1>,
    pub lens_intent: Option<String>,
    pub composition: Option<String>,
    pub keyframes: Vec<VisualKeyframeV2>,
    pub entity_events: Vec<VisualEntityEventV2>,
    pub camera_events: Vec<VisualCameraEventV2>,
    pub sound_events: Vec<VisualSoundEventV2>,
    pub technique_events: Vec<VisualTechniqueEventV2>,
    pub continuity_notes: Vec<String>,
    pub transition_to_next: Option<VisualTransitionV2>,
}

/// 关键帧时间 at_ms 相对于所在镜头起点。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VisualKeyframeV2 {
    pub id: String,
    pub at_ms: u64,
    pub label: String,
    pub interpolation_to_next: VisualInterpolationV1,
    pub camera: VisualCameraStateV1,
    pub entity_states: Vec<VisualEntityStateV1>,
    pub relations: Vec<VisualRelationV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualEntityEventV2 {
    pub id: String,
    pub kind: VisualEntityEventKindV1,
    pub start_ms: u64,
    pub end_ms: u64,
    pub entity_id: String,
    pub target_entity_id: Option<String>,
    pub description: String,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualCameraEventV2 {
    pub id: String,
    pub kind: VisualCameraEventKindV1,
    pub start_ms: u64,
    pub end_ms: u64,
    pub target_entity_id: Option<String>,
    pub description: String,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualSoundEventV2 {
    pub id: String,
    pub kind: VisualSoundEventKindV1,
    pub start_ms: u64,
    pub end_ms: u64,
    pub speaker_entity_id: Option<String>,
    pub content: String,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualTechniqueEventV2 {
    pub id: String,
    pub kind: VisualTechniqueKindV1,
    pub start_ms: u64,
    pub end_ms: u64,
    pub description: String,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualTransitionV2 {
    pub duration_ms: u64,
    pub kind: ShootingTransitionTypeV1,
    pub preserved_action: Option<String>,
    pub preserved_sound: Option<String>,
    pub notes: Option<String>,
}
