use a4n_core::error::AppError;
use a4n_core::event::snapshot::{Snapshot, SnapshotDelta};
use a4n_core::models::ai::generated_image;
use a4n_core::models::character::{
    attribute_event, character_portrait, protagonist_behavior_record, protagonist_deletion_mark,
    protagonist_profile, snapshot_reference,
};
use a4n_orchestrator::{
    OrchestratorContext,
    character::{
        ApplyImplicitUpdateDecisionInput, ApplyImplicitUpdateDecisionResultSummary,
        CharacterAttributeDeletionExecutionResult, CharacterAttributeDeletionRequestInput,
        CharacterAttributeEventHistoryFilter, CharacterAttributeLifecycleError,
        CharacterAttributeMutationResult, CharacterAttributeValueChangeInput,
        CharacterConflictCheckInput, CharacterConflictProgressReporter,
        CharacterConflictRequestPreview, CharacterConflictTaskScope, CharacterDiscoveryInput,
        CharacterDiscoveryProgress, CharacterDiscoveryRequestPreview, CharacterDiscoveryTaskScope,
        CharacterProfileRecordDeletionResult, CharacterProfileSyncApplicationInput,
        CharacterProfileSyncPreviewStep, CharacterProfileSyncProgress,
        CharacterProfileSyncStageTracePort, CharacterRecordLifecycleError,
        CharacterSyncConflictReadModel, CharacterTimelineRollbackLifecycleResult,
        CheckImplicitUpdateInput, GetImplicitReviewInput, ImplicitReviewSummary,
        ImplicitUpdateDecisionSummary, ImplicitUpdateReviewRequestPreview,
    },
};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

