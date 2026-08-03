mod attribute_lifecycle;
mod checkpoint_rules;
mod conflict;
mod conflict_context_rules;
mod conflict_output_rules;
mod conflict_rules;
mod discovery;
mod discovery_rules;
mod discovery_workflow_rules;
mod implicit_classification;
mod implicit_review;
mod implicit_review_rules;
mod implicit_review_workflow_rules;
mod json_field_rules;
mod profile_sync;
mod profile_sync_batch;
mod profile_sync_stages;
pub mod profiles;
mod record_lifecycle;
mod snapshot_rules;
mod sync_projection;
mod sync_rules;
use a4n_core::error::AppError;

pub use a4n_db::character::{
    AppliedCharacterImplicitDecisionAction, AppliedCharacterImplicitDecisionRecord,
    AppliedCharacterSyncPersistenceRecord, CharacterAttributeDeletionExecutionInput,
    CharacterAttributeDeletionExecutionResult, CharacterAttributeDeletionRequestInput,
    CharacterAttributeEventHistoryFilter, CharacterAttributeEventInsertInput,
    CharacterAttributeMutationResult, CharacterAttributeValueChangeInput,
    CharacterConflictContextRows, CharacterDeletionMarkExecutedUpdateInput,
    CharacterDeletionMarkRewindSummary, CharacterDeletionMarkUpsertInput,
    CharacterImplicitReviewDecisionRecordInput, CharacterPortraitCreateInput,
    CharacterPortraitRecordError, CharacterPortraitRollbackError, CharacterPortraitRollbackResult,
    CharacterProfileCreateError, CharacterProfileCreateInput, CharacterProfileDeleteError,
    CharacterProfileDeletionSummary, CharacterProfileRollbackResult,
    CharacterSnapshotReferenceUpsertInput, CharacterStateIndexUpsertInput,
    CharacterSyncAttributeEventRecordInput, CharacterSyncChapterStateRecordInput,
    CharacterSyncPersistenceInput, CharacterSyncWriteStepRecordInput,
    CharacterTimelineDeletionSummary, DELETION_THRESHOLD, DeletionMarkListStatus,
    InsertBehaviorRecordInput, PortraitNamedPromptProfileRows, PortraitPromptProfileRows,
    apply_attribute_to_state, character_sync_attribute_events_at_chapter,
    snapshot_attributes_from_data,
};
pub use a4n_db::generation::{GenerationRunTraceTarget, RecordCompletedGenerationRunInput};
pub use attribute_lifecycle::CharacterAttributeLifecycleError;
pub use record_lifecycle::{
    CharacterProfileRecordDeletionResult, CharacterRecordLifecycleError,
    CharacterTimelineRollbackLifecycleResult,
};

pub(crate) fn character_db_error(error: a4n_db::character::CharacterDbError) -> AppError {
    match error {
        a4n_db::character::CharacterDbError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::character::CharacterDbError::NotFound(message) => AppError::NotFound(message),
        a4n_db::character::CharacterDbError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        a4n_db::character::CharacterDbError::Internal(message) => {
            AppError::Internal(anyhow::anyhow!(message))
        }
        a4n_db::character::CharacterDbError::Database(error) => AppError::Database(error),
        a4n_db::character::CharacterDbError::Generation(error) => {
            crate::generation::generation_error(error)
        }
        a4n_db::character::CharacterDbError::Serialization(error) => AppError::Serialization(error),
    }
}

pub(crate) fn character_profile_delete_error(error: CharacterProfileDeleteError) -> AppError {
    match error {
        CharacterProfileDeleteError::NotFound(message) => AppError::NotFound(message),
        CharacterProfileDeleteError::Database(error) => AppError::Database(error),
        CharacterProfileDeleteError::Generation(error) => {
            crate::generation::generation_error(error)
        }
        CharacterProfileDeleteError::ImageCleanup(error) => crate::video::video_db_error(error),
    }
}

pub use attribute_lifecycle::{
    implicit_review_attribute_event, normalize_character_attribute_deletion_execution,
    normalize_character_attribute_deletion_request, normalize_character_attribute_value_change,
    normalize_deletion_mark_reset_target, parse_deletion_mark_list_category,
    parse_deletion_mark_list_status,
};

