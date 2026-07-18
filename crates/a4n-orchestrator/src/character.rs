mod attribute_lifecycle;
mod attributes;
mod behavior;
mod profiles;
mod record_lifecycle;
mod snapshots;
mod timeline;

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
