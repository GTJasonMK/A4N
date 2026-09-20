use super::builders::*;
use serde_json::Value;
pub(crate) fn result_json_value() -> Value {
    let mut schema = raw_schema::<Value>();
    schema
        .as_object_mut()
        .expect("serde_json::Value schema must be an object")
        .insert(
            "x-a4n-schema-quality".into(),
            Value::String("json-value".into()),
        );
    schema
}

pub(crate) fn result_optional_json_value() -> Value {
    typed_schema::<Option<Value>>()
}

pub(crate) fn result_character_conflicts() -> Value {
    typed_schema::<Vec<crate::character::CharacterConflictSummary>>()
}

pub(crate) fn result_discover_character_profiles() -> Value {
    typed_schema::<crate::character::DiscoverCharacterProfilesResult>()
}

pub(crate) fn result_invite_code() -> Value {
    typed_schema::<crate::admin::InviteCodeSummary>()
}

pub(crate) fn result_invite_codes() -> Value {
    typed_schema::<Vec<crate::admin::InviteCodeSummary>>()
}

pub(crate) fn result_users() -> Value {
    typed_schema::<Vec<crate::auth::UserSummary>>()
}

pub(crate) fn result_project_generated_data_reset() -> Value {
    typed_schema::<crate::project::ProjectGeneratedDataResetResponse>()
}

pub(crate) fn result_video_segment_plan_draft() -> Value {
    typed_schema::<crate::video::VideoSegmentPlanDraftSummary>()
}

pub(crate) fn result_rag_enhanced_search() -> Value {
    typed_schema::<crate::rag::RagEnhancedSearchOutput>()
}

pub(crate) fn result_rag_prepare_enhanced_search() -> Value {
    typed_schema::<crate::rag::RagPreparedEnhancedSearchOutput>()
}

pub(crate) fn result_rag_ingest() -> Value {
    typed_schema::<crate::rag::RagIngestResult>()
}

pub(crate) fn result_rag_prepare_ingest_all() -> Value {
    typed_schema::<crate::rag::ClientRagIngestPrepareOutput>()
}

pub(crate) fn result_prepare_chapter_generation_rag_embedding() -> Value {
    typed_schema::<crate::chapter::PreparedGenerationRagEmbeddingRequest>()
}

pub(crate) fn result_unit() -> Value {
    typed_schema::<()>()
}

pub(crate) fn result_source_snapshot_chapters() -> Value {
    typed_schema::<Vec<crate::project::SourceSnapshotChapterSummary>>()
}

pub(crate) fn result_source_sync_plan() -> Value {
    typed_schema::<crate::source_sync::SourceSyncPlanSummary>()
}

pub(crate) fn result_activate_theme() -> Value {
    typed_schema::<crate::theme::ThemeSummary>()
}

pub(crate) fn result_theme_preference() -> Value {
    typed_schema::<crate::theme::ThemePreferenceSummary>()
}

pub(crate) fn result_theme_resource() -> Value {
    typed_schema::<crate::theme::ThemeResourceSummary>()
}

pub(crate) fn result_theme_package_export() -> Value {
    typed_schema::<crate::theme::ThemePackageExport>()
}

pub(crate) fn result_add_image_config() -> Value {
    typed_schema::<crate::config::ImageConfigSummary>()
}

pub(crate) fn result_add_video_config() -> Value {
    typed_schema::<crate::config::VideoConfigSummary>()
}

pub(crate) fn result_assembled_chapter_video_preview() -> Value {
    typed_schema::<crate::video::AssembledChapterVideoPreview>()
}

pub(crate) fn result_assembly_push() -> Value {
    typed_schema::<crate::video::AssemblyPushResult>()
}

pub(crate) fn result_apply_implicit_update_decision() -> Value {
    typed_schema::<crate::character::ApplyImplicitUpdateDecisionResultSummary>()
}

pub(crate) fn result_apply_optimization_patches() -> Value {
    typed_schema::<crate::optimize::ApplyOptimizationPatchesResponse>()
}