pub use checkpoint_rules::{
    optional_character_sync_checkpoint_string, required_character_sync_checkpoint_string,
};
pub use conflict::{
    CharacterConflictCheckInput, CharacterConflictProgressReporter,
    CharacterConflictRequestPreview, CharacterConflictTaskScope,
};
pub use discovery::{
    CharacterDiscoveryInput, CharacterDiscoveryProgress, CharacterDiscoveryRequestPreview,
    CharacterDiscoveryTaskScope, character_identity_key, character_llm_chapter_context,
    profile_character_keys,
};
pub use implicit_classification::{
    CharacterImplicitClassificationInput, CharacterImplicitClassificationOutput,
    IMPLICIT_CLASSIFICATION_PROMPT_NAME,
};
pub use implicit_review::{
    ApplyImplicitUpdateDecisionInput, ApplyImplicitUpdateDecisionResultSummary,
    AttributeEventSummary, CharacterSyncImplicitReviewInput, CheckImplicitUpdateInput,
    DeletionMarkSummary, GetImplicitReviewInput, IMPLICIT_UPDATE_PROMPT_NAME,
    ImplicitAttributeReviewSummary, ImplicitEvidenceSummary, ImplicitReviewSummary,
    ImplicitUpdateDecisionSummary, ImplicitUpdateReviewRequestPreview,
};
pub use implicit_review_rules::{
    AUTO_IMPLICIT_REVIEW_EVIDENCE_LIMIT, AUTO_IMPLICIT_REVIEW_THRESHOLD,
    AUTO_IMPLICIT_REVIEW_WINDOW_CHAPTERS, validate_implicit_review_params,
};
pub use profile_sync::{
    CharacterProfileSyncAnchorMode, CharacterProfileSyncApplicationInput,
    CharacterProfileSyncProgress, CharacterProfileSyncSubject,
};
pub use profile_sync_batch::{
    CHARACTER_PROFILE_SYNC_STEPS, CharacterProfileBatchProgress,
    CharacterProfileBatchProgressReporter, analysis_sync_character_names,
    character_keys_from_names, character_names_from_chapter_analysis,
    character_names_from_discovery_result,
};
pub use profile_sync_stages::{
    CharacterProfileSyncChapterStateDraft, CharacterProfileSyncExtraction,
    CharacterProfileSyncGenerationTrace, CharacterProfileSyncPreviewStep,
    CharacterProfileSyncStageInput, CharacterProfileSyncStageOutput,
    CharacterProfileSyncStageProgress, CharacterProfileSyncStageTraceInput,
    CharacterProfileSyncStageTracePort,
};
pub use sync_projection::{
    CharacterSyncAttributeEventDraft, pending_character_sync_attribute_events,
    project_character_attribute_changes,
};
pub use sync_rules::{
    CharacterSyncRange, CharacterSyncRangeMode, analysis_profile_sync_skip_reason,
    character_profile_sync_business_key, character_profile_sync_mode_label,
    character_profile_sync_stage_call_key, character_profile_sync_workflow_fingerprint,
    character_sync_range, character_sync_start_chapter, profile_first_seen_chapter,
    profile_sync_skip_reason, sync_skip_summary, sync_skip_summary_with_reason,
    validate_character_sync_target_chapter, validate_contiguous_character_sync_range,
};

pub use snapshot_rules::{
    CHARACTER_SNAPSHOT_ENTITY_TYPE, ensure_positive_snapshot_chapter, initial_snapshot_data,
    validate_snapshot_diff_chapters, validate_snapshot_range,
};

#[derive(Debug, Clone)]
pub struct CharacterSyncConflictReadModel {
    pub profile_id: i64,
    pub has_conflict: bool,
    pub last_synced_chapter: i32,
    pub max_available_chapter: i32,
    pub available_snapshot_chapters: Vec<i32>,
}

pub use conflict_rules::{
    character_conflict_business_key, character_conflict_call_fingerprint_json,
    character_conflict_workflow_fingerprint,
};
pub use json_field_rules::{
    attribute_counts, category_attributes, category_storage_key, optional_string,
    optional_string_or_null, required_array, required_attribute_change_operation,
    required_category, required_i32, required_i32_at, required_i64, required_string,
    required_string_array, required_usize, required_value,
};
