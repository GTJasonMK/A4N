//! 漫画导演执行计划领域合同。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod contracts;
mod validation;

pub use contracts::*;
pub use validation::MangaPlanValidationError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MangaPipelineStartStage {
    CharacterBinding,
    SpeechSelection,
    VisualAnchorExtraction,
    ItemExtraction,
    ComicSynthesis,
    PagePlanning,
    DrawingPrompt,
}

impl MangaPipelineStartStage {
    pub const fn key(self) -> &'static str {
        match self {
            Self::CharacterBinding => "character_binding",
            Self::SpeechSelection => "speech_selection",
            Self::VisualAnchorExtraction => "visual_anchor_extraction",
            Self::ItemExtraction => "item_extraction",
            Self::ComicSynthesis => "comic_synthesis",
            Self::PagePlanning => "page_planning",
            Self::DrawingPrompt => "drawing_prompt",
        }
    }

    pub const fn order(self) -> i32 {
        match self {
            Self::CharacterBinding => 1,
            Self::SpeechSelection => 2,
            Self::VisualAnchorExtraction => 3,
            Self::ItemExtraction => 4,
            Self::ComicSynthesis => 5,
            Self::PagePlanning => 6,
            Self::DrawingPrompt => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MangaReferenceAssetKind {
    CharacterPortrait,
    GeneratedImage,
}
