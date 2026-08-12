use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CHARACTER_DELETION_THRESHOLD: i32 = 5;

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

#[cfg(test)]
mod tests {
    use super::ForeshadowingStatus;

    #[test]
    fn foreshadowing_status_accepts_only_canonical_values() {
        assert_eq!(
            ForeshadowingStatus::parse("partially_revealed"),
            Some(ForeshadowingStatus::PartiallyRevealed)
        );
        assert!(ForeshadowingStatus::Pending.can_resolve());
        assert!(!ForeshadowingStatus::Resolved.can_resolve());
        assert_eq!(ForeshadowingStatus::parse(" pending "), None);
        assert_eq!(ForeshadowingStatus::parse("legacy_pending"), None);
    }
}
