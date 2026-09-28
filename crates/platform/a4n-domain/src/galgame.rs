use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

pub mod pipeline;
pub mod validation;

pub use pipeline::GalgamePipelineStartStage;

pub const SCENE_ENTRY_MAIN: &str = "main";
pub const SCENE_ENTRY_BRANCH: &str = "branch";
pub const LINE_TYPE_NARRATION: &str = "narration";
pub const LINE_TYPE_DIALOGUE: &str = "dialogue";
pub const LINE_TYPE_MONOLOGUE: &str = "monologue";
pub const LINE_TYPE_STAGE_DIRECTION: &str = "stage_direction";
pub const LINE_TYPE_CHOICE: &str = "choice";
pub const GALGAME_TARGET_BACKGROUND: &str = "galgame_background";
pub const GALGAME_TARGET_CG: &str = "galgame_cg";
pub const GALGAME_TARGET_PORTRAIT_EXPRESSION: &str = "portrait_expression";
pub const GALGAME_ENTITY_SCENE: &str = "galgame_scene";
pub const GALGAME_ENTITY_LINE: &str = "galgame_line";
pub const GALGAME_ENTITY_CHARACTER_PROFILE: &str = "character_profile";
pub const GALGAME_EXPRESSIONS: &[&str] = &[
    "neutral",
    "smile",
    "sad",
    "angry",
    "surprised",
    "shy",
    "serious",
];
pub const GALGAME_PIPELINE_TOTAL_STEPS: u32 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameLineType {
    Dialogue,
    Narration,
    Monologue,
    StageDirection,
    Choice,
}

impl GalgameLineType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dialogue => LINE_TYPE_DIALOGUE,
            Self::Narration => LINE_TYPE_NARRATION,
            Self::Monologue => LINE_TYPE_MONOLOGUE,
            Self::StageDirection => LINE_TYPE_STAGE_DIRECTION,
            Self::Choice => LINE_TYPE_CHOICE,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            LINE_TYPE_DIALOGUE => Some(Self::Dialogue),
            LINE_TYPE_NARRATION => Some(Self::Narration),
            LINE_TYPE_MONOLOGUE => Some(Self::Monologue),
            LINE_TYPE_STAGE_DIRECTION => Some(Self::StageDirection),
            LINE_TYPE_CHOICE => Some(Self::Choice),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameLineType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameSceneEntryKind {
    Main,
    Branch,
}

impl GalgameSceneEntryKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Main => SCENE_ENTRY_MAIN,
            Self::Branch => SCENE_ENTRY_BRANCH,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            SCENE_ENTRY_MAIN => Some(Self::Main),
            SCENE_ENTRY_BRANCH => Some(Self::Branch),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameSceneEntryKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameMediaTargetKind {
    GalgameScene,
    GalgameLine,
}

impl GalgameMediaTargetKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GalgameScene => GALGAME_ENTITY_SCENE,
            Self::GalgameLine => GALGAME_ENTITY_LINE,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            GALGAME_ENTITY_SCENE => Some(Self::GalgameScene),
            GALGAME_ENTITY_LINE => Some(Self::GalgameLine),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameMediaTargetKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameMediaRole {
    Background,
    Cg,
    Bgm,
    Sfx,
}

impl GalgameMediaRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Cg => "cg",
            Self::Bgm => "bgm",
            Self::Sfx => "sfx",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "background" => Some(Self::Background),
            "cg" => Some(Self::Cg),
            "bgm" => Some(Self::Bgm),
            "sfx" => Some(Self::Sfx),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameMediaRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameAudioRole {
    Bgm,
    Sfx,
}

impl GalgameAudioRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bgm => "bgm",
            Self::Sfx => "sfx",
        }
    }
}

impl fmt::Display for GalgameAudioRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameGenerationTargetKind {
    GalgameBackground,
    GalgameCg,
    PortraitExpression,
}

