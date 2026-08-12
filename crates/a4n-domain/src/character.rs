use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CHARACTER_DELETION_THRESHOLD: i32 = 5;

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
    LegacyUnknown,
}

impl CharacterTimelineSourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::AiSync => "ai_sync",
            Self::LegacyUnknown => "legacy_unknown",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Self::User),
            "ai_sync" => Some(Self::AiSync),
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
