//! 漫画画格合同。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::text::MangaTextLayerV2;
use crate::media_authoring::AdaptationBeatRef;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MangaPanelV2 {
    pub panel_number: u32,
    pub beat_refs: Vec<AdaptationBeatRef>,
    pub function: MangaPanelFunctionV2,
    pub size: MangaPanelSizeV2,
    pub shot_scale: MangaShotScaleV2,
    pub camera_angle: MangaCameraAngleV2,
    pub participants: Vec<String>,
    pub composition: String,
    pub observable_action: String,
    pub reaction: Option<String>,
    pub spatial_relationship: String,
    pub continuity_from_previous: Option<String>,
    pub text_layers: Vec<MangaTextLayerV2>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MangaPanelFunctionV2 {
    Establish,
    Action,
    Dialogue,
    Reaction,
    Revelation,
    Transition,
    Hold,
    Resolution,
    Detail,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MangaPanelSizeV2 {
    Dominant,
    Large,
    Medium,
    Small,
    Inset,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MangaShotScaleV2 {
    ExtremeWide,
    Wide,
    MediumWide,
    Medium,
    MediumCloseUp,
    CloseUp,
    ExtremeCloseUp,
    Insert,
    Detail,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MangaCameraAngleV2 {
    EyeLevel,
    High,
    Low,
    TopDown,
    Overhead,
    Dutch,
    Subjective,
    OverShoulder,
    Profile,
}

impl MangaCameraAngleV2 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EyeLevel => "eye_level",
            Self::High => "high",
            Self::Low => "low",
            Self::TopDown => "top_down",
            Self::Overhead => "overhead",
            Self::Dutch => "dutch",
            Self::Subjective => "subjective",
            Self::OverShoulder => "over_shoulder",
            Self::Profile => "profile",
        }
    }
}