impl GalgameGenerationTargetKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GalgameBackground => GALGAME_TARGET_BACKGROUND,
            Self::GalgameCg => GALGAME_TARGET_CG,
            Self::PortraitExpression => GALGAME_TARGET_PORTRAIT_EXPRESSION,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            GALGAME_TARGET_BACKGROUND => Some(Self::GalgameBackground),
            GALGAME_TARGET_CG => Some(Self::GalgameCg),
            GALGAME_TARGET_PORTRAIT_EXPRESSION => Some(Self::PortraitExpression),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameGenerationTargetKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameGenerationSpecUpsertInput {
    pub canonical_key: String,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: i64,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub target_entity_id: Option<i64>,
    pub params_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameAssetPlanSpecRecordInput {
    pub spec: GalgameGenerationSpecUpsertInput,
    pub text_anchor_ids: Vec<i64>,
    pub replace_entity_refs: bool,
    pub entity_profile_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalgamePromptArtifactUpsertInput {
    pub generation_spec_id: i64,
    pub prompt_kind: String,
    pub prompt_text: String,
    pub renderer_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameTargetEntityKind {
    GalgameScene,
    GalgameLine,
    CharacterProfile,
}

impl GalgameTargetEntityKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GalgameScene => GALGAME_ENTITY_SCENE,
            Self::GalgameLine => GALGAME_ENTITY_LINE,
            Self::CharacterProfile => GALGAME_ENTITY_CHARACTER_PROFILE,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            GALGAME_ENTITY_SCENE => Some(Self::GalgameScene),
            GALGAME_ENTITY_LINE => Some(Self::GalgameLine),
            GALGAME_ENTITY_CHARACTER_PROFILE => Some(Self::CharacterProfile),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameTargetEntityKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameCgImportance {
    Normal,
    High,
    Critical,
}

impl GalgameCgImportance {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "normal" => Some(Self::Normal),
            "high" => Some(Self::High),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameCgImportance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameTransition {
    Cut,
    Fade,
    Dissolve,
}

impl GalgameTransition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cut => "cut",
            Self::Fade => "fade",
            Self::Dissolve => "dissolve",
        }
    }
}

impl fmt::Display for GalgameTransition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum GalgameLinePresentationSchemaVersion {
    #[serde(rename = "galgame_line_presentation_v2")]
    #[schemars(rename = "galgame_line_presentation_v2")]
    V2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameStagePositionV2 {
    Left,
    Center,
    Right,
}

impl GalgameStagePositionV2 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "left" => Some(Self::Left),
            "center" => Some(Self::Center),
            "right" => Some(Self::Right),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameStagePositionV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameFacingV2 {
    Left,
    Right,
    Front,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameStageLayerV2 {
    Back,
    Middle,
    Front,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct GalgamePipelineBeatRefV4 {
    pub source_chapter_id: i64,
    pub unit_id: String,
    pub beat_id: String,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct GalgameAdaptationUnitRef {
    pub source_chapter_id: i64,
    pub unit_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum GalgameLineDocumentSchemaVersion {
    #[serde(rename = "galgame_line_presentation_v3")]
    V3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GalgameVisibleCastMemberV2 {
    pub character_profile_id: i64,
    pub expression: String,
    pub pose: String,
    pub position: GalgameStagePositionV2,
    pub facing: GalgameFacingV2,
    pub layer: GalgameStageLayerV2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GalgameLinePresentationV2 {
    pub schema_version: GalgameLinePresentationSchemaVersion,
    pub visible_cast: Vec<GalgameVisibleCastMemberV2>,
    pub enter: Vec<i64>,
    pub exit: Vec<i64>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub focus_character_profile_id: Option<i64>,
    pub hold: bool,
    pub player_knowledge: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GalgameLinePresentationDocumentV3 {
    pub schema_version: GalgameLineDocumentSchemaVersion,
    pub origin: crate::media_authoring::AdaptationOrigin,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub beat_ref: Option<GalgamePipelineBeatRefV4>,
    pub visible_cast: Vec<GalgameVisibleCastMemberV2>,
    pub enter: Vec<i64>,
    pub exit: Vec<i64>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub focus_character_profile_id: Option<i64>,
    pub hold: bool,
    pub player_knowledge: Vec<String>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub cg: Option<bool>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub cg_note: Option<String>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub cg_importance: Option<GalgameCgImportance>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub transition: Option<GalgameTransition>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub branch: Option<bool>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub branch_id: Option<String>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub returns_to_scene_index: Option<i32>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub after_beat_ref: Option<GalgamePipelineBeatRefV4>,
}

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use super::GalgameLinePresentationDocumentV3;
    use serde_json::json;

    #[test]
    fn presentation_document_requires_nullable_wire_fields() {
        let canonical = json!({
            "schema_version": "galgame_line_presentation_v3",
            "origin": "added", "beat_ref": null,
            "visible_cast": [],
            "enter": [],
            "exit": [],
            "focus_character_profile_id": null,
            "hold": false,
            "player_knowledge": [],
            "cg": null,
            "cg_note": null,
            "cg_importance": null,
            "transition": null,
            "branch": null,
            "branch_id": null,
            "returns_to_scene_index": null,
            "after_beat_ref": null
        });

        serde_json::from_value::<GalgameLinePresentationDocumentV3>(canonical.clone())
            .expect("canonical presentation");
        for field in [
            "origin",
            "beat_ref",
            "focus_character_profile_id",
            "cg",
            "cg_note",
            "cg_importance",
            "transition",
            "branch",
            "branch_id",
            "returns_to_scene_index",
            "after_beat_ref",
        ] {
            let mut missing = canonical.clone();
            missing
                .as_object_mut()
                .expect("presentation object")
                .remove(field);
            assert!(
                serde_json::from_value::<GalgameLinePresentationDocumentV3>(missing).is_err(),
                "missing {field} must fail"
            );
        }
    }
}
