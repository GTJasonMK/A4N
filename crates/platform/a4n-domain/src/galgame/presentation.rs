use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use super::{GalgameCgImportance, GalgamePipelineBeatRefV4, GalgameTransition};

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