pub(crate) fn result_attribute_history() -> Value {
    typed_schema::<Vec<crate::character::AttributeEventSummary>>()
}

pub(crate) fn result_bind_galgame_media() -> Value {
    typed_schema::<crate::galgame::GalgameMediaBindingSummary>()
}

pub(crate) fn result_bind_manga_reference_image() -> Value {
    typed_schema::<crate::manga::MangaPagePromptSummary>()
}

pub(crate) fn result_cancel_galgame_pipeline() -> Value {
    typed_schema::<crate::galgame::GalgamePipelineStatusSummary>()
}

pub(crate) fn result_cancel_manga_pipeline() -> Value {
    typed_schema::<i64>()
}

pub(crate) fn result_chapter_summary() -> Value {
    typed_schema::<crate::chapter::ChapterSummary>()
}

pub(crate) fn result_chapter_summaries() -> Value {
    typed_schema::<Vec<crate::chapter::ChapterSummary>>()
}

pub(crate) fn result_chapter_analysis_mutation() -> Value {
    typed_schema::<crate::chapter::ChapterAnalysisMutationResult>()
}

pub(crate) fn result_chapter_scene() -> Value {
    typed_schema::<crate::chapter::ChapterSceneSummary>()
}

pub(crate) fn result_chapter_scenes() -> Value {
    typed_schema::<Vec<crate::chapter::ChapterSceneSummary>>()
}

pub(crate) fn result_chapter_outline_pacing() -> Value {
    typed_schema::<crate::outline::ChapterOutlinePacing>()
}

pub(crate) fn result_chapter_outlines() -> Value {
    typed_schema::<Vec<crate::outline::ChapterOutlineSummary>>()
}

pub(crate) fn result_check_implicit_update() -> Value {
    typed_schema::<crate::character::ImplicitUpdateDecisionSummary>()
}

pub(crate) fn result_check_profile_sync_conflict() -> Value {
    typed_schema::<crate::character::ProfileSyncConflictSummary>()
}

pub(crate) fn result_cleanup_orphan_project_data_dirs() -> Value {
    typed_schema::<crate::storage::OrphanProjectDataDirsSummary>()
}

pub(crate) fn result_create_embedding_config() -> Value {
    typed_schema::<crate::config::EmbeddingConfigSummary>()
}

pub(crate) fn result_create_llm_config() -> Value {
    typed_schema::<crate::config::LlmConfigSummary>()
}

pub(crate) fn result_create_profile() -> Value {
    typed_schema::<crate::character::CharacterProfileCreateResult>()
}

pub(crate) fn result_create_project() -> Value {
    typed_schema::<crate::project::ProjectSummary>()
}

pub(crate) fn result_object_profile_mutation() -> Value {
    typed_schema::<crate::profile::ObjectProfileMutationResult>()
}

pub(crate) fn result_foreshadowing_write() -> Value {
    typed_schema::<crate::profile::ForeshadowingWriteResult>()
}

pub(crate) fn result_scene_profile_mutation() -> Value {
    typed_schema::<crate::profile::SceneProfileMutationResult>()
}

pub(crate) fn result_export_project_package_data() -> Value {
    typed_schema::<crate::project_transfer::ExportProjectPackageData>()
}

pub(crate) fn result_project_bundle_import_report() -> Value {
    typed_schema::<crate::project_transfer::ProjectBundleImportReport>()
}

pub(crate) fn result_delete_profile() -> Value {
    typed_schema::<crate::character::CharacterProfileDeleteResult>()
}

pub(crate) fn result_object_profile_delete() -> Value {
    typed_schema::<crate::profile::ObjectProfileDeleteResult>()
}

pub(crate) fn result_scene_profile_delete() -> Value {
    typed_schema::<crate::profile::SceneProfileDeleteResult>()
}

pub(crate) fn result_detect_image_config_models() -> Value {
    typed_schema::<crate::config::ModelDiscoveryResult>()
}

