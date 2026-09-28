use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct ExpectedVisualPlanDraft {
    pub record_id: i64,
    pub revision: i32,
}

#[derive(Debug, Clone)]
pub struct ConfirmChapterVisualPlanInput {
    pub record_id: i64,
    pub expected_revision: i32,
}

#[derive(Debug, Clone)]
pub struct DiscardChapterVisualPlanDraftInput {
    pub record_id: i64,
    pub expected_revision: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum ChapterVisualPlanSchemaVersion {
    #[serde(rename = "chapter_visual_plan_v3")]
    V3,
}

/// Shared provenance for user-authored facts in shooting scripts and derived media.
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
