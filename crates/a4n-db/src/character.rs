mod attributes;
mod behavior;
mod blueprint_seed;
mod portrait;
mod profile;
mod snapshot;
mod snapshot_store;
mod sync_persistence;
mod timeline;

pub use attributes::{
    AppliedCharacterImplicitDecisionAction, AppliedCharacterImplicitDecisionRecord,
    CharacterAttributeDeletionExecutionInput, CharacterAttributeDeletionExecutionResult,
    CharacterAttributeDeletionRequestInput, CharacterAttributeEventHistoryFilter,
    CharacterAttributeMutationResult, CharacterAttributeValueChangeInput,
    CharacterDeletionMarkRewindSummary, CharacterImplicitReviewDecisionRecordInput,
    add_profile_attribute, apply_attribute_to_state, apply_implicit_review_decision_record,
    attribute_events_by_ids, attribute_history, deletion_marks_by_profile,
    execute_ready_attribute_deletion, modify_profile_attribute, request_attribute_deletion,
    reset_deletion_marks_in, rewind_deletion_marks_after,
};
pub use behavior::{
    InsertBehaviorRecordInput, behavior_records_by_profile, implicit_review_behavior_records,
};
pub use blueprint_seed::{CharacterProfileSeedError, sync_character_profiles_from_blueprint};
pub use portrait::{
    CharacterPortraitCreateInput, CharacterPortraitRecordError, CharacterPortraitRollbackError,
    CharacterPortraitRollbackResult, PortraitNamedPromptProfileRows, PortraitPromptProfileRows,
    character_portrait_by_id, character_portrait_records_by_project,
    create_character_portrait_for_profile, missing_portrait_profiles,
    portrait_named_prompt_profile_rows, portrait_prompt_profile_rows,
    portrait_reference_images_by_ids, primary_character_portrait_record,
    rollback_failed_portrait_creation_records, set_primary_character_portrait_record,
};
pub(crate) use profile::apply_profile_current_state;
pub(crate) use profile::delete_profile_cascade_in;
pub use profile::{
    CharacterProfileCreateError, CharacterProfileCreateInput, CharacterProfileCurrentStateFields,
    CharacterProfileDeleteError, CharacterProfileDeletionSummary, create_profile,
    create_profile_with_generation_run, create_profile_with_generation_run_in,
    delete_profile_cascade_transaction, profile_by_id, profile_by_project_name,
    profile_current_state_fields, profiles_by_project, profiles_with_timeline_state,
    update_initial_profile,
};
pub use snapshot::{
    snapshot_attributes_from_data, snapshot_reference_by_chapter, snapshot_references_by_profile,
};
pub use snapshot_store::Sha256SnapshotStore;
pub use sync_persistence::{
    AppliedCharacterSyncPersistenceRecord, CharacterSyncAttributeEventRecordInput,
    CharacterSyncChapterStateRecordInput, CharacterSyncPersistenceInput,
    CharacterSyncProtectedAttribute, CharacterSyncWriteStepRecordInput, CharacterSyncWriteSummary,
    apply_character_sync_persistence_record, character_sync_attribute_events_at_chapter,
};
pub use timeline::{
    CharacterConflictContextRows, CharacterProfileRollbackResult, CharacterTimelineDeletionSummary,
    behavior_records_before_chapter, character_conflict_context_rows,
    delete_timeline_artifacts_from_chapter_checked, replay_profile_state_through_chapter,
    rollback_profile_to_chapter,
};

use a4n_core::models::character::attribute_event::{
    SOURCE_KIND_AI_SYNC, SOURCE_KIND_LEGACY_UNKNOWN, SOURCE_KIND_USER,
};
use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum CharacterDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("状态无效：当前状态 {current}，不允许操作 {operation}")]
    InvalidState { current: String, operation: String },
    #[error("{0}")]
    Internal(String),
    #[error(transparent)]
    Database(#[from] DbErr),
    #[error(transparent)]
    Generation(#[from] crate::generation::GenerationDbError),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}

fn validate_category(category: &str) -> Result<(), CharacterDbError> {
    if matches!(category, "explicit" | "implicit" | "social") {
        Ok(())
    } else {
        Err(CharacterDbError::InvalidArgument(
            "属性分类必须是 explicit/implicit/social".to_string(),
        ))
    }
}

fn require_non_empty(value: &str, field_name: &str) -> Result<String, CharacterDbError> {
    let value = value.trim();
    if value.is_empty() {
        Err(CharacterDbError::InvalidArgument(format!(
            "{field_name}不能为空"
        )))
    } else {
        Ok(value.to_string())
    }
}

fn validate_chapter_number(chapter_number: i32) -> Result<(), CharacterDbError> {
    if chapter_number <= 0 {
        Err(CharacterDbError::InvalidArgument(
            "章节号必须大于 0".to_string(),
        ))
    } else {
        Ok(())
    }
}

pub(in crate::character) fn validate_timeline_source_kind(
    source_kind: &str,
) -> Result<String, CharacterDbError> {
    let source_kind = source_kind.trim();
    if matches!(
        source_kind,
        SOURCE_KIND_USER | SOURCE_KIND_AI_SYNC | SOURCE_KIND_LEGACY_UNKNOWN
    ) {
        Ok(source_kind.to_string())
    } else {
        Err(CharacterDbError::InvalidArgument(format!(
            "角色时间线 source_kind 无效: {source_kind}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn character_db_error_preserves_structured_classification() {
        let invalid_argument = CharacterDbError::InvalidArgument("章节范围无效".to_string());
        assert!(matches!(
            invalid_argument,
            CharacterDbError::InvalidArgument(message) if message == "章节范围无效"
        ));

        let database =
            CharacterDbError::Database(DbErr::Custom("timeline query failed".to_string()));
        assert!(matches!(
            database,
            CharacterDbError::Database(DbErr::Custom(message))
                if message == "timeline query failed"
        ));

        let generation =
            CharacterDbError::from(crate::generation::GenerationDbError::InvalidArgument(
                "generation contract failed".to_string(),
            ));
        assert!(matches!(
            generation,
            CharacterDbError::Generation(
                crate::generation::GenerationDbError::InvalidArgument(message)
            ) if message == "generation contract failed"
        ));

        let serialization = CharacterDbError::from(
            serde_json::from_str::<Value>("{").expect_err("invalid JSON must fail"),
        );
        assert!(matches!(serialization, CharacterDbError::Serialization(_)));
    }
}
