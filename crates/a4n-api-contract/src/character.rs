use a4n_domain::character::{
    AppliedImplicitUpdateAction, CharacterAttributeCategory, CharacterAttributeOperation,
    CharacterTimelineSourceKind, DeletionMarkStatus, ImplicitUpdateDecision,
};
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

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct AttributeCountsSummary {
    pub explicit: usize,
    pub implicit: usize,
    pub social: usize,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CharacterProfileSummary {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub base_attributes: Value,
    pub current_state: Value,
    pub character_arc: Option<Value>,
    pub voice_fingerprint: Option<Value>,
    pub appearance: Option<String>,
    pub background: Option<String>,
    pub personality: Option<String>,
    pub last_synced_chapter: i32,
    pub attribute_counts: AttributeCountsSummary,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CharacterProfileCreateResult {
    pub profile_id: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CharacterProfileMutationResult {
    pub profile: CharacterProfileSummary,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
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
pub struct CharacterProfileProtectedAttribute {
    pub chapter_number: i32,
    pub category: String,
    pub key: String,
    pub operation: String,
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
    pub protected_attributes: Vec<CharacterProfileProtectedAttribute>,
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

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddProfileAttributeInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    pub category: CharacterAttributeCategory,
    #[schemars(length(min = 1))]
    pub key: String,
    pub value: Value,
    #[schemars(length(min = 1))]
    pub cause: String,
    #[schemars(length(min = 1))]
    pub evidence: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModifyProfileAttributeInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    pub category: CharacterAttributeCategory,
    #[schemars(length(min = 1))]
    pub key: String,
    pub new_value: Value,
    #[schemars(length(min = 1))]
    pub cause: String,
    #[schemars(length(min = 1))]
    pub evidence: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttributeHistoryInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_from: Option<i32>,
    #[schemars(range(min = 1))]
    pub chapter_to: Option<i32>,
    pub category: Option<CharacterAttributeCategory>,
    pub key: Option<String>,
    pub operation: Option<CharacterAttributeOperation>,
    #[schemars(range(min = 1, max = 300))]
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AttributeEventSummary {
    pub id: i64,
    pub chapter_number: i32,
    pub source_kind: CharacterTimelineSourceKind,
    pub timestamp: Option<String>,
    pub category: CharacterAttributeCategory,
    pub key: String,
    pub operation: CharacterAttributeOperation,
    pub change_description: String,
    pub old_value: Value,
    pub new_value: Value,
    pub cause: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListBehaviorsInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_from: Option<i32>,
    #[schemars(range(min = 1))]
    pub chapter_to: Option<i32>,
    #[schemars(range(min = 1, max = 300))]
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BehaviorRecordSummary {
    pub id: i64,
    pub chapter_number: i32,
    pub source_kind: CharacterTimelineSourceKind,
    pub behavior_description: String,
    pub behavior_type: String,
    pub original_text: String,
    pub behavior_tags: Value,
    pub classification_results: Value,
    pub related_characters: Option<Value>,
    pub impact_analysis: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetImplicitReviewInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 1))]
    pub threshold: usize,
    #[schemars(range(min = 1, max = 50))]
    pub window_chapters: usize,
    #[schemars(range(min = 1, max = 50))]
    pub evidence_limit: usize,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckImplicitUpdateInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(length(min = 1))]
    pub attribute_key: String,
    #[schemars(range(min = 1))]
    pub threshold: usize,
    #[schemars(range(min = 1, max = 50))]
    pub window_chapters: usize,
    #[schemars(range(min = 1, max = 50))]
    pub evidence_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImplicitEvidenceSummary {
    pub behavior_id: i64,
    pub chapter_number: i32,
    pub behavior_type: String,
    pub behavior_description: String,
    pub original_text: String,
    pub behavior_tags: Value,
    pub classification: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImplicitAttributeReviewSummary {
    pub attribute_key: String,
    pub current_value: Value,
    pub conform_count: usize,
    pub non_conform_count: usize,
    pub conform_rate: f64,
    pub threshold_reached: bool,
    pub evidence: Vec<ImplicitEvidenceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImplicitReviewSummary {
    pub attributes: Vec<ImplicitAttributeReviewSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImplicitUpdateDecisionSummary {
    pub attribute_key: String,
    pub current_value: Value,
    pub decision: ImplicitUpdateDecision,
    pub reasoning: String,
    pub suggested_new_value: Option<Value>,
    pub evidence_summary: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyImplicitUpdateDecisionInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    pub decision: ImplicitUpdateDecisionSummary,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApplyImplicitUpdateDecisionResultSummary {
    pub action: AppliedImplicitUpdateAction,
    pub deletion_mark: Option<DeletionMarkSummary>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListSnapshotsInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 1))]
    pub start_chapter: Option<i32>,
    #[schemars(range(min = 1))]
    pub end_chapter: Option<i32>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SnapshotReferenceSummary {
    pub id: i64,
    pub chapter_number: i32,
    pub snapshot_hash: String,
    pub attribute_counts: AttributeCountsSummary,
    pub state_summary: Option<String>,
    pub changes_in_chapter: usize,
    pub behaviors_in_chapter: usize,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetProfileStateAtChapterInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProfileChapterStateSummary {
    pub chapter_number: i32,
    pub attribute_counts: AttributeCountsSummary,
    pub explicit_attributes: Value,
    pub implicit_attributes: Value,
    pub social_attributes: Value,
    pub state_summary: Option<String>,
    pub current_location: Option<String>,
    pub changes_in_chapter: usize,
    pub behaviors_in_chapter: usize,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetSnapshotInput {
    #[schemars(length(min = 1))]
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SnapshotSummary {
    pub data: Value,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiffProfileChaptersInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 0))]
    pub from_chapter: i32,
    #[schemars(range(min = 1))]
    pub to_chapter: i32,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SnapshotDeltaSummary {
    pub delta: Value,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckProfileSyncConflictInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProfileSyncConflictSummary {
    pub has_conflict: bool,
    pub last_synced_chapter: i32,
    pub max_available_chapter: i32,
    pub available_snapshot_chapters: Vec<i32>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RollbackToChapterInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 0))]
    pub chapter_number: i32,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RollbackResultSummary {
    pub deleted_events: u64,
    pub deleted_snapshots: u64,
    pub deleted_behaviors: u64,
    pub deleted_state_indexes: u64,
    pub deleted_state_rag: usize,
    pub deleted_attribute_event_rag: usize,
    pub deleted_deletion_marks: u64,
    pub rewound_deletion_marks: u64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListDeletionMarksInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    pub category: Option<CharacterAttributeCategory>,
    pub status: DeletionMarkStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeletionMarkSummary {
    pub id: i64,
    pub source_kind: CharacterTimelineSourceKind,
    pub category: CharacterAttributeCategory,
    pub key: String,
    pub reason: String,
    pub evidence: String,
    pub consecutive_count: i32,
    pub last_marked_chapter: i32,
    pub is_executed: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestAttributeDeletionInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    pub category: CharacterAttributeCategory,
    #[schemars(length(min = 1))]
    pub key: String,
    #[schemars(length(min = 1))]
    pub reason: String,
    #[schemars(length(min = 1))]
    pub evidence: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecuteAttributeDeletionInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    pub category: CharacterAttributeCategory,
    #[schemars(length(min = 1))]
    pub key: String,
    #[schemars(length(min = 1))]
    pub cause: String,
    #[schemars(length(min = 1))]
    pub evidence: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResetDeletionMarksInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    pub category: CharacterAttributeCategory,
    #[schemars(length(min = 1))]
    pub key: String,
}