pub(crate) fn result_diff_blueprint_versions() -> Value {
    typed_schema::<crate::blueprint::BlueprintDiffSummary>()
}

pub(crate) fn result_diff_profile_chapters() -> Value {
    typed_schema::<crate::character::SnapshotDeltaSummary>()
}

pub(crate) fn result_download_embedding_model() -> Value {
    typed_schema::<crate::embedding::LocalEmbeddingModelSummary>()
}

pub(crate) fn result_edit_chapter_shooting_script() -> Value {
    typed_schema::<crate::shooting_script::EditedChapterShootingScriptView>()
}

pub(crate) fn result_chapter_visual_plan() -> Value {
    typed_schema::<crate::chapter_visual_plan::ChapterVisualPlanView>()
}

pub(crate) fn result_chapter_visual_plan_set() -> Value {
    typed_schema::<crate::chapter_visual_plan::ChapterVisualPlanSetView>()
}

pub(crate) fn result_read_chapter_visual_plan() -> Value {
    typed_schema::<crate::chapter_visual_plan::ChapterVisualPlanReadResult>()
}

pub(crate) fn result_edit_chapter_visual_plan_draft() -> Value {
    typed_schema::<crate::chapter_visual_plan::ChapterVisualPlanEditResult>()
}

pub(crate) fn result_validate_chapter_visual_plan() -> Value {
    typed_schema::<crate::chapter_visual_plan::ChapterVisualPlanValidationResult>()
}

pub(crate) fn result_confirmed_chapter_visual_plan() -> Value {
    typed_schema::<crate::chapter_visual_plan::ConfirmedChapterVisualPlanView>()
}

pub(crate) fn result_export_batch_data() -> Value {
    typed_schema::<Vec<crate::export::ExportFileData>>()
}

pub(crate) fn result_export_characters_json_data() -> Value {
    typed_schema::<crate::export::ExportFileData>()
}

pub(crate) fn result_string() -> Value {
    typed_schema::<String>()
}

pub(crate) fn result_import_all_configs() -> Value {
    typed_schema::<Vec<String>>()
}

pub(crate) fn result_generate_chapter_shooting_script() -> Value {
    typed_schema::<crate::shooting_script::ChapterShootingScriptView>()
}

pub(crate) fn result_generate_next_chapter_shooting_script_unit() -> Value {
    typed_schema::<crate::shooting_script::ChapterShootingScriptUnitProgressView>()
}

pub(crate) fn result_get_admin_overview() -> Value {
    typed_schema::<crate::admin::AdminOverviewSummary>()
}

pub(crate) fn result_get_blueprint() -> Value {
    typed_schema::<Option<crate::blueprint::BlueprintSummary>>()
}

pub(crate) fn result_get_canvas_graph() -> Value {
    typed_schema::<crate::canvas::CanvasGraphDocument>()
}

pub(crate) fn result_get_canvas_node_contracts() -> Value {
    typed_schema::<Vec<crate::canvas::CanvasNodeContract>>()
}

pub(crate) fn result_get_chapter_shooting_script() -> Value {
    typed_schema::<Option<crate::shooting_script::ChapterShootingScriptView>>()
}

pub(crate) fn result_get_chapter_shooting_script_unit_progress() -> Value {
    typed_schema::<Option<crate::shooting_script::ChapterShootingScriptUnitProgressView>>()
}

pub(crate) fn result_get_current_user() -> Value {
    typed_schema::<crate::auth::UserSummary>()
}

pub(crate) fn result_get_embedding_model_download_info() -> Value {
    typed_schema::<crate::embedding::EmbeddingModelDownloadInfoSummary>()
}

pub(crate) fn result_get_galgame_script() -> Value {
    typed_schema::<crate::galgame::GalgameScriptDetail>()
}

pub(crate) fn result_get_generation_call_analytics() -> Value {
    typed_schema::<crate::admin::GenerationCallAnalytics>()
}

pub(crate) fn result_get_model_usage_report() -> Value {
    typed_schema::<crate::cost::ModelUsageReport>()
}

