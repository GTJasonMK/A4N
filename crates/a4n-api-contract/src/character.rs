use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateProfileInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub name: String,
    pub base_attributes: Value,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateCharacterProfileInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    pub expected_current_state: Value,
    pub base_attributes: Value,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListProfilesInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SyncCharacterProfileInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1))]
    pub name: String,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
}

#[derive(Debug, Clone, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SyncAllCharacterProfilesInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
}

#[derive(Serialize, JsonSchema)]
pub struct CharacterProfileSummary {
    pub id: i64,
    pub name: String,
    pub current_state: Value,
    pub last_synced_chapter: i32,
}

#[derive(Serialize, JsonSchema)]
pub struct CharacterProfileCreateResult {
    pub profile_id: i64,
    pub warnings: Vec<String>,
}

#[derive(Serialize, JsonSchema)]
pub struct CharacterProfileMutationResult {
    pub profile: CharacterProfileSummary,
    pub warnings: Vec<String>,
}

#[derive(Serialize, JsonSchema)]
pub struct CharacterProfileDeleteResult {
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CharacterProfileSyncSkipReason {
    AlreadySyncedToTarget,
    AlreadySyncedPastTarget,
    TargetBeforeFirstSeen,
}

impl CharacterProfileSyncSkipReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AlreadySyncedToTarget => "already_synced_to_target",
            Self::AlreadySyncedPastTarget => "already_synced_past_target",
            Self::TargetBeforeFirstSeen => "target_before_first_seen",
        }
    }
}

impl fmt::Display for CharacterProfileSyncSkipReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct CharacterProfileSyncRange {
    pub from: i32,
    pub to: i32,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct CharacterProfileSyncedResult {
    pub name: String,
    pub chapters_analyzed: usize,
    pub sync_range: CharacterProfileSyncRange,
    pub synced_chapter: i32,
    pub changes_applied: usize,
    pub behaviors_recorded: usize,
    pub classification_suggestions_applied: usize,
    pub deletions_marked: usize,
    pub deletions_executed: usize,
    pub deleted_deletion_marks: u64,
    pub rewound_deletion_marks: u64,
    pub implicit_reviews_checked: usize,
    pub implicit_reviews_applied: usize,
    pub snapshot_hash: String,
    pub state_indexes_upserted: usize,
    pub deleted_state_rag: usize,
    pub state_rag_indexed_count: usize,
    pub state_rag_indexed: bool,
    pub state_rag_error: Option<String>,
    pub deleted_attribute_event_rag: usize,
    pub attribute_event_rag_indexed_count: usize,
    pub attribute_event_rag_indexed: bool,
    pub attribute_event_rag_error: Option<String>,
    pub llm_trace_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct CharacterProfileSkippedResult {
    pub name: String,
    pub synced_chapter: i32,
    pub target_chapter: i32,
    pub first_seen_chapter: Option<i32>,
    pub reason: CharacterProfileSyncSkipReason,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SyncCharacterProfileResult {
    Synced(CharacterProfileSyncedResult),
    Skipped(CharacterProfileSkippedResult),
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct CharacterProfileSyncFailure {
    pub name: String,
    pub error: String,
}

#[derive(Debug, Clone, Copy, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CharacterProfileSyncBatchStatus {
    Synced,
    Skipped,
    Failed,
    CompletedWithErrors,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct SyncAllCharacterProfilesResult {
    pub status: CharacterProfileSyncBatchStatus,
    pub target_chapter: i32,
    pub total: usize,
    pub synced: Vec<CharacterProfileSyncedResult>,
    pub skipped: Vec<CharacterProfileSkippedResult>,
    pub failed: Vec<CharacterProfileSyncFailure>,
}
