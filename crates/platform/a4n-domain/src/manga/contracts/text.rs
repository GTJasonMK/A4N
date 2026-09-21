use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MangaTextLayerV2 {
    pub layer_index: u32,
    pub kind: MangaTextLayerKindV2,
    pub speaker: Option<String>,
    pub text: String,
    pub placement: MangaTextPlacementV2,
    pub avoid_covering: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MangaTextLayerKindV2 {
    Dialogue,
    Thought,
    Narration,
    Sfx,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MangaTextPlacementV2 {
    TopLeft,
    TopRight,
    UpperCenter,
    CenterLeft,
    CenterRight,
    BottomLeft,
    BottomRight,
    LowerCenter,
}