pub(crate) fn result_get_implicit_review() -> Value {
    typed_schema::<crate::character::ImplicitReviewSummary>()
}

pub(crate) fn result_get_profile_state_at_chapter() -> Value {
    typed_schema::<crate::character::ProfileChapterStateSummary>()
}

pub(crate) fn result_get_snapshot() -> Value {
    typed_schema::<crate::character::SnapshotSummary>()
}

pub(crate) fn result_web_search() -> Value {
    typed_schema::<crate::web_search::WebSearchResult>()
}

pub(crate) fn result_web_fetch() -> Value {
    typed_schema::<crate::web_fetch::WebFetchResult>()
}

pub(crate) fn result_read_image() -> Value {
    typed_schema::<crate::image_read::ReadImageResult>()
}

pub(crate) fn result_get_manga_pipeline_status() -> Value {
    typed_schema::<crate::manga::MangaPipelineStatusSummary>()
}

pub(crate) fn result_list_manga_pipeline_statuses() -> Value {
    typed_schema::<Vec<crate::manga::MangaPipelineStatusSummary>>()
}

pub(crate) fn result_get_primary_portrait() -> Value {
    typed_schema::<Option<crate::portrait::PortraitSummary>>()
}

pub(crate) fn result_get_project_asset_dependency_graph() -> Value {
    typed_schema::<crate::project::AssetDependencyGraph>()
}

pub(crate) fn result_get_project_asset_dependency_graph_node_detail() -> Value {
    typed_schema::<crate::project::AssetGraphNodeDetail>()
}

pub(crate) fn result_get_project_status_machine() -> Value {
    typed_schema::<crate::project::ProjectStatusMachineSummary>()
}

pub(crate) fn result_package_command_availability() -> Value {
    typed_schema::<Vec<crate::package::PackageCommandAvailability>>()
}

pub(crate) fn result_package_lifecycle() -> Value {
    typed_schema::<crate::package::PackageLifecycleSummary>()
}

pub(crate) fn result_package_overview() -> Value {
    typed_schema::<Vec<crate::package::PackageOverview>>()
}

pub(crate) fn result_package_preferences() -> Value {
    typed_schema::<crate::package::PackagePreferencesSummary>()
}

pub(crate) fn result_get_queue_stats() -> Value {
    typed_schema::<crate::task::QueueStatsSummary>()
}

pub(crate) fn result_get_storage_paths() -> Value {
    typed_schema::<crate::storage::StoragePathsSummary>()
}

pub(crate) fn result_get_task_history() -> Value {
    typed_schema::<Vec<crate::task::TaskInfoSummary>>()
}

pub(crate) fn result_get_usage_trends() -> Value {
    typed_schema::<Vec<crate::admin::DailyUsagePoint>>()
}

pub(crate) fn result_get_user_usage() -> Value {
    typed_schema::<Vec<crate::admin::UsageLogSummary>>()
}

pub(crate) fn result_get_workflow_progress_events() -> Value {
    typed_schema::<crate::workflow::WorkflowProgressEventsResponse>()
}

pub(crate) fn result_get_workflow_run_detail() -> Value {
    typed_schema::<crate::workflow::WorkflowRunDetailSummary>()
}

pub(crate) fn result_import_canvas_media() -> Value {
    typed_schema::<crate::canvas::ImportedCanvasMedia>()
}

pub(crate) fn result_dedup() -> Value {
    typed_schema::<crate::import_review::DedupResult>()
}

pub(crate) fn result_import_review() -> Value {
    typed_schema::<crate::import_review::ImportReviewSummary>()
}

pub(crate) fn result_import_reviews() -> Value {
    typed_schema::<Vec<crate::import_review::ImportReviewSummary>>()
}

pub(crate) fn result_import_analysis_progress() -> Value {
    typed_schema::<Option<crate::import::ProjectImportAnalysisProgress>>()
}

pub(crate) fn result_import_analysis_summary() -> Value {
    typed_schema::<crate::import::ImportProjectAnalysisSummary>()
}

