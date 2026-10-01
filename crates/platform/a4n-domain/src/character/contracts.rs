use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterProfileCreateInput {
    pub project_id: i64,
    pub name: String,
    pub base_attributes: Value,
    pub current_state: Value,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub background: Option<String>,
    pub personality: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterPortraitCreateInput {
    pub profile_id: i64,
    pub project_id: i64,
    pub portrait_type: String,
    pub image_path: String,
    pub prompt: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterSyncChapterStateRecordInput {
    pub chapter_number: i32,
    pub state_summary: String,
    pub current_location: Option<String>,
    pub current_goal: Option<String>,
    pub emotional_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterAttributeValueChangeInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub category: String,
    pub key: String,
    pub value: Value,
    pub change_description: Option<String>,
    pub cause: String,
    pub evidence: String,
}

#[derive(Debug, Clone)]
pub struct CharacterAttributeDeletionRequestInput {
    pub profile_id: i64,
    pub category: String,
    pub key: String,
    pub chapter_number: i32,
    pub reason: String,
    pub evidence: String,
}

#[derive(Debug, Clone)]
pub struct CharacterAttributeDeletionExecutionInput {
    pub profile_id: i64,
    pub category: String,
    pub key: String,
    pub event_cause: String,
    pub evidence: String,
    pub chapter_number: i32,
}

#[derive(Debug, Clone)]
pub struct CharacterImplicitReviewDecisionRecordInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub attribute_key: String,
    pub current_value: Value,
    pub decision: String,
    pub suggested_new_value: Option<Value>,
    pub reasoning: String,
    pub evidence_summary: String,
}

#[derive(Debug, Clone)]
pub struct InsertBehaviorRecordInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub source_kind: String,
    pub behavior_type: String,
    pub behavior_description: String,
    pub original_text: String,
    pub behavior_tags: Value,
    pub classification_results: Value,
    pub related_characters: Option<Value>,
    pub impact_analysis: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ForeshadowingStatus {
    Pending,
    PartiallyRevealed,
    Resolved,
    Abandoned,
}

impl ForeshadowingStatus {
    pub const ALL: [Self; 4] = [
        Self::Pending,
        Self::PartiallyRevealed,
        Self::Resolved,
        Self::Abandoned,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::PartiallyRevealed => "partially_revealed",
            Self::Resolved => "resolved",
            Self::Abandoned => "abandoned",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }

    pub const fn can_resolve(self) -> bool {
        matches!(self, Self::Pending | Self::PartiallyRevealed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ForeshadowingRecordInput {
    pub description: String,
    #[schemars(range(min = 1))]
    pub setup_chapter: i32,
    #[schemars(range(min = 1))]
    #[serde(default)]
    pub payoff_chapter: Option<i32>,
    pub status: ForeshadowingStatus,
    #[serde(default)]
    pub related_characters: Vec<String>,
    #[schemars(range(min = 1, max = 5))]
    pub importance: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CharacterAttributeCategory {
    Explicit,
    Implicit,
    Social,
}

impl CharacterAttributeCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::Implicit => "implicit",
            Self::Social => "social",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "explicit" => Some(Self::Explicit),
            "implicit" => Some(Self::Implicit),
            "social" => Some(Self::Social),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CharacterAttributeOperation {
    Add,
    Modify,
    Delete,
}

impl CharacterAttributeOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Modify => "modify",
            Self::Delete => "delete",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "add" => Some(Self::Add),
            "modify" => Some(Self::Modify),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CharacterTimelineSourceKind {
    User,
    AiSync,
    TextFact,
    LegacyUnknown,
}

impl CharacterTimelineSourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::AiSync => "ai_sync",
            Self::TextFact => "text_fact",
            Self::LegacyUnknown => "legacy_unknown",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Self::User),
            "ai_sync" => Some(Self::AiSync),
            "text_fact" => Some(Self::TextFact),
            "legacy_unknown" => Some(Self::LegacyUnknown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeletionMarkStatus {
    All,
    Pending,
    Ready,
    Executed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImplicitUpdateDecision {
    Keep,
    Modify,
    Delete,
}

impl ImplicitUpdateDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Modify => "modify",
            Self::Delete => "delete",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "keep" => Some(Self::Keep),
            "modify" => Some(Self::Modify),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AppliedImplicitUpdateAction {
    Keep,
    Modify,
    MarkDeletion,
}
