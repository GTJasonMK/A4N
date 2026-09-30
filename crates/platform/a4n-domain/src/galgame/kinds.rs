use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