pub(crate) fn result_reverse_engineering_candidate() -> Value {
    typed_schema::<crate::import_reverse::ReverseEngineeringCandidate>()
}

pub(crate) fn result_optional_reverse_engineering_candidate() -> Value {
    typed_schema::<Option<crate::import_reverse::ReverseEngineeringCandidate>>()
}

pub(crate) fn result_insert_galgame_line() -> Value {
    typed_schema::<crate::galgame::GalgameLine>()
}

pub(crate) fn result_list_active_workflows() -> Value {
    typed_schema::<Vec<crate::workflow::WorkflowStateSummary>>()
}

pub(crate) fn result_list_workflow_runs() -> Value {
    typed_schema::<Vec<crate::workflow::WorkflowRunSummary>>()
}

pub(crate) fn result_list_all_projects() -> Value {
    typed_schema::<Vec<crate::admin::AdminProjectSummary>>()
}

pub(crate) fn result_list_blueprint_versions() -> Value {
    typed_schema::<Vec<crate::blueprint::BlueprintVersionSummary>>()
}

pub(crate) fn result_list_behaviors() -> Value {
    typed_schema::<Vec<crate::character::BehaviorRecordSummary>>()
}

pub(crate) fn result_list_chapter_video_prompts() -> Value {
    typed_schema::<Vec<crate::video::ChapterVideoPromptSummary>>()
}

pub(crate) fn result_video_timeline() -> Value {
    typed_schema::<crate::video::VideoTimelineSummary>()
}

pub(crate) fn result_video_segment_execution_recipes() -> Value {
    typed_schema::<Vec<crate::video::VideoSegmentExecutionRecipeV1>>()
}

pub(crate) fn result_list_embedding_config() -> Value {
    typed_schema::<Vec<crate::config::EmbeddingConfigSummary>>()
}

pub(crate) fn result_list_deletion_marks() -> Value {
    typed_schema::<Vec<crate::character::DeletionMarkSummary>>()
}

pub(crate) fn result_list_image_configs() -> Value {
    typed_schema::<Vec<crate::config::ImageConfigSummary>>()
}

pub(crate) fn result_inspiration_history() -> Value {
    typed_schema::<Vec<crate::inspiration::InspirationMessageSummary>>()
}

pub(crate) fn result_list_images_by_chapter() -> Value {
    typed_schema::<Vec<crate::image::GeneratedImageSummary>>()
}

pub(crate) fn result_list_object_profiles() -> Value {
    typed_schema::<Vec<crate::profile::ObjectProfileSummary>>()
}

pub(crate) fn result_list_foreshadowing() -> Value {
    typed_schema::<Vec<crate::profile::ForeshadowingSummary>>()
}

pub(crate) fn result_list_profile_materials() -> Value {
    typed_schema::<Vec<crate::profile::ProfileMaterialGroup>>()
}

pub(crate) fn result_list_scene_profiles() -> Value {
    typed_schema::<Vec<crate::profile::SceneProfileSummary>>()
}

pub(crate) fn result_list_llm_config() -> Value {
    typed_schema::<Vec<crate::config::LlmConfigSummary>>()
}

pub(crate) fn result_list_local_embedding_models() -> Value {
    typed_schema::<Vec<crate::embedding::LocalEmbeddingModelSummary>>()
}

pub(crate) fn result_list_manga_page_prompts() -> Value {
    typed_schema::<Vec<crate::manga::MangaPagePromptSummary>>()
}

pub(crate) fn result_list_model_provider_catalog() -> Value {
    typed_schema::<crate::provider_catalog::ModelProviderCatalog>()
}

pub(crate) fn result_list_my_project_invites() -> Value {
    typed_schema::<Vec<crate::collaboration::ProjectInviteSummary>>()
}

pub(crate) fn result_list_pending_tasks() -> Value {
    typed_schema::<Vec<crate::admin::TaskQueueSummary>>()
}

pub(crate) fn result_list_portraits() -> Value {
    typed_schema::<Vec<crate::portrait::PortraitSummary>>()
}

