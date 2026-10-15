use a4n_domain::character::{
    AppliedImplicitUpdateAction, CharacterAttributeCategory, CharacterAttributeOperation,
    CharacterTimelineSourceKind, DeletionMarkStatus, ImplicitUpdateDecision,
};
use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateProfileInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub name: String,
    pub base_attributes: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateCharacterProfileInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    pub expected_current_state: Value,
    pub base_attributes: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DiscoverCharacterProfilesInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CharacterConflictCheckInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1))]
    pub name: String,
    #[schemars(range(min = 1))]
    pub start_chapter: Option<i32>,
    #[schemars(range(min = 1))]
    pub end_chapter: Option<i32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CharacterConflictSeverity {
    Minor,
    Moderate,
    Major,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CharacterConflictSummary {
    pub chapter_number: i64,
    pub field: String,
    pub expected: Value,
    pub actual: Value,
    pub expected_source_id: String,
    pub actual_source_id: String,
    pub severity: CharacterConflictSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AttributeCountsSummary {
    pub explicit: usize,
    pub implicit: usize,
    pub social: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
    pub timeline_started: bool,
    pub initial_profile_editable: bool,
    pub attribute_counts: AttributeCountsSummary,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Narrow overview reads used by list pages and cross-domain identity checks.
/// History, snapshots and synchronization remain on the character timeline
/// owner so an overview change does not rebuild those consumers.
#[async_trait::async_trait]
pub trait CharacterProfileOverviewReadApplicationPort: Send + Sync {
    async fn profile_project_id(
        &self,
        user_id: i64,
        profile_id: i64,
    ) -> Result<i64, a4n_error::AppError>;

    async fn list(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<CharacterProfileSummary>, a4n_error::AppError>;

    async fn character_arc_by_name(
        &self,
        user_id: i64,
        project_id: i64,
        name: String,
    ) -> Result<Option<Value>, a4n_error::AppError>;

    async fn voice_fingerprint_by_name(
        &self,
        user_id: i64,
        project_id: i64,
        name: String,
    ) -> Result<Option<Value>, a4n_error::AppError>;
}

/// Character timeline reads that only need the character owner and database
/// projections. Snapshot diff and sync-conflict checks remain separate because
/// they combine multiple source owners.
#[async_trait::async_trait]
pub trait CharacterTimelineReadApplicationPort: Send + Sync {
    async fn attribute_history(
        &self,
        user_id: i64,
        input: AttributeHistoryInput,
    ) -> Result<Vec<AttributeEventSummary>, a4n_error::AppError>;

    async fn behavior_history(
        &self,
        user_id: i64,
        input: ListBehaviorsInput,
    ) -> Result<Vec<BehaviorRecordSummary>, a4n_error::AppError>;

    async fn deletion_marks(
        &self,
        user_id: i64,
        input: ListDeletionMarksInput,
    ) -> Result<Vec<DeletionMarkSummary>, a4n_error::AppError>;

    async fn snapshots(
        &self,
        user_id: i64,
        input: ListSnapshotsInput,
    ) -> Result<Vec<SnapshotReferenceSummary>, a4n_error::AppError>;

    async fn state_at_chapter(
        &self,
        user_id: i64,
        input: GetProfileStateAtChapterInput,
    ) -> Result<ProfileChapterStateSummary, a4n_error::AppError>;

    async fn snapshot_by_hash(
        &self,
        user_id: i64,
        input: GetSnapshotInput,
    ) -> Result<SnapshotSummary, a4n_error::AppError>;

    async fn sync_conflict(
        &self,
        user_id: i64,
        input: CheckProfileSyncConflictInput,
    ) -> Result<ProfileSyncConflictSummary, a4n_error::AppError>;

    async fn diff_chapters(
        &self,
        user_id: i64,
        input: DiffProfileChaptersInput,
    ) -> Result<SnapshotDeltaSummary, a4n_error::AppError>;
}

/// Host-facing mutation boundary for initial character profiles.
///
/// Authorization, persistence and the profile projection stay with the
/// character owner; Runtime only appends post-commit RAG warnings.
#[async_trait::async_trait]
pub trait CharacterProfileMutationApplicationPort: Send + Sync {
    async fn create(
        &self,
        user_id: i64,
        input: CreateProfileInput,
    ) -> Result<CharacterProfileCreateResult, a4n_error::AppError>;

    async fn update(
        &self,
        user_id: i64,
        input: UpdateCharacterProfileInput,
    ) -> Result<CharacterProfileMutationResult, a4n_error::AppError>;
}

/// Host callback for removing a deleted profile from the project's RAG set.
#[async_trait::async_trait]
pub trait CharacterProfileDeletionHostPort: Send + Sync {
    async fn sync_profile_rag(&self, project_id: i64) -> Result<(), a4n_error::AppError>;
}

/// Host-facing mutation boundary for deleting a character profile.
///
/// The owner keeps the committed database cascade; the Host callback owns the
/// post-commit RAG cleanup and warning semantics.
#[async_trait::async_trait]
pub trait CharacterProfileDeletionApplicationPort: Send + Sync {
    async fn delete(
        &self,
        user_id: i64,
        host_port: &dyn CharacterProfileDeletionHostPort,
        profile_id: i64,
    ) -> Result<CharacterProfileDeleteResult, a4n_error::AppError>;
}

/// Host-facing mutation boundary for character timeline rollback.
///
/// The character owner keeps authorization and timeline persistence; the Host
/// callback owns deletion of the committed timeline's RAG records.
#[async_trait::async_trait]
pub trait CharacterProfileTimelineMutationHostPort: Send + Sync {
    async fn delete_timeline_rag(
        &self,
        project_id: i64,
        state_index_ids: Vec<i64>,
        attribute_event_ids: Vec<i64>,
    ) -> Result<(usize, usize), a4n_error::AppError>;
}

#[async_trait::async_trait]
pub trait CharacterProfileTimelineMutationApplicationPort: Send + Sync {
    async fn rollback(
        &self,
        user_id: i64,
        host_port: &dyn CharacterProfileTimelineMutationHostPort,
        input: RollbackToChapterInput,
    ) -> Result<RollbackResultSummary, a4n_error::AppError>;
}

/// Host-facing mutation boundary for explicit character attribute changes.
///
/// Attribute validation and persistence stay with the character owner; the
/// Host callback owns the post-write RAG side effect.
#[async_trait::async_trait]
pub trait CharacterAttributeRagSyncHostPort: Send + Sync {
    async fn sync_attribute_event(
        &self,
        project_id: i64,
        event_id: i64,
    ) -> Result<(), a4n_error::AppError>;
}

#[async_trait::async_trait]
pub trait CharacterProfileAttributeMutationApplicationPort: Send + Sync {
    async fn add(
        &self,
        user_id: i64,
        host_port: &dyn CharacterAttributeRagSyncHostPort,
        input: AddProfileAttributeInput,
    ) -> Result<AttributeEventWriteResult, a4n_error::AppError>;

    async fn modify(
        &self,
        user_id: i64,
        host_port: &dyn CharacterAttributeRagSyncHostPort,
        input: ModifyProfileAttributeInput,
    ) -> Result<AttributeEventWriteResult, a4n_error::AppError>;
}

/// Host-facing mutation boundary for character attribute deletion marks.
///
/// Mark validation, persistence, execution and DTO projection stay with the
/// character owner; Runtime only forwards the typed command.
#[async_trait::async_trait]
pub trait CharacterProfileAttributeDeletionApplicationPort: Send + Sync {
    async fn request(
        &self,
        user_id: i64,
        input: RequestAttributeDeletionInput,
    ) -> Result<DeletionMarkSummary, a4n_error::AppError>;

    async fn execute(
        &self,
        user_id: i64,
        host_port: &dyn CharacterAttributeRagSyncHostPort,
        input: ExecuteAttributeDeletionInput,
    ) -> Result<AttributeEventWriteResult, a4n_error::AppError>;

    async fn reset(
        &self,
        user_id: i64,
        input: ResetDeletionMarksInput,
    ) -> Result<u64, a4n_error::AppError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CharacterProfileCreateResult {
    pub profile: CharacterProfileSummary,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CharacterProfileMutationResult {
    pub profile: CharacterProfileSummary,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CharacterProfileDeleteResult {
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct DiscoverCharacterProfilesResult {
    pub status: String,
    pub chapters_analyzed: usize,
    pub chapter_numbers: Vec<i32>,
    pub discovered_count: usize,
    pub created: Vec<DiscoveredCharacterProfileSummary>,
    pub existing: Vec<ExistingCharacterProfileDiscovery>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct DiscoveredCharacterProfileSummary {
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
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ExistingCharacterProfileDiscovery {
    pub name: String,
    pub aliases: Value,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CharacterProfileSyncRange {
    pub from: i32,
    pub to: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CharacterProfileProtectedAttribute {
    pub chapter_number: i32,
    pub category: String,
    pub key: String,
    pub operation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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
    /// 本次为重算清理 AI 时间线的结束章节。大于 `synced_chapter` 时说明档案的已同步进度
    /// 被回退，`cleared_*` 是被清掉、需要重新同步的记录数。
    pub cleared_through_chapter: i32,
    pub cleared_attribute_events: u64,
    pub cleared_behaviors: u64,
    pub cleared_state_indexes: u64,
    pub cleared_snapshots: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CharacterProfileSkippedResult {
    pub name: String,
    pub synced_chapter: i32,
    pub target_chapter: i32,
    pub first_seen_chapter: Option<i32>,
    pub reason: CharacterProfileSyncSkipReason,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AttributeEventWriteResult {
    pub event: AttributeEventSummary,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

/// Host-facing read boundary for implicit character reviews.
///
/// Decision checks, request previews and application remain with the review
/// workflow owner.
#[async_trait::async_trait]
pub trait CharacterImplicitReviewReadApplicationPort: Send + Sync {
    async fn summary(
        &self,
        user_id: i64,
        input: GetImplicitReviewInput,
    ) -> Result<ImplicitReviewSummary, a4n_error::AppError>;
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListSnapshotsInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 1))]
    pub start_chapter: Option<i32>,
    #[schemars(range(min = 1))]
    pub end_chapter: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SnapshotReferenceSummary {
    pub id: i64,
    pub chapter_number: i32,
    pub snapshot_hash: String,
    pub attribute_counts: AttributeCountsSummary,
    pub state_summary: Option<String>,
    pub changes_in_chapter: usize,
    pub behaviors_in_chapter: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetProfileStateAtChapterInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetSnapshotInput {
    #[schemars(length(min = 1))]
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SnapshotSummary {
    pub data: Value,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiffProfileChaptersInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 0))]
    pub from_chapter: i32,
    #[schemars(range(min = 1))]
    pub to_chapter: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SnapshotDeltaSummary {
    pub delta: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckProfileSyncConflictInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProfileSyncConflictSummary {
    pub has_conflict: bool,
    pub last_synced_chapter: i32,
    pub max_available_chapter: i32,
    pub available_snapshot_chapters: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
    #[schemars(with = "String")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecuteAttributeDeletionInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    pub category: CharacterAttributeCategory,
    #[schemars(length(min = 1))]
    pub key: String,
    #[schemars(range(min = 1))]
    pub expected_mark_id: i64,
    #[schemars(with = "String")]
    pub expected_mark_updated_at: NaiveDateTime,
    #[schemars(length(min = 1))]
    pub cause: String,
    #[schemars(length(min = 1))]
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResetDeletionMarksInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    pub category: CharacterAttributeCategory,
    #[schemars(length(min = 1))]
    pub key: String,
    #[schemars(range(min = 1))]
    pub expected_mark_id: i64,
    #[schemars(with = "String")]
    pub expected_mark_updated_at: NaiveDateTime,
}
