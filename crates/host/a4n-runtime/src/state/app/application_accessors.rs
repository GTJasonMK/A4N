use super::*;

impl AppState {
    /// Host 窗口/路径初始化用配置视图。
    pub fn host_app_config(&self) -> &a4n_config::app_config::AppConfig {
        &self.config
    }

    pub(crate) fn runtime_profile(&self) -> RuntimeProfile {
        self.runtime_profile
    }

    pub(crate) fn capabilities(&self) -> &RuntimeCapabilities {
        &self.capabilities
    }

    pub(crate) fn log_dir(&self) -> &Path {
        &self.config.log_dir
    }

    pub(crate) fn temp_dir(&self) -> &Path {
        &self.config.temp_dir
    }

    pub(crate) fn embedding_models_dir(&self) -> std::path::PathBuf {
        self.config.embedding_models_dir()
    }

    pub(crate) fn required_jwt_secret(&self) -> Result<&str, a4n_error::AppError> {
        self.config
            .security
            .jwt_secret
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| a4n_error::AppError::InvalidState {
                current: "jwt_secret 未配置".to_string(),
                operation: "读取 JWT 密钥".to_string(),
            })
    }

    pub fn subscribe_internal_events(&self) -> tokio::sync::broadcast::Receiver<InternalEvent> {
        self.event_bus.subscribe()
    }

    pub(crate) fn event_bus(&self) -> Arc<BroadcastEventBus> {
        self.event_bus.clone()
    }

    pub(crate) fn auth_read_application(
        &self,
    ) -> &dyn a4n_api_contract::auth::AuthReadApplicationPort {
        self.auth_read_application.as_ref()
    }

    pub(crate) fn auth_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::auth::AuthMutationApplicationPort {
        self.auth_mutation_application.as_ref()
    }

    pub(crate) fn admin_read_application(
        &self,
    ) -> &dyn a4n_api_contract::admin::AdminReadApplicationPort {
        self.admin_read_application.as_ref()
    }

    pub(crate) fn model_usage_read_application(
        &self,
    ) -> &dyn a4n_api_contract::cost::ModelUsageReadApplicationPort {
        self.model_usage_read_application.as_ref()
    }

    pub(crate) fn admin_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::admin::AdminMutationApplicationPort {
        self.admin_mutation_application.as_ref()
    }

    pub(crate) fn notification_application(
        &self,
    ) -> &dyn a4n_api_contract::notifications::NotificationApplicationPort {
        self.notification_application.as_ref()
    }

    pub(crate) fn test_console(&self) -> &crate::actions::admin_records::TestConsole {
        self.test_console.as_ref()
    }

    pub(crate) fn project_status_read_application(
        &self,
    ) -> &dyn a4n_api_contract::project::ProjectStatusReadApplicationPort {
        self.project_status_read_application.as_ref()
    }

    pub(crate) fn project_status_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::project::ProjectStatusMutationApplicationPort {
        self.project_status_mutation_application.as_ref()
    }

    pub(crate) fn project_collaboration_application(
        &self,
    ) -> &dyn a4n_api_contract::collaboration::ProjectCollaborationApplicationPort {
        self.project_collaboration_application.as_ref()
    }

    pub(crate) fn project_access_application(
        &self,
    ) -> &dyn a4n_api_contract::collaboration::ProjectAccessApplicationPort {
        self.project_access_application.as_ref()
    }

    pub(crate) fn project_read_application(
        &self,
    ) -> &dyn a4n_api_contract::project::ProjectReadApplicationPort {
        self.project_read_application.as_ref()
    }

    pub(crate) fn project_source_snapshot_read_application(
        &self,
    ) -> &dyn a4n_api_contract::project::ProjectSourceSnapshotReadApplicationPort {
        self.project_source_snapshot_read_application.as_ref()
    }

    pub(crate) fn project_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::project::ProjectMutationApplicationPort {
        self.project_mutation_application.as_ref()
    }

    pub(crate) fn project_asset_graph_application(
        &self,
    ) -> &dyn a4n_api_contract::project::ProjectAssetGraphApplicationPort {
        self.project_asset_graph_application.as_ref()
    }

    pub(crate) fn project_duplication_application(
        &self,
    ) -> &dyn a4n_api_contract::project::ProjectDuplicationApplicationPort {
        self.project_duplication_application.as_ref()
    }

    pub(crate) fn project_family_transfer_application(
        &self,
    ) -> &dyn a4n_api_contract::project_transfer::ProjectFamilyTransferApplicationPort {
        self.project_family_transfer_application.as_ref()
    }

    pub(crate) fn project_bundle_transfer_application(
        &self,
    ) -> &dyn a4n_api_contract::project_transfer::ProjectBundleTransferApplicationPort {
        self.project_bundle_transfer_application.as_ref()
    }

    pub(crate) fn canvas_project_transfer_application(
        &self,
    ) -> &dyn a4n_api_contract::project_transfer::CanvasProjectTransferApplicationPort {
        self.canvas_project_transfer_application.as_ref()
    }

    pub(crate) fn chapter_scene_read_application(
        &self,
    ) -> &dyn a4n_api_contract::chapter::ChapterSceneReadApplicationPort {
        self.chapter_scene_read_application.as_ref()
    }

    pub(crate) fn chapter_scene_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::chapter::ChapterSceneMutationApplicationPort {
        self.chapter_scene_mutation_application.as_ref()
    }

    pub(crate) fn chapter_scene_generation_application(
        &self,
    ) -> &dyn a4n_novel_host_contract::ChapterSceneGenerationApplicationPort {
        self.chapter_scene_generation_application.as_ref()
    }

    pub(crate) fn optimize_chat_generation_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizeChatGenerationApplicationPort {
        self.optimize_chat_generation_application.as_ref()
    }

    pub(crate) fn optimize_analysis_generation_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizeAnalysisGenerationApplicationPort {
        self.optimize_analysis_generation_application.as_ref()
    }

    pub(crate) fn optimize_patch_generation_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizePatchGenerationApplicationPort {
        self.optimize_patch_generation_application.as_ref()
    }

    pub(crate) fn optimize_session_read_application(
        &self,
    ) -> &dyn a4n_api_contract::optimize::OptimizeSessionReadApplicationPort {
        self.optimize_session_read_application.as_ref()
    }

    pub(crate) fn optimize_agent_tool_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizeAgentToolApplicationPort {
        self.optimize_agent_tool_application.as_ref()
    }

    pub(crate) fn optimize_session_mutation_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizeSessionMutationApplicationPort {
        self.optimize_session_mutation_application.as_ref()
    }

    pub(crate) fn optimize_patch_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizePatchApplicationPort {
        self.optimize_patch_application.as_ref()
    }

    pub(crate) fn optimize_patch_preview_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizePatchPreviewApplicationPort {
        self.optimize_patch_preview_application.as_ref()
    }

    pub(crate) fn optimize_analysis_preview_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizeAnalysisPreviewApplicationPort {
        self.optimize_analysis_preview_application.as_ref()
    }

    pub(crate) fn optimize_chat_preview_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizeChatPreviewApplicationPort {
        self.optimize_chat_preview_application.as_ref()
    }

    pub(crate) fn scene_profile_sync_application(
        &self,
    ) -> &dyn a4n_scene_host_contract::SceneProfileSyncApplicationPort {
        self.scene_profile_sync_application.as_ref()
    }

    pub(crate) fn service_health_application(
        &self,
    ) -> &dyn a4n_api_contract::service_health::ServiceHealthApplicationPort {
        self.service_health_application.as_ref()
    }

    pub(crate) fn cleanup_outbox_application(
        &self,
    ) -> &dyn a4n_cleanup_contract::CleanupOutboxApplicationPort {
        self.cleanup_outbox_application.as_ref()
    }

    pub(crate) fn character_discovery_application(
        &self,
    ) -> &dyn a4n_character_host_contract::CharacterDiscoveryApplicationPort {
        self.character_discovery_application.as_ref()
    }

    pub(crate) fn character_profile_sync_application(
        &self,
    ) -> &dyn a4n_character_host_contract::CharacterProfileSyncApplicationPort {
        self.character_profile_sync_application.as_ref()
    }

    pub(crate) fn workflow_read_application(
        &self,
    ) -> &dyn a4n_api_contract::workflow::WorkflowReadApplicationPort {
        self.workflow_read_application.as_ref()
    }

    pub(crate) fn workflow_task_lookup_application(
        &self,
    ) -> &dyn a4n_api_contract::workflow::WorkflowTaskLookupApplicationPort {
        self.workflow_task_lookup_application.as_ref()
    }

    pub(crate) fn task_read_application(
        &self,
    ) -> &dyn a4n_api_contract::task::TaskReadApplicationPort {
        self.task_read_application.as_ref()
    }

    pub(crate) fn task_cancellation_application(
        &self,
    ) -> &dyn a4n_api_contract::task::TaskCancellationPort {
        self.application_ports
            .task_cancellation_application
            .as_ref()
    }

    pub(crate) fn task_queue_stats_application(
        &self,
    ) -> &dyn a4n_api_contract::task::TaskQueueStatsApplicationPort {
        self.application_ports.task_queue_stats_application.as_ref()
    }

    pub(crate) fn workflow_cancellation_application(
        &self,
    ) -> &dyn a4n_api_contract::workflow::WorkflowCancellationPort {
        self.workflow_cancellation_application.as_ref()
    }

    pub(crate) fn object_profile_read_application(
        &self,
    ) -> &dyn a4n_api_contract::profile::ObjectProfileReadApplicationPort {
        self.object_profile_read_application.as_ref()
    }

    pub(crate) fn object_profile_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::profile::ObjectProfileMutationApplicationPort {
        self.object_profile_mutation_application.as_ref()
    }

    pub(crate) fn scene_profile_read_application(
        &self,
    ) -> &dyn a4n_api_contract::profile::SceneProfileReadApplicationPort {
        self.scene_profile_read_application.as_ref()
    }

    pub(crate) fn scene_profile_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::profile::SceneProfileMutationApplicationPort {
        self.scene_profile_mutation_application.as_ref()
    }

    pub(crate) fn scene_profile_record_sync_application(
        &self,
    ) -> Arc<dyn a4n_api_contract::profile::SceneProfileRecordSyncApplicationPort> {
        self.scene_profile_record_sync_application.clone()
    }

    pub(crate) fn character_profile_overview_read_application(
        &self,
    ) -> &dyn a4n_api_contract::character::CharacterProfileOverviewReadApplicationPort {
        self.character_profile_overview_read_application.as_ref()
    }

    pub(crate) fn character_timeline_read_application(
        &self,
    ) -> &dyn a4n_api_contract::character::CharacterTimelineReadApplicationPort {
        self.character_timeline_read_application.as_ref()
    }

    pub(crate) fn character_profile_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::character::CharacterProfileMutationApplicationPort {
        self.character_profile_mutation_application.as_ref()
    }

    pub(crate) fn character_profile_timeline_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::character::CharacterProfileTimelineMutationApplicationPort {
        self.character_profile_timeline_mutation_application
            .as_ref()
    }

    pub(crate) fn character_profile_attribute_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::character::CharacterProfileAttributeMutationApplicationPort {
        self.character_profile_attribute_mutation_application
            .as_ref()
    }

    pub(crate) fn character_profile_attribute_deletion_application(
        &self,
    ) -> &dyn a4n_api_contract::character::CharacterProfileAttributeDeletionApplicationPort {
        self.character_profile_attribute_deletion_application
            .as_ref()
    }

    pub(crate) fn character_implicit_review_check_application(
        &self,
    ) -> &dyn a4n_character_host_contract::CharacterImplicitReviewCheckApplicationPort {
        self.character_implicit_review_check_application.as_ref()
    }

    pub(crate) fn character_implicit_review_preview_application(
        &self,
    ) -> &dyn a4n_character_host_contract::CharacterImplicitReviewPreviewApplicationPort {
        self.character_implicit_review_preview_application.as_ref()
    }

    pub(crate) fn character_implicit_review_apply_application(
        &self,
    ) -> &dyn a4n_character_host_contract::CharacterImplicitReviewApplyApplicationPort {
        self.character_implicit_review_apply_application.as_ref()
    }

    pub(crate) fn character_implicit_review_read_application(
        &self,
    ) -> &dyn a4n_api_contract::character::CharacterImplicitReviewReadApplicationPort {
        self.character_implicit_review_read_application.as_ref()
    }

    pub(crate) fn character_profile_deletion_application(
        &self,
    ) -> &dyn a4n_api_contract::character::CharacterProfileDeletionApplicationPort {
        self.character_profile_deletion_application.as_ref()
    }

    pub(crate) fn character_conflict_application(
        &self,
    ) -> &dyn a4n_character_host_contract::CharacterConflictApplicationPort {
        self.character_conflict_application.as_ref()
    }

    pub(crate) fn foreshadowing_read_application(
        &self,
    ) -> &dyn a4n_api_contract::profile::ForeshadowingReadApplicationPort {
        self.foreshadowing_read_application.as_ref()
    }

    pub(crate) fn foreshadowing_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::profile::ForeshadowingMutationApplicationPort {
        self.foreshadowing_mutation_application.as_ref()
    }

    pub(crate) fn blueprint_read_application(
        &self,
    ) -> &dyn a4n_api_contract::blueprint::BlueprintReadApplicationPort {
        self.blueprint_read_application.as_ref()
    }

    pub(crate) fn blueprint_generation_preview_application(
        &self,
    ) -> &dyn a4n_blueprint_contract::BlueprintGenerationPreviewApplicationPort {
        self.blueprint_generation_preview_application.as_ref()
    }

    pub(crate) fn blueprint_refinement_preview_application(
        &self,
    ) -> &dyn a4n_blueprint_contract::BlueprintRefinementPreviewApplicationPort {
        self.blueprint_refinement_preview_application.as_ref()
    }

    pub(crate) fn blueprint_generation_application(
        &self,
    ) -> &dyn a4n_blueprint_contract::BlueprintGenerationApplicationPort {
        self.blueprint_generation_application.as_ref()
    }

    pub(crate) fn blueprint_refinement_application(
        &self,
    ) -> &dyn a4n_blueprint_contract::BlueprintRefinementApplicationPort {
        self.blueprint_refinement_application.as_ref()
    }

    pub(crate) fn blueprint_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::blueprint::BlueprintMutationApplicationPort {
        self.blueprint_mutation_application.as_ref()
    }

    pub(crate) fn galgame_read_application(
        &self,
    ) -> &dyn a4n_api_contract::galgame::GalgameReadApplicationPort {
        self.galgame_read_application.as_ref()
    }

    pub(crate) fn galgame_story_graph_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::galgame::GalgameStoryGraphMutationApplicationPort {
        self.galgame_story_graph_mutation_application.as_ref()
    }

    pub(crate) fn inspiration_history_read_application(
        &self,
    ) -> &dyn a4n_api_contract::inspiration::InspirationHistoryReadApplicationPort {
        self.inspiration_history_read_application.as_ref()
    }

    pub(crate) fn inspiration_generation_application(
        &self,
    ) -> &dyn a4n_inspiration_host_contract::InspirationGenerationApplicationPort {
        self.inspiration_generation_application.as_ref()
    }

    pub(crate) fn inspiration_request_application(
        &self,
    ) -> &dyn a4n_inspiration_host_contract::InspirationRequestPreviewApplicationPort {
        self.inspiration_request_application.as_ref()
    }

    pub(crate) fn package_read_application(
        &self,
    ) -> &dyn a4n_api_contract::package::PackageReadApplicationPort {
        self.package_read_application.as_ref()
    }

    pub(crate) fn package_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::package::PackageMutationApplicationPort {
        self.package_mutation_application.as_ref()
    }

    pub(crate) fn import_analysis_generation_application(
        &self,
    ) -> &dyn a4n_import_analysis_host_contract::ImportAnalysisGenerationApplicationPort {
        self.import_analysis_generation_application.as_ref()
    }

    pub(crate) fn import_analysis_cancellation_application(
        &self,
    ) -> &dyn a4n_api_contract::import::ImportAnalysisCancellationApplicationPort {
        self.import_analysis_cancellation_application.as_ref()
    }

    pub(crate) fn import_analysis_progress_application(
        &self,
    ) -> &dyn a4n_api_contract::import::ImportAnalysisProgressApplicationPort {
        self.import_analysis_progress_application.as_ref()
    }

    pub(crate) fn import_analysis_preview_application(
        &self,
    ) -> &dyn a4n_import_analysis_host_contract::ImportAnalysisPreviewApplicationPort {
        self.import_analysis_preview_application.as_ref()
    }

    pub(crate) fn import_reverse_preview_application(
        &self,
    ) -> &dyn a4n_novel_host_contract::ImportReversePreviewApplicationPort {
        self.import_reverse_preview_application.as_ref()
    }

    pub(crate) fn import_reverse_generation_application(
        &self,
    ) -> &dyn a4n_novel_host_contract::ImportReverseGenerationApplicationPort {
        self.import_reverse_generation_application.as_ref()
    }

    pub(crate) fn import_reverse_confirmation_application(
        &self,
    ) -> &dyn a4n_api_contract::import_reverse::ImportReverseConfirmationApplicationPort {
        self.import_reverse_confirmation_application.as_ref()
    }

    pub(crate) fn import_reverse_read_application(
        &self,
    ) -> &dyn a4n_api_contract::import_reverse::ImportReverseReadApplicationPort {
        self.import_reverse_read_application.as_ref()
    }

    pub(crate) fn chapter_analysis_generation_application(
        &self,
    ) -> &dyn a4n_chapter_analysis_host_contract::ChapterAnalysisGenerationApplicationPort {
        self.chapter_analysis_generation_application.as_ref()
    }

    pub(crate) fn project_fact_feedback_application(
        &self,
    ) -> &dyn a4n_chapter_analysis_host_contract::ProjectFactFeedbackApplicationPort {
        self.project_fact_feedback_application.as_ref()
    }

    pub(crate) fn chapter_analysis_resume_application(
        &self,
    ) -> &dyn a4n_chapter_analysis_host_contract::ChapterAnalysisResumeApplicationPort {
        self.chapter_analysis_resume_application.as_ref()
    }

    pub(crate) fn chapter_analysis_generation_preview_application(
        &self,
    ) -> &dyn a4n_chapter_analysis_host_contract::ChapterAnalysisGenerationPreviewApplicationPort
    {
        self.chapter_analysis_generation_preview_application
            .as_ref()
    }
}