pub(crate) fn result_list_profiles() -> Value {
    typed_schema::<Vec<crate::character::CharacterProfileSummary>>()
}

pub(crate) fn result_list_snapshots() -> Value {
    typed_schema::<Vec<crate::character::SnapshotReferenceSummary>>()
}

pub(crate) fn result_list_project_members() -> Value {
    typed_schema::<Vec<crate::collaboration::ProjectMemberSummary>>()
}

pub(crate) fn result_project_summaries() -> Value {
    typed_schema::<Vec<crate::project::ProjectSummary>>()
}

pub(crate) fn result_list_projects() -> Value {
    typed_schema::<crate::project::PaginatedProjects>()
}

pub(crate) fn result_list_sessions() -> Value {
    typed_schema::<Vec<crate::auth::SessionSummary>>()
}

pub(crate) fn result_list_themes() -> Value {
    typed_schema::<Vec<crate::theme::ThemeSummary>>()
}

pub(crate) fn result_list_video_configs() -> Value {
    typed_schema::<Vec<crate::config::VideoConfigSummary>>()
}

pub(crate) fn result_list_videos_by_chapter() -> Value {
    typed_schema::<Vec<crate::video::GeneratedVideoSummary>>()
}

pub(crate) fn result_login_user() -> Value {
    typed_schema::<crate::auth::AuthResult>()
}

pub(crate) fn result_optimize_chapter_video_prompt() -> Value {
    typed_schema::<crate::video::ChapterVideoPromptSummary>()
}

pub(crate) fn result_optimization_patch_sets() -> Value {
    typed_schema::<Vec<crate::optimize::OptimizationPatchSetSummary>>()
}

pub(crate) fn result_optimization_session() -> Value {
    typed_schema::<crate::optimize::OptimizationSessionSummary>()
}

pub(crate) fn result_optimization_sessions() -> Value {
    typed_schema::<Vec<crate::optimize::OptimizationSessionSummary>>()
}

pub(crate) fn result_optimization_turns() -> Value {
    typed_schema::<Vec<crate::optimize::OptimizationTurnSummary>>()
}

pub(crate) fn result_plan_galgame_assets() -> Value {
    typed_schema::<crate::galgame::GalgameAssetPlan>()
}

pub(crate) fn result_model_request_preview() -> Value {
    typed_schema::<crate::request_preview::ModelRequestPreview>()
}

pub(crate) fn result_optional_assembled_chapter_video_preview() -> Value {
    typed_schema::<Option<crate::video::AssembledChapterVideoPreview>>()
}

pub(crate) fn result_outline_generation_progress() -> Value {
    typed_schema::<crate::outline::OutlineGenerationProgressSummary>()
}

pub(crate) fn result_outline_mutation() -> Value {
    typed_schema::<crate::outline::OutlineMutationResult>()
}

pub(crate) fn result_part_outlines() -> Value {
    typed_schema::<Vec<crate::outline::PartOutlineSummary>>()
}

pub(crate) fn result_object_profile_image_prompt() -> Value {
    typed_schema::<crate::image_generation::ObjectProfileImagePromptOutput>()
}

pub(crate) fn result_scene_profile_image_prompt() -> Value {
    typed_schema::<crate::image_generation::SceneProfileImagePromptOutput>()
}

pub(crate) fn result_preview_rollback() -> Value {
    typed_schema::<crate::project::ProjectRollbackPreview>()
}

pub(crate) fn result_prompt_validate_variables() -> Value {
    typed_schema::<Vec<String>>()
}

pub(crate) fn result_prompt_list() -> Value {
    typed_schema::<Vec<crate::prompt::PromptEntrySummary>>()
}

pub(crate) fn result_prompt_list_categories() -> Value {
    typed_schema::<Vec<crate::prompt::CategoryInfo>>()
}

pub(crate) fn result_prompt_list_project_overrides() -> Value {
    typed_schema::<Vec<crate::prompt::ProjectPromptOverrideSummary>>()
}

