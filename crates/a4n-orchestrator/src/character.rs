mod attribute_lifecycle;
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
mod record_lifecycle;
mod snapshot_rules;
mod sync_projection;
mod sync_rules;
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;

pub(crate) use a4n_character_host_contract::{
    CHARACTER_PROFILE_SYNC_STEPS, CharacterDiscoveryApplicationPort, CharacterDiscoveryProgress,
    CharacterDiscoveryRequestPreview, CharacterImplicitReviewApplyApplicationPort,
    CharacterImplicitReviewCheckApplicationPort, CharacterImplicitReviewPreviewApplicationPort,
    CharacterProfileSyncApplicationPort, CharacterProfileSyncPreviewStep,
    CharacterProfileSyncProgress, CharacterProfileSyncStageTracePort,
};

pub(crate) use a4n_db::character::{
    AppliedCharacterImplicitDecisionAction, AppliedCharacterImplicitDecisionRecord,
    AppliedCharacterSyncPersistenceRecord, CharacterAttributeDeletionExecutionResult,
    CharacterAttributeEventHistoryFilter, CharacterAttributeMutationResult,
    CharacterConflictContextRows, CharacterPortraitRecordError, CharacterProfileCreateError,
    CharacterProfileDeleteError, CharacterProfileDeletionSummary, CharacterProfileRollbackResult,
    CharacterSyncAttributeEventRecordInput, CharacterSyncPersistenceInput,
    CharacterSyncWriteStepRecordInput, CharacterTimelineDeletionSummary,
    PortraitNamedPromptProfileRows, PortraitPromptProfileRows,
};
pub(crate) use a4n_db::generation::RecordCompletedGenerationRunInput;
pub(crate) use a4n_domain::character::category_storage_key;
pub(crate) use attribute_lifecycle::CharacterAttributeLifecycleError;
pub(crate) use record_lifecycle::{
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

pub(crate) fn character_portrait_record_error(error: CharacterPortraitRecordError) -> AppError {
    match error {
        CharacterPortraitRecordError::NotFound(message) => AppError::NotFound(message),
        CharacterPortraitRecordError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        CharacterPortraitRecordError::Database(error) => AppError::Database(error),
    }
}

pub(crate) fn character_profile_create_error(error: CharacterProfileCreateError) -> AppError {
    match error {
        CharacterProfileCreateError::AlreadyExists { name } => AppError::InvalidState {
            current: "character_profile_already_exists".to_string(),
            operation: format!("创建角色档案 {name}"),
        },
        CharacterProfileCreateError::InvalidArgument(message) => AppError::InvalidArgument(message),
        CharacterProfileCreateError::Database(error) => AppError::Database(error),
        CharacterProfileCreateError::Generation(error) => {
            crate::generation::generation_error(error)
        }
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

pub(crate) use attribute_lifecycle::implicit_review_attribute_event;

#[async_trait::async_trait]
impl CharacterDiscoveryApplicationPort for crate::OrchestratorContext {
    async fn preview_discovery(
        &self,
        user_id: i64,
        input: a4n_api_contract::character::DiscoverCharacterProfilesInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<CharacterDiscoveryRequestPreview, AppError> {
        self.verify_project_permission(
            input.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.prepare_character_discovery_request_preview_validated(input, prompt_renderer)
            .await
    }

    async fn discover(
        &self,
        user_id: i64,
        input: a4n_api_contract::character::DiscoverCharacterProfilesInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: Option<CharacterDiscoveryProgress<'_>>,
    ) -> Result<a4n_api_contract::character::DiscoverCharacterProfilesResult, AppError> {
        self.verify_project_permission(
            input.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.discover_character_profiles_validated(input, prompt_renderer, progress)
            .await
    }
}

#[async_trait::async_trait]
impl CharacterProfileSyncApplicationPort for crate::OrchestratorContext {
    async fn preview_profile_sync(
        &self,
        user_id: i64,
        input: a4n_api_contract::character::SyncCharacterProfileInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Option<CharacterProfileSyncPreviewStep>, AppError> {
        self.verify_project_permission(
            input.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.prepare_character_profile_sync_preview_validated(
            CharacterProfileSyncApplicationInput::preview_by_name(
                input.project_id,
                input.name,
                input.chapter_number,
            ),
            prompt_renderer,
        )
        .await
    }

    async fn sync_profile(
        &self,
        user_id: i64,
        input: a4n_api_contract::character::SyncCharacterProfileInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: Option<CharacterProfileSyncProgress<'_>>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<a4n_api_contract::character::SyncCharacterProfileResult, AppError> {
        self.verify_project_permission(
            input.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.run_character_profile_sync_validated(
            CharacterProfileSyncApplicationInput::incremental_by_name(
                input.project_id,
                input.name,
                input.chapter_number,
            ),
            prompt_renderer,
            progress,
            trace_port,
        )
        .await
    }

    async fn preview_all_profiles_sync(
        &self,
        user_id: i64,
        input: a4n_api_contract::character::SyncAllCharacterProfilesInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<CharacterProfileSyncPreviewStep>, AppError> {
        self.verify_project_permission(
            input.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.prepare_all_character_profile_sync_preview_validated(
            input.project_id,
            input.chapter_number,
            prompt_renderer,
        )
        .await
    }

    async fn sync_all_profiles(
        &self,
        user_id: i64,
        input: a4n_api_contract::character::SyncAllCharacterProfilesInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: CharacterProfileSyncProgress<'_>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<a4n_api_contract::character::SyncAllCharacterProfilesResult, AppError> {
        self.verify_project_permission(
            input.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.run_all_character_profile_sync_validated(
            input.project_id,
            input.chapter_number,
            prompt_renderer,
            progress,
            trace_port,
        )
        .await
    }
}
pub(crate) use discovery::{CHARACTER_DISCOVERY_CALL_KEY, CHARACTER_DISCOVERY_KIND};
pub(crate) use discovery::{
    character_identity_key, character_llm_chapter_context, profile_character_keys,
};
pub(crate) use implicit_classification::CharacterImplicitClassificationInput;
pub(crate) use implicit_review::CharacterSyncImplicitReviewInput;
pub(crate) use implicit_review::IMPLICIT_UPDATE_WORKFLOW_KIND;
pub(crate) use profile_sync::{
    CharacterProfileSyncAnchorMode, CharacterProfileSyncApplicationInput,
    CharacterProfileSyncSubject,
};
pub(crate) use profile_sync_stages::{
    CharacterProfileSyncChapterStateDraft, CharacterProfileSyncGenerationTrace,
    CharacterProfileSyncStageInput,
};
pub(crate) use sync_projection::{
    CharacterSyncAttributeEventDraft, pending_character_sync_attribute_events,
    project_character_attribute_changes,
};
pub(crate) use sync_rules::{
    CharacterSyncRangeMode, character_profile_skipped_result, profile_first_seen_chapter,
    validate_character_sync_target_chapter, validate_contiguous_character_sync_range,
};
pub(crate) use sync_rules::{
    character_profile_sync_business_key, character_profile_sync_mode_label,
    character_profile_sync_stage_call_key, character_profile_sync_workflow_fingerprint,
    character_sync_range_with_first_seen,
};

pub(crate) use snapshot_rules::{
    CHARACTER_SNAPSHOT_ENTITY_TYPE, ensure_positive_snapshot_chapter, initial_snapshot_data,
    validate_snapshot_diff_chapters, validate_snapshot_range,
};

pub(crate) use json_field_rules::{
    attribute_counts, category_attributes, optional_string, optional_string_or_null,
    required_array, required_attribute_change_operation, required_category, required_i32,
    required_i32_at, required_string, required_string_array, required_usize, required_value,
};
