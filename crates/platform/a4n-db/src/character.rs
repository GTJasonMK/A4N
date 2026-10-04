mod attributes;
mod behavior;
mod blueprint_seed;
mod portrait;
mod profile;
mod record_mappers;
mod snapshot;

mod sync_persistence;
mod timeline;

pub use attributes::{
    AppliedCharacterImplicitDecisionAction, AppliedCharacterImplicitDecisionRecord,
    CharacterAttributeDeletionExecutionResult, CharacterAttributeEventHistoryFilter,
    CharacterAttributeMutationResult, CharacterDeletionMarkRewindSummary, add_profile_attribute,
    apply_implicit_review_decision_record, attribute_events_by_ids, attribute_history,
    deletion_mark_records_by_profile, deletion_marks_by_profile, execute_ready_attribute_deletion,
    modify_profile_attribute, request_attribute_deletion, request_attribute_deletion_record,
    reset_deletion_marks_in, rewind_ai_deletion_marks_after, rewind_deletion_marks_after,
};
pub use behavior::{
    behavior_records_by_profile, behavior_records_by_profile_records,
    implicit_review_behavior_records,
};
pub use blueprint_seed::{CharacterProfileSeedError, sync_character_profiles_from_blueprint};
pub(crate) use portrait::character_portrait_record_from_model;
pub use portrait::{
    CharacterPortraitRecordError, CharacterPortraitRollbackError, CharacterPortraitRollbackResult,
    PortraitNamedPromptProfileRows, PortraitPromptProfileRows, character_portrait_by_id,
    character_portrait_record_by_id, character_portrait_records_by_project,
    character_portrait_records_by_project_records, create_character_portrait_for_profile,
    create_character_portrait_record_for_profile, missing_portrait_profile_records,
    missing_portrait_profiles, portrait_named_prompt_profile_rows, portrait_prompt_profile_rows,
    portrait_reference_images_by_ids, primary_character_portrait_record,
    primary_character_portrait_record_value, rollback_failed_portrait_creation_records,
    set_primary_character_portrait_record,
};
pub(crate) use profile::apply_profile_current_state;
pub(crate) use profile::character_profile_record;
pub(crate) use profile::delete_profiles_cascade_in;
pub(crate) use profile::referenced_profile_ids;
pub use profile::{
    CharacterProfileCreateError, CharacterProfileCurrentStateFields, CharacterProfileDeleteError,
    CharacterProfileDeletionSummary, create_profile, create_profile_domain_record,
    create_profile_with_generation_run, create_profile_with_generation_run_in,
    delete_profile_cascade_transaction, persist_character_discovery_candidates, profile_by_id,
    profile_by_project_name, profile_current_state_fields, profile_record_by_id,
    profile_record_by_project_name, profile_records_by_project, profiles_by_project,
    profiles_with_timeline_state, profiles_with_timeline_state_records, update_initial_profile,
    update_initial_profile_record,
};
pub(in crate::character) use record_mappers::{
    character_attribute_event_record, character_behavior_record, character_deletion_mark_record,
    character_snapshot_reference_record, character_state_index_record,
};
pub use snapshot::store::Sha256SnapshotStore;
pub use snapshot::{
    snapshot_attributes_from_data, snapshot_reference_by_chapter,
    snapshot_reference_record_by_chapter, snapshot_references_by_profile,
    snapshot_references_by_profile_records,
};
pub use sync_persistence::{
    AppliedCharacterSyncPersistenceRecord, CharacterSyncAttributeEventRecordInput,
    CharacterSyncPersistenceInput, CharacterSyncProtectedAttribute,
    CharacterSyncWriteStepRecordInput, CharacterSyncWriteSummary,
    apply_character_sync_persistence_record,
};
pub use timeline::{
    CharacterConflictContextRows, CharacterProfileRollbackResult, CharacterTimelineDeletionSummary,
    behavior_records_before_chapter, character_conflict_context_rows,
    delete_ai_timeline_artifacts_from_chapter_checked,
    delete_timeline_artifacts_from_chapter_checked, replay_profile_state_through_chapter,
    replay_profile_state_through_chapter_by_id, retract_profile_chapter_facts,
    rollback_profile_to_chapter,
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

fn validate_timeline_source_kind(source_kind: &str) -> Result<String, CharacterDbError> {
    let source_kind = source_kind.trim();
    if a4n_domain::character::CharacterTimelineSourceKind::parse(source_kind).is_some() {
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