pub(crate) fn result_prompt_load_defaults() -> Value {
    typed_schema::<usize>()
}

pub(crate) fn result_prompt_render_ab() -> Value {
    typed_schema::<crate::prompt::AbRenderResult>()
}

pub(crate) fn result_prompt_update() -> Value {
    typed_schema::<crate::prompt::PromptEntrySummary>()
}

pub(crate) fn result_prompt_version_history() -> Value {
    typed_schema::<Vec<crate::prompt::PromptVersionSummary>>()
}

pub(crate) fn result_read_recent_logs() -> Value {
    typed_schema::<crate::logging::LogReadResult>()
}

pub(crate) fn result_profile_material_group() -> Value {
    typed_schema::<crate::profile::ProfileMaterialGroup>()
}

pub(crate) fn result_portrait_summary() -> Value {
    typed_schema::<crate::portrait::PortraitSummary>()
}

pub(crate) fn result_request_attribute_deletion() -> Value {
    typed_schema::<crate::character::DeletionMarkSummary>()
}

pub(crate) fn result_reset_deletion_marks() -> Value {
    typed_schema::<u64>()
}

pub(crate) fn result_refresh_token() -> Value {
    typed_schema::<crate::auth::TokenPairSummary>()
}

pub(crate) fn result_reorder_galgame_lines() -> Value {
    typed_schema::<Vec<crate::galgame::GalgameLine>>()
}

pub(crate) fn result_blueprint_mutation() -> Value {
    typed_schema::<crate::blueprint::BlueprintMutationResult>()
}

pub(crate) fn result_rollback_to_chapter() -> Value {
    typed_schema::<crate::character::RollbackResultSummary>()
}

pub(crate) fn result_run_canvas_graph() -> Value {
    typed_schema::<crate::canvas::CanvasRunSummary>()
}

pub(crate) fn result_run_galgame_asset_generation() -> Value {
    typed_schema::<crate::galgame::GalgameAssetGenerationSummary>()
}

pub(crate) fn result_save_canvas_text_asset() -> Value {
    typed_schema::<crate::canvas::CanvasTextAssetSummary>()
}

pub(crate) fn result_save_chapter_outline() -> Value {
    typed_schema::<crate::outline::SaveChapterOutlineResult>()
}

pub(crate) fn result_save_part_outline() -> Value {
    typed_schema::<crate::outline::SavePartOutlineResult>()
}

pub(crate) fn result_generated_video() -> Value {
    typed_schema::<crate::video::GeneratedVideoSummary>()
}

pub(crate) fn result_video_playback_data() -> Value {
    typed_schema::<crate::video::VideoPlaybackData>()
}

pub(crate) fn result_generation_task_handle() -> Value {
    typed_schema::<crate::generation_tasks::GenerationTaskHandle>()
}

pub(crate) fn result_sync_all_character_profiles() -> Value {
    typed_schema::<crate::character::SyncAllCharacterProfilesResult>()
}

pub(crate) fn result_sync_character_from_chapters() -> Value {
    typed_schema::<crate::character::SyncCharacterProfileResult>()
}

pub(crate) fn result_update_chapter_outline() -> Value {
    typed_schema::<crate::outline::UpdateChapterOutlineResult>()
}

pub(crate) fn result_unbind_galgame_media() -> Value {
    typed_schema::<u64>()
}

pub(crate) fn result_update_character_profile() -> Value {
    typed_schema::<crate::character::CharacterProfileMutationResult>()
}

pub(crate) fn result_update_galgame_choices() -> Value {
    typed_schema::<Vec<crate::galgame::GalgameChoice>>()
}

pub(crate) fn result_update_galgame_scene() -> Value {
    typed_schema::<crate::galgame::GalgameScene>()
}

pub(crate) fn result_update_image() -> Value {
    typed_schema::<crate::image::GeneratedImageSummary>()
}

pub(crate) fn result_workflow_run() -> Value {
    typed_schema::<crate::workflow::WorkflowRunSummary>()
}