pub(crate) struct CharacterRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> CharacterRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<protagonist_profile::Model>, AppError> {
        self.orchestrator.profiles_by_project(project_id).await
    }

    pub(crate) async fn profile_by_id(
        &self,
        profile_id: i64,
    ) -> Result<Option<protagonist_profile::Model>, AppError> {
        self.orchestrator.profile_by_id(profile_id).await
    }

    pub(crate) async fn profile_by_project_name(
        &self,
        project_id: i64,
        name: &str,
    ) -> Result<Option<protagonist_profile::Model>, AppError> {
        self.orchestrator
            .profile_by_project_name(project_id, name)
            .await
    }

    pub(crate) async fn missing_portrait_profiles(
        &self,
        project_id: i64,
    ) -> Result<Vec<protagonist_profile::Model>, AppError> {
        self.orchestrator
            .missing_portrait_profiles(project_id)
            .await
    }

    pub(crate) async fn create_profile_validated(
        &self,
        project_id: i64,
        name: String,
        base_attributes: Value,
    ) -> Result<protagonist_profile::Model, AppError> {
        self.orchestrator
            .create_profile_validated(project_id, name, base_attributes)
            .await
    }

    pub(crate) async fn character_portrait_by_id(
        &self,
        portrait_id: i64,
    ) -> Result<Option<character_portrait::Model>, AppError> {
        self.orchestrator
            .character_portrait_by_id(portrait_id)
            .await
    }

    pub(crate) async fn character_portrait_records_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<character_portrait::Model>, AppError> {
        self.orchestrator
            .character_portrait_records_by_project(project_id)
            .await
    }

    pub(crate) async fn primary_character_portrait_record(
        &self,
        profile_id: i64,
    ) -> Result<Option<character_portrait::Model>, AppError> {
        self.orchestrator
            .primary_character_portrait_record(profile_id)
            .await
    }

    pub(crate) async fn set_primary_character_portrait_record(
        &self,
        portrait_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .set_primary_character_portrait_record(portrait_id)
            .await
    }

    pub(crate) async fn portrait_reference_images_by_ids(
        &self,
        project_id: i64,
        image_ids: &[i64],
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.orchestrator
            .portrait_reference_images_by_ids(project_id, image_ids)
            .await
    }

    pub(crate) async fn attribute_history(
        &self,
        profile_id: i64,
        filter: CharacterAttributeEventHistoryFilter,
    ) -> Result<Vec<attribute_event::Model>, AppError> {
        self.orchestrator
            .attribute_history(profile_id, filter)
            .await
    }

    pub(crate) async fn add_profile_attribute_validated(
        &self,
        input: CharacterAttributeValueChangeInput,
    ) -> Result<CharacterAttributeMutationResult, CharacterAttributeLifecycleError> {
        self.orchestrator
            .add_profile_attribute_validated(input)
            .await
    }

    pub(crate) async fn modify_profile_attribute_validated(
        &self,
        input: CharacterAttributeValueChangeInput,
    ) -> Result<CharacterAttributeMutationResult, CharacterAttributeLifecycleError> {
        self.orchestrator
            .modify_profile_attribute_validated(input)
            .await
    }

    pub(crate) async fn execute_ready_attribute_deletion_validated(
        &self,
        profile_id: i64,
        category: String,
        key: String,
        chapter_number: Option<i32>,
        cause: Option<String>,
        evidence: String,
    ) -> Result<CharacterAttributeDeletionExecutionResult, CharacterAttributeLifecycleError> {
        self.orchestrator
            .execute_ready_attribute_deletion_validated(
                profile_id,
                category,
                key,
                chapter_number,
                cause,
                evidence,
            )
            .await
    }

    pub(crate) async fn behavior_records_by_profile(
        &self,
        profile_id: i64,
        chapter_from: Option<i32>,
        chapter_to: Option<i32>,
        limit: Option<u64>,
    ) -> Result<Vec<protagonist_behavior_record::Model>, AppError> {
        self.orchestrator
            .behavior_records_by_profile(profile_id, chapter_from, chapter_to, limit)
            .await
    }

    pub(crate) async fn deletion_marks_by_profile_validated(
        &self,
        profile_id: i64,
        category: Option<&str>,
        status: Option<&str>,
        deletion_threshold: i32,
    ) -> Result<Vec<protagonist_deletion_mark::Model>, AppError> {
        self.orchestrator
            .deletion_marks_by_profile_validated(profile_id, category, status, deletion_threshold)
            .await
    }

    pub(crate) async fn request_attribute_deletion_validated(
        &self,
        input: CharacterAttributeDeletionRequestInput,
    ) -> Result<protagonist_deletion_mark::Model, AppError> {
        self.orchestrator
            .request_attribute_deletion_validated(input)
            .await
    }

    pub(crate) async fn reset_deletion_marks_validated(
        &self,
        profile_id: i64,
        category: &str,
        key: &str,
    ) -> Result<u64, AppError> {
        self.orchestrator
            .reset_deletion_marks_validated(profile_id, category, key)
            .await
    }

    pub(crate) async fn snapshot_references_for_user_validated(
        &self,
        user_id: i64,
        profile_id: i64,
        start_chapter: Option<i32>,
        end_chapter: Option<i32>,
    ) -> Result<Vec<snapshot_reference::Model>, AppError> {
        self.orchestrator
            .snapshot_references_for_user_validated(user_id, profile_id, start_chapter, end_chapter)
            .await
    }

    pub(crate) async fn profile_snapshot_for_user_at_chapter_validated(
        &self,
        user_id: i64,
        profile_id: i64,
        chapter_number: i32,
    ) -> Result<snapshot_reference::Model, AppError> {
        self.orchestrator
            .profile_snapshot_for_user_at_chapter_validated(user_id, profile_id, chapter_number)
            .await
    }

    pub(crate) async fn character_snapshot_for_user_by_hash_validated(
        &self,
        user_id: i64,
        content_hash: &str,
    ) -> Result<Snapshot, AppError> {
        self.orchestrator
            .character_snapshot_for_user_by_hash_validated(user_id, content_hash)
            .await
    }

    pub(crate) async fn diff_profile_snapshots_for_user_validated(
        &self,
        user_id: i64,
        profile_id: i64,
        from_chapter: i32,
        to_chapter: i32,
    ) -> Result<SnapshotDelta, AppError> {
        self.orchestrator
            .diff_profile_snapshots_for_user_validated(
                user_id,
                profile_id,
                from_chapter,
                to_chapter,
            )
            .await
    }

    pub(crate) async fn character_sync_conflict_for_user_validated(
        &self,
        user_id: i64,
        profile_id: i64,
    ) -> Result<CharacterSyncConflictReadModel, AppError> {
        self.orchestrator
            .character_sync_conflict_for_user_validated(user_id, profile_id)
            .await
    }

    pub(crate) async fn character_conflict_task_scope_validated(
        &self,
        user_id: i64,
        input: &CharacterConflictCheckInput,
    ) -> Result<CharacterConflictTaskScope, AppError> {
        self.orchestrator
            .character_conflict_task_scope_validated(user_id, input)
            .await
    }

    pub(crate) async fn prepare_character_conflict_request_preview_validated(
        &self,
        user_id: i64,
        input: CharacterConflictCheckInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<CharacterConflictRequestPreview, AppError> {
        self.orchestrator
            .prepare_character_conflict_request_preview_validated(user_id, input, prompt_renderer)
            .await
    }

    pub(crate) async fn check_character_conflict_validated(
        &self,
        user_id: i64,
        input: CharacterConflictCheckInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: CharacterConflictProgressReporter<'_>,
        active_task_id: Option<&str>,
    ) -> Result<Vec<Value>, AppError> {
        self.orchestrator
            .check_character_conflict_validated(
                user_id,
                input,
                prompt_renderer,
                progress,
                active_task_id,
            )
            .await
    }

    pub(crate) async fn delete_character_profile_records_validated(
        &self,
        profile_id: i64,
    ) -> Result<CharacterProfileRecordDeletionResult, CharacterRecordLifecycleError> {
        self.orchestrator
            .delete_character_profile_records_validated(profile_id)
            .await
    }

    pub(crate) async fn rollback_character_profile_timeline_validated(
        &self,
        profile_id: i64,
        chapter_number: i32,
    ) -> Result<CharacterTimelineRollbackLifecycleResult, CharacterRecordLifecycleError> {
        self.orchestrator
            .rollback_character_profile_timeline_validated(profile_id, chapter_number)
            .await
    }

    pub(crate) async fn prepare_generate_portrait_request_preview_validated(
        &self,
        input: &a4n_orchestrator::portrait::GeneratePortraitInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<a4n_llm::provider::ChatRequest, AppError> {
        self.orchestrator
            .prepare_generate_portrait_request_preview_validated(input, prompt_renderer)
            .await
    }

    pub(crate) async fn prepare_generate_portrait_prompt_request_preview_validated(
        &self,
        input: &a4n_orchestrator::portrait::GeneratePortraitPromptInput,
        prompt_renderer: &dyn PromptRenderer,
        variables: &std::collections::HashMap<String, Value>,
    ) -> Result<a4n_llm::provider::ChatRequest, AppError> {
        self.orchestrator
            .prepare_generate_portrait_prompt_request_preview_validated(
                input,
                prompt_renderer,
                variables,
            )
            .await
    }

    pub(crate) async fn generate_portrait_prompt_validated(
        &self,
        input: a4n_orchestrator::portrait::GeneratePortraitPromptInput,
        prompt_renderer: &dyn PromptRenderer,
        variables: &std::collections::HashMap<String, Value>,
    ) -> Result<String, AppError> {
        self.orchestrator
            .generate_portrait_prompt_validated(input, prompt_renderer, variables)
            .await
    }

    pub(crate) async fn upload_portrait_validated(
        &self,
        input: a4n_orchestrator::portrait::UploadPortraitInput,
        host_port: &dyn a4n_orchestrator::portrait::PortraitGenerationHostPort,
    ) -> Result<a4n_orchestrator::portrait::PortraitSummary, AppError> {
        self.orchestrator
            .upload_portrait_validated(input, host_port)
            .await
    }

    pub(crate) async fn prepare_character_profile_sync_preview_validated(
        &self,
        input: CharacterProfileSyncApplicationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<CharacterProfileSyncPreviewStep>, AppError> {
        self.orchestrator
            .prepare_character_profile_sync_preview_validated(input, prompt_renderer)
            .await
    }

    pub(crate) async fn prepare_all_character_profile_sync_preview_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<CharacterProfileSyncPreviewStep>, AppError> {
        self.orchestrator
            .prepare_all_character_profile_sync_preview_validated(
                project_id,
                chapter_number,
                prompt_renderer,
            )
            .await
    }

    pub(crate) async fn run_all_character_profile_sync_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
        prompt_renderer: &dyn PromptRenderer,
        progress: a4n_orchestrator::character::CharacterProfileBatchProgress<'_>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<Value, AppError> {
        self.orchestrator
            .run_all_character_profile_sync_validated(
                project_id,
                chapter_number,
                prompt_renderer,
                progress,
                trace_port,
            )
            .await
    }

    pub(crate) async fn run_character_profile_sync_validated(
        &self,
        input: CharacterProfileSyncApplicationInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: Option<CharacterProfileSyncProgress<'_>>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<Value, AppError> {
        self.orchestrator
            .run_character_profile_sync_validated(input, prompt_renderer, progress, trace_port)
            .await
    }

    pub(crate) async fn character_discovery_task_scope_validated(
        &self,
        input: &CharacterDiscoveryInput,
    ) -> Result<CharacterDiscoveryTaskScope, AppError> {
        self.orchestrator
            .character_discovery_task_scope_validated(input)
            .await
    }

    pub(crate) async fn prepare_character_discovery_request_preview_validated(
        &self,
        input: CharacterDiscoveryInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<CharacterDiscoveryRequestPreview, AppError> {
        self.orchestrator
            .prepare_character_discovery_request_preview_validated(input, prompt_renderer)
            .await
    }

    pub(crate) async fn discover_character_profiles_validated(
        &self,
        input: CharacterDiscoveryInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: Option<CharacterDiscoveryProgress<'_>>,
    ) -> Result<Value, AppError> {
        self.orchestrator
            .discover_character_profiles_validated(input, prompt_renderer, progress)
            .await
    }

    pub(crate) async fn implicit_review_summary_validated(
        &self,
        user_id: i64,
        input: GetImplicitReviewInput,
    ) -> Result<ImplicitReviewSummary, AppError> {
        self.orchestrator
            .implicit_review_summary_validated(user_id, input)
            .await
    }

    pub(crate) async fn prepare_implicit_update_review_request_preview_validated(
        &self,
        user_id: i64,
        input: CheckImplicitUpdateInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ImplicitUpdateReviewRequestPreview, AppError> {
        self.orchestrator
            .prepare_implicit_update_review_request_preview_validated(
                user_id,
                input,
                prompt_renderer,
            )
            .await
    }

    pub(crate) async fn check_implicit_update_validated(
        &self,
        user_id: i64,
        input: CheckImplicitUpdateInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ImplicitUpdateDecisionSummary, AppError> {
        self.orchestrator
            .check_implicit_update_validated(user_id, input, prompt_renderer)
            .await
    }

    pub(crate) async fn apply_implicit_update_decision_validated(
        &self,
        user_id: i64,
        input: ApplyImplicitUpdateDecisionInput,
    ) -> Result<ApplyImplicitUpdateDecisionResultSummary, AppError> {
        self.orchestrator
            .apply_implicit_update_decision_validated(user_id, input)
            .await
    }
}
