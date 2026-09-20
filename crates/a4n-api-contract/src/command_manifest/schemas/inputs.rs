use super::builders::*;
use serde_json::{Value, json};
pub(crate) fn args_character_profile_value() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "name": { "type": "string", "minLength": 1 },
        },
        "required": ["projectId", "name"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_check_character_conflict() -> Value {
    required_task_runtime_typed_input_schema::<crate::character::CharacterConflictCheckInput>()
}

pub(crate) fn args_preview_check_character_conflict() -> Value {
    strict_runtime_typed_input_schema::<crate::character::CharacterConflictCheckInput>()
}

pub(crate) fn args_discover_character_profiles() -> Value {
    required_task_runtime_typed_input_schema::<crate::character::DiscoverCharacterProfilesInput>()
}

pub(crate) fn args_preview_discover_character_profiles() -> Value {
    strict_runtime_typed_input_schema::<crate::character::DiscoverCharacterProfilesInput>()
}

pub(crate) fn args_config_id() -> Value {
    json!({
        "type": "object",
        "properties": { "configId": { "type": "integer", "minimum": 1 } },
        "required": ["configId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_activate_theme() -> Value {
    json!({
        "type": "object",
        "properties": { "themeId": { "type": "integer", "minimum": 1 } },
        "required": ["themeId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_delete_theme_resource() -> Value {
    strict_typed_input_schema::<crate::theme::DeleteThemeResourceInput>()
}

pub(crate) fn args_theme_resource_upload() -> Value {
    strict_typed_input_schema::<crate::theme::ThemeResourceUploadInput>()
}

pub(crate) fn args_theme_package_upload() -> Value {
    strict_typed_input_schema::<crate::theme::ThemePackageUploadInput>()
}

pub(crate) fn args_update_theme_preference() -> Value {
    strict_typed_input_schema::<crate::theme::UpdateThemePreferenceInput>()
}

pub(crate) fn args_accept_import_review() -> Value {
    strict_typed_input_schema::<crate::import_review::AcceptImportReviewInput>()
}

pub(crate) fn args_add_profile_attribute() -> Value {
    strict_typed_input_schema::<crate::character::AddProfileAttributeInput>()
}

pub(crate) fn args_add_image_config() -> Value {
    strict_typed_input_schema::<crate::config::ImageConfigInput>()
}

pub(crate) fn args_add_video_config() -> Value {
    strict_typed_input_schema::<crate::config::VideoConfigInput>()
}

pub(crate) fn args_apply_assembly_postprocess_edit() -> Value {
    json!({
        "type": "object",
        "properties": {
            "pushVideoId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::video::ApplyAssemblyPostprocessEditInput>(),
        },
        "required": ["pushVideoId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn args_apply_optimization_patches() -> Value {
    strict_runtime_typed_input_schema::<crate::optimize::ApplyOptimizationPatchesInput>()
}

pub(crate) fn args_discard_optimization_patch_set() -> Value {
    strict_runtime_typed_input_schema::<crate::optimize::DiscardOptimizationPatchSetInput>()
}

pub(crate) fn args_assemble_chapter_videos() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "workflowMode": { "type": "string", "minLength": 1 },
        },
        "required": ["projectId", "chapterId", "workflowMode"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_apply_implicit_update_decision() -> Value {
    strict_typed_input_schema::<crate::character::ApplyImplicitUpdateDecisionInput>()
}

pub(crate) fn args_attribute_history() -> Value {
    strict_typed_input_schema::<crate::character::AttributeHistoryInput>()
}

pub(crate) fn args_bind_galgame_media() -> Value {
    strict_typed_input_schema::<crate::galgame::BindGalgameMediaInput>()
}

pub(crate) fn args_bind_manga_reference_image() -> Value {
    strict_typed_input_schema::<crate::manga::BindMangaReferenceImageInput>()
}

pub(crate) fn args_batch_accept_reviews() -> Value {
    strict_typed_input_schema::<crate::import_review::BatchAcceptImportReviewsInput>()
}

pub(crate) fn args_cancel_embedding_model_download() -> Value {
    typed_input_schema::<crate::embedding::CancelEmbeddingModelDownloadInput>()
}

pub(crate) fn args_cancel_generation_task() -> Value {
    strict_typed_input_schema::<crate::generation_tasks::CancelGenerationTaskInput>()
}

pub(crate) fn args_cancel_galgame_pipeline() -> Value {
    strict_typed_input_schema::<crate::galgame::GalgameProjectInput>()
}

pub(crate) fn args_cancel_manga_pipeline() -> Value {
    strict_typed_input_schema::<crate::manga::MangaChapterInput>()
}

pub(crate) fn args_list_manga_pipeline_statuses() -> Value {
    strict_typed_input_schema::<crate::manga::MangaPipelineStatusesInput>()
}

pub(crate) fn args_cancel_task() -> Value {
    typed_schema::<crate::task::CancelTaskInput>()
}

pub(crate) fn args_cancel_workflow() -> Value {
    strict_typed_input_schema::<crate::workflow::CancelWorkflowInput>()
}

pub(crate) fn args_change_user_password() -> Value {
    typed_input_schema::<crate::auth::ChangePasswordInput>()
}

pub(crate) fn args_apply_source_sync() -> Value {
    strict_runtime_typed_input_schema::<crate::source_sync::ApplySourceSyncInput>()
}

pub(crate) fn args_confirm_reverse_engineering_candidate() -> Value {
    strict_runtime_typed_input_schema::<
        crate::import_reverse::ConfirmReverseEngineeringCandidateInput,
    >()
}

pub(crate) fn args_check_import_dedup() -> Value {
    strict_typed_input_schema::<crate::import_review::CheckImportDedupInput>()
}

pub(crate) fn args_check_implicit_update() -> Value {
    strict_runtime_typed_input_schema::<crate::character::CheckImplicitUpdateInput>()
}

pub(crate) fn args_check_profile_sync_conflict() -> Value {
    strict_typed_input_schema::<crate::character::CheckProfileSyncConflictInput>()
}

pub(crate) fn args_chapter_analysis() -> Value {
    strict_runtime_typed_input_schema::<crate::chapter::ChapterAnalysisInput>()
}

pub(crate) fn args_chapter_analysis_task() -> Value {
    optional_task_runtime_typed_input_schema::<crate::chapter::ChapterAnalysisInput>()
}

pub(crate) fn args_create_derived_project() -> Value {
    strict_typed_input_schema::<crate::project::CreateDerivedProjectInput>()
}

pub(crate) fn args_source_sync_project() -> Value {
    strict_typed_input_schema::<crate::source_sync::SourceSyncProjectInput>()
}

pub(crate) fn args_create_embedding_config() -> Value {
    strict_typed_input_schema::<crate::config::CreateEmbeddingInput>()
}

pub(crate) fn args_create_freeform_video_project() -> Value {
    strict_typed_input_schema::<crate::project::CreateFreeformVideoProjectInput>()
}

pub(crate) fn args_create_llm_config() -> Value {
    strict_typed_input_schema::<crate::config::CreateLlmInput>()
}

pub(crate) fn args_create_optimization_analysis_session() -> Value {
    strict_runtime_typed_input_schema::<crate::optimize::AnalyzeChapterInput>()
}

pub(crate) fn args_create_optimization_chat_session() -> Value {
    strict_runtime_typed_input_schema::<crate::optimize::CreateOptimizationChatSessionInput>()
}

pub(crate) fn args_create_profile() -> Value {
    strict_runtime_typed_input_schema::<crate::character::CreateProfileInput>()
}

pub(crate) fn args_create_object_profile() -> Value {
    strict_typed_input_schema::<crate::profile::CreateObjectProfileInput>()
}

pub(crate) fn args_create_foreshadowing() -> Value {
    strict_runtime_typed_input_schema::<crate::profile::CreateForeshadowingInput>()
}

pub(crate) fn args_create_scene_profile() -> Value {
    strict_runtime_typed_input_schema::<crate::profile::CreateSceneProfileInput>()
}

pub(crate) fn args_create_project() -> Value {
    strict_typed_input_schema::<crate::project::CreateProjectInput>()
}

pub(crate) fn args_create_theme() -> Value {
    strict_typed_input_schema::<crate::theme::CreateThemeInput>()
}

pub(crate) fn args_create_invite_code() -> Value {
    strict_typed_input_schema::<crate::admin::CreateInviteCodeInput>()
}

pub(crate) fn args_create_user() -> Value {
    strict_typed_input_schema::<crate::admin::CreateUserInput>()
}

pub(crate) fn args_commit_inserted_segment_draft() -> Value {
    typed_input_schema::<crate::video::CommitInsertedSegmentDraftInput>()
}

pub(crate) fn args_delete_chapter_video_prompt() -> Value {
    typed_input_schema::<crate::video::DeleteChapterVideoPromptInput>()
}

pub(crate) fn args_delete_inserted_segment() -> Value {
    typed_input_schema::<crate::video::DeleteInsertedSegmentInput>()
}

pub(crate) fn args_delete_chapter_scene() -> Value {
    json!({
        "type": "object",
        "properties": { "sceneId": { "type": "integer", "minimum": 1 } },
        "required": ["sceneId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_delete_chapter_shooting_script() -> Value {
    strict_typed_input_schema::<crate::shooting_script::DeleteChapterShootingScriptInput>()
}

pub(crate) fn args_delete_galgame_line() -> Value {
    project_typed_input_schema::<crate::galgame::DeleteGalgameLineInput>()
}

pub(crate) fn args_delete_image() -> Value {
    strict_typed_input_schema::<crate::image::DeleteImageInput>()
}

pub(crate) fn args_delete_object_profile() -> Value {
    strict_typed_input_schema::<crate::profile::DeleteObjectProfileInput>()
}

pub(crate) fn args_delete_foreshadowing() -> Value {
    strict_typed_input_schema::<crate::profile::DeleteForeshadowingInput>()
}

pub(crate) fn args_delete_scene_profile() -> Value {
    strict_runtime_typed_input_schema::<crate::profile::DeleteSceneProfileInput>()
}

pub(crate) fn args_delete_latest_outline_records() -> Value {
    strict_typed_input_schema::<crate::outline::DeleteLatestOutlineRecordsInput>()
}

pub(crate) fn args_apply_beat_mapping() -> Value {
    strict_typed_input_schema::<crate::outline::ApplyBeatMappingInput>()
}

pub(crate) fn args_batch_generate_part_outlines() -> Value {
    strict_typed_input_schema::<crate::outline::BatchGeneratePartOutlinesInput>()
}

pub(crate) fn args_continue_part_outlines() -> Value {
    strict_typed_input_schema::<crate::outline::ContinuePartOutlinesInput>()
}

pub(crate) fn args_generate_chapter_outlines_by_count() -> Value {
    strict_typed_input_schema::<crate::outline::GenerateChapterOutlinesByCountInput>()
}

pub(crate) fn args_generate_part_chapter_outlines() -> Value {
    strict_typed_input_schema::<crate::outline::GeneratePartChapterOutlinesInput>()
}

pub(crate) fn args_generate_parts() -> Value {
    strict_typed_input_schema::<crate::outline::GeneratePartsInput>()
}

pub(crate) fn args_regenerate_chapter_outline() -> Value {
    strict_typed_input_schema::<crate::outline::RegenerateChapterOutlineInput>()
}

pub(crate) fn args_regenerate_last_part() -> Value {
    strict_typed_input_schema::<crate::outline::RegenerateLastPartInput>()
}

pub(crate) fn args_regenerate_part_outline() -> Value {
    strict_typed_input_schema::<crate::outline::RegeneratePartOutlineInput>()
}

pub(crate) fn args_regenerate_parts() -> Value {
    strict_typed_input_schema::<crate::outline::RegeneratePartsInput>()
}

pub(crate) fn args_update_chapter_outline() -> Value {
    strict_typed_input_schema::<crate::outline::UpdateChapterOutlineInput>()
}

pub(crate) fn args_delete_local_embedding_model() -> Value {
    typed_input_schema::<crate::embedding::EmbeddingModelInput>()
}

pub(crate) fn args_delete_manga_prompt() -> Value {
    strict_typed_input_schema::<crate::manga::DeleteMangaPromptInput>()
}

pub(crate) fn args_delete_profile() -> Value {
    json!({
        "type": "object",
        "properties": { "profileId": { "type": "integer", "minimum": 1 } },
        "required": ["profileId"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_project_id() -> Value {
    json!({
        "type": "object",
        "properties": { "projectId": { "type": "integer", "minimum": 1 } },
        "required": ["projectId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_list_projects() -> Value {
    json!({
        "type": "object",
        "properties": {
            "page": { "type": "integer", "minimum": 1 },
            "pageSize": { "type": "integer", "minimum": 1, "maximum": 100 },
            "projectKind": { "type": ["string", "null"], "minLength": 1 },
            "rootProjectId": { "type": ["integer", "null"], "minimum": 1 },
            "accessScope": { "type": ["string", "null"], "minLength": 1 },
        },
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_load_video_segment_plan_draft() -> Value {
    strict_typed_input_schema::<crate::video::LoadVideoSegmentPlanDraftInput>()
}

pub(crate) fn args_oauth2_authorize_url() -> Value {
    strict_typed_input_schema::<crate::auth::OAuth2AuthorizeInput>()
}

pub(crate) fn args_oauth2_callback() -> Value {
    strict_typed_input_schema::<crate::auth::OAuth2CallbackInput>()
}

pub(crate) fn args_preview_chapter_prompt() -> Value {
    strict_runtime_typed_input_schema::<crate::chapter::PreviewChapterPromptInput>()
}

pub(crate) fn args_prepare_chapter_generation_rag_embedding() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterNumber": { "type": "integer", "minimum": 1 },
            "writingNotes": { "type": ["string", "null"] },
            "minWordCount": { "type": ["integer", "null"] },
            "maxWordCount": { "type": ["integer", "null"] },
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["projectId", "chapterNumber"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_rag_enhanced_search() -> Value {
    project_runtime_typed_input_schema::<crate::rag::EnhancedSearchInput>()
}

pub(crate) fn args_rag_enhanced_search_with_vectors() -> Value {
    project_runtime_typed_input_schema::<crate::rag::RagEnhancedSearchWithVectorsInput>()
}

pub(crate) fn args_rag_ingest_all() -> Value {
    project_optional_task_runtime_typed_input_schema::<crate::rag::RagIngestOptions>()
}

pub(crate) fn args_rag_prepare_ingest_all() -> Value {
    project_runtime_typed_input_schema::<crate::rag::RagIngestOptions>()
}

pub(crate) fn args_rag_upload_ingest_embeddings() -> Value {
    project_runtime_typed_input_schema::<crate::rag::ClientRagIngestUploadInput>()
}

pub(crate) fn args_video_id() -> Value {
    json!({
        "type": "object",
        "properties": { "videoId": { "type": "integer", "minimum": 1 } },
        "required": ["videoId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_poll_video_job() -> Value {
    json!({
        "type": "object",
        "properties": {
            "videoId": { "type": "integer", "minimum": 1 },
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["videoId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_duplicate_project() -> Value {
    typed_input_schema::<crate::project::DuplicateProjectInput>()
}

pub(crate) fn args_detect_image_config_models() -> Value {
    strict_typed_input_schema::<crate::config::ModelDiscoveryInput>()
}

pub(crate) fn args_diff_blueprint_versions() -> Value {
    strict_typed_input_schema::<crate::blueprint::DiffBlueprintVersionsInput>()
}

pub(crate) fn args_diff_profile_chapters() -> Value {
    strict_typed_input_schema::<crate::character::DiffProfileChaptersInput>()
}

pub(crate) fn args_download_embedding_model() -> Value {
    typed_input_schema::<crate::embedding::DownloadEmbeddingModelInput>()
}

pub(crate) fn args_execute_attribute_deletion() -> Value {
    strict_typed_input_schema::<crate::character::ExecuteAttributeDeletionInput>()
}

pub(crate) fn args_edit_chapter_shooting_script() -> Value {
    strict_typed_input_schema::<crate::shooting_script::EditChapterShootingScriptInput>()
}

pub(crate) fn args_chapter_visual_plan_source() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::ChapterVisualPlanSourceInput>()
}

pub(crate) fn args_save_chapter_visual_plan_draft() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::SaveChapterVisualPlanDraftInput>()
}

pub(crate) fn args_chapter_visual_plan_record() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::ChapterVisualPlanRecordInput>()
}

pub(crate) fn args_read_chapter_visual_plan() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::ReadChapterVisualPlanInput>()
}

pub(crate) fn args_read_visual_plan_prompt_candidate() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::ReadVisualPlanPromptCandidateInput>()
}

pub(crate) fn result_visual_plan_prompt_candidate() -> Value {
    typed_schema::<crate::chapter_visual_plan::VisualPlanPromptCandidate>()
}

pub(crate) fn args_preview_visual_plan_point_generation() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::PreviewVisualPlanPointInput>()
}

pub(crate) fn result_visual_plan_point_generation_preview() -> Value {
    typed_schema::<crate::chapter_visual_plan::VisualPlanPointGenerationPreview>()
}

pub(crate) fn args_start_visual_plan_prompt_generation() -> Value {
    strict_runtime_typed_input_schema::<
        crate::chapter_visual_plan::StartVisualPlanPromptGenerationInput,
    >()
}

pub(crate) fn args_respond_agent_editor() -> Value {
    strict_typed_input_schema::<crate::agent::RespondAgentEditorInput>()
}

pub(crate) fn result_agent_editor_reply() -> Value {
    typed_schema::<crate::agent::AgentEditorReply>()
}

pub(crate) fn args_review_agent_delegation() -> Value {
    strict_typed_input_schema::<crate::agent::ReviewAgentDelegationInput>()
}

pub(crate) fn args_update_agent_skill_configuration() -> Value {
    strict_typed_input_schema::<crate::agent::UpdateAgentSkillConfigurationInput>()
}

pub(crate) fn result_agent_delegation() -> Value {
    typed_schema::<crate::agent::AgentDelegationView>()
}

pub(crate) fn result_agent_skill_profile() -> Value {
    typed_schema::<crate::agent::AgentSkillProfileView>()
}

pub(crate) fn result_agent_skill_settings() -> Value {
    typed_schema::<crate::agent::AgentSkillSettingsView>()
}

pub(crate) fn args_edit_chapter_visual_plan_draft() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::EditChapterVisualPlanDraftInput>()
}

pub(crate) fn args_validate_chapter_visual_plan() -> Value {
    strict_typed_input_schema::<crate::chapter_visual_plan::ValidateChapterVisualPlanInput>()
}

pub(crate) fn args_export_batch_data() -> Value {
    required_task_typed_input_schema::<crate::export::BatchExportInput>()
}

pub(crate) fn args_export_all_configs_json() -> Value {
    empty_args_schema()
}

pub(crate) fn args_export_characters_json_data() -> Value {
    required_task_typed_input_schema::<crate::export::ExportProjectInput>()
}

pub(crate) fn args_export_derived_project_data() -> Value {
    strict_typed_input_schema::<crate::project_transfer::ExportDerivedProjectDataInput>()
}

pub(crate) fn args_export_epub_data() -> Value {
    required_task_typed_input_schema::<crate::export::ExportPublicationInput>()
}

pub(crate) fn args_export_project_family_data() -> Value {
    strict_typed_input_schema::<crate::project_transfer::ExportProjectFamilyDataInput>()
}

pub(crate) fn args_export_project_bundle_data() -> Value {
    strict_typed_input_schema::<crate::project_transfer::ExportProjectBundleDataInput>()
}

pub(crate) fn args_export_video_studio_project_data() -> Value {
    strict_typed_input_schema::<crate::project_transfer::ExportVideoStudioProjectDataInput>()
}

pub(crate) fn args_forward_project_status() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "target": raw_schema::<crate::project::ProjectStatus>(),
        },
        "required": ["projectId", "target"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_project_asset_dependency_graph_node_detail() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "nodeId": { "type": "string", "minLength": 1 },
        },
        "required": ["projectId", "nodeId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_generate_chapter_shooting_script() -> Value {
    optional_task_runtime_typed_input_schema::<
        crate::shooting_script::GenerateChapterShootingScriptInput,
    >()
}

pub(crate) fn args_generate_chapter_scene() -> Value {
    optional_task_runtime_typed_input_schema::<crate::chapter::GenerateChapterSceneInput>()
}

pub(crate) fn args_generate_object_profile_image_prompt() -> Value {
    strict_typed_input_schema::<crate::image_generation::ObjectProfileImageInput>()
}

pub(crate) fn args_generate_portrait_prompt() -> Value {
    strict_runtime_typed_input_schema::<crate::portrait::GeneratePortraitPromptInput>()
}

pub(crate) fn args_generate_scene_profile_image_prompt() -> Value {
    strict_typed_input_schema::<crate::image_generation::SceneProfileImageInput>()
}

pub(crate) fn args_get_canvas_graph() -> Value {
    strict_typed_input_schema::<crate::canvas::GetCanvasGraphInput>()
}

pub(crate) fn args_get_canvas_node_contracts() -> Value {
    strict_typed_input_schema::<crate::canvas::GetCanvasNodeContractsInput>()
}

pub(crate) fn args_get_chapter_shooting_script() -> Value {
    strict_typed_input_schema::<crate::shooting_script::ChapterShootingScriptSourceInput>()
}

pub(crate) fn args_get_generation_call_analytics() -> Value {
    strict_typed_input_schema::<crate::admin::GenerationCallAnalyticsInput>()
}

pub(crate) fn args_get_model_usage_report() -> Value {
    strict_typed_input_schema::<crate::cost::ModelUsageReportInput>()
}

pub(crate) fn args_get_implicit_review() -> Value {
    strict_typed_input_schema::<crate::character::GetImplicitReviewInput>()
}

pub(crate) fn args_get_inspiration_history() -> Value {
    strict_typed_input_schema::<crate::inspiration::InspirationHistoryInput>()
}

pub(crate) fn args_get_primary_portrait() -> Value {
    strict_typed_input_schema::<crate::portrait::GetPrimaryPortraitInput>()
}

pub(crate) fn args_get_profile_state_at_chapter() -> Value {
    strict_typed_input_schema::<crate::character::GetProfileStateAtChapterInput>()
}

pub(crate) fn args_get_snapshot() -> Value {
    strict_typed_input_schema::<crate::character::GetSnapshotInput>()
}

pub(crate) fn args_get_package_operation() -> Value {
    strict_typed_input_schema::<crate::package::GetPackageOperationInput>()
}

pub(crate) fn args_get_task_history() -> Value {
    typed_schema::<crate::task::GetTaskHistoryInput>()
}

pub(crate) fn args_user_id() -> Value {
    json!({
        "type": "object",
        "properties": { "userId": { "type": "integer", "minimum": 1 } },
        "required": ["userId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_task_id() -> Value {
    json!({
        "type": "object",
        "properties": { "taskId": { "type": "string", "minLength": 1 } },
        "required": ["taskId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_user_usage() -> Value {
    json!({
        "type": "object",
        "properties": {
            "userId": { "type": "integer", "minimum": 1 },
            "limit": { "type": "integer", "minimum": 1 },
        },
        "required": ["userId", "limit"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_get_usage_trends() -> Value {
    strict_typed_input_schema::<crate::admin::UsageTrendsInput>()
}

pub(crate) fn args_get_workflow_progress_events() -> Value {
    strict_typed_input_schema::<crate::workflow::WorkflowProgressEventsInput>()
}

pub(crate) fn args_get_workflow_run_detail() -> Value {
    strict_typed_input_schema::<crate::workflow::GetWorkflowRunInput>()
}

pub(crate) fn args_get_reverse_engineering_candidate() -> Value {
    strict_typed_input_schema::<crate::import_reverse::GetReverseEngineeringCandidateInput>()
}

pub(crate) fn args_import_canvas_media() -> Value {
    strict_typed_input_schema::<crate::canvas::ImportCanvasMediaInput>()
}

pub(crate) fn args_import_chapter() -> Value {
    strict_typed_input_schema::<crate::chapter::ImportChapterInput>()
}

pub(crate) fn args_import_all_configs() -> Value {
    typed_input_schema::<crate::config::ImportAllConfigsInput>()
}

pub(crate) fn args_import_galgame_audio() -> Value {
    strict_typed_input_schema::<crate::galgame::ImportGalgameAudioInput>()
}

pub(crate) fn args_import_project_family_upload() -> Value {
    strict_typed_input_schema::<crate::project_transfer::ImportProjectPackageUploadInput>()
}

pub(crate) fn args_inspect_project_bundle_upload() -> Value {
    strict_typed_input_schema::<crate::project_transfer::InspectProjectBundleUploadInput>()
}

pub(crate) fn args_import_project_bundle_upload() -> Value {
    strict_typed_input_schema::<crate::project_transfer::ImportProjectPackageUploadInput>()
}

pub(crate) fn args_import_project_reviews() -> Value {
    strict_typed_input_schema::<crate::import_review::ImportProjectReviewsInput>()
}

pub(crate) fn args_import_project_text_upload() -> Value {
    strict_typed_input_schema::<crate::import::ImportProjectTextUploadInput>()
}

pub(crate) fn args_import_review_id() -> Value {
    strict_typed_input_schema::<crate::import_review::ImportReviewIdInput>()
}

pub(crate) fn args_import_video_studio_project_upload() -> Value {
    strict_typed_input_schema::<crate::project_transfer::ImportProjectPackageUploadInput>()
}

pub(crate) fn args_insert_galgame_line() -> Value {
    project_typed_input_schema::<crate::galgame::InsertGalgameLineInput>()
}

pub(crate) fn args_invite_project_member() -> Value {
    typed_input_schema::<crate::collaboration::InviteProjectMemberInput>()
}

pub(crate) fn args_invite_id() -> Value {
    json!({
        "type": "object",
        "properties": { "inviteId": { "type": "integer", "minimum": 1 } },
        "required": ["inviteId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_list_blueprint_versions() -> Value {
    strict_typed_input_schema::<crate::blueprint::ListBlueprintVersionsInput>()
}

pub(crate) fn args_list_behaviors() -> Value {
    strict_typed_input_schema::<crate::character::ListBehaviorsInput>()
}

pub(crate) fn args_list_chapter_scenes() -> Value {
    json!({
        "type": "object",
        "properties": { "chapterId": { "type": "integer", "minimum": 1 } },
        "required": ["chapterId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_list_chapter_video_prompts() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
        },
        "required": ["projectId", "chapterId"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_list_videos_by_chapter() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
        },
        "required": ["projectId", "chapterId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_load_video_scrub_playback_data() -> Value {
    json!({
        "type": "object",
        "properties": {
            "videoId": { "type": "integer", "minimum": 1 },
            "includeAudio": { "type": "boolean" },
        },
        "required": ["videoId", "includeAudio"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_get_video_timeline() -> Value {
    typed_input_schema::<crate::video::GetVideoTimelineInput>()
}

pub(crate) fn args_move_inserted_segment() -> Value {
    typed_input_schema::<crate::video::MoveInsertedSegmentInput>()
}

pub(crate) fn args_list_deletion_marks() -> Value {
    strict_typed_input_schema::<crate::character::ListDeletionMarksInput>()
}

pub(crate) fn args_list_images_by_chapter() -> Value {
    strict_typed_input_schema::<crate::image::ListImagesByChapterInput>()
}

pub(crate) fn args_list_images_by_project() -> Value {
    strict_typed_input_schema::<crate::image::ListImagesByProjectInput>()
}

pub(crate) fn args_list_images_by_projects() -> Value {
    strict_typed_input_schema::<crate::image::ListImagesByProjectsInput>()
}

pub(crate) fn args_list_optimize_sessions() -> Value {
    strict_typed_input_schema::<crate::optimize::ListOptimizeSessionsInput>()
}

pub(crate) fn args_list_object_profiles() -> Value {
    strict_typed_input_schema::<crate::profile::ListObjectProfilesInput>()
}

pub(crate) fn args_list_package_command_availability() -> Value {
    strict_typed_input_schema::<crate::package::ListPackageCommandAvailabilityInput>()
}

pub(crate) fn args_list_foreshadowing() -> Value {
    strict_typed_input_schema::<crate::profile::ListForeshadowingInput>()
}

pub(crate) fn args_list_profile_materials() -> Value {
    strict_typed_input_schema::<crate::profile::ListProfileMaterialsInput>()
}

pub(crate) fn args_list_scene_profiles() -> Value {
    strict_typed_input_schema::<crate::profile::ListSceneProfilesInput>()
}

pub(crate) fn args_list_profiles() -> Value {
    strict_typed_input_schema::<crate::character::ListProfilesInput>()
}

pub(crate) fn args_list_portraits_by_project() -> Value {
    strict_typed_input_schema::<crate::portrait::ListPortraitsByProjectInput>()
}

pub(crate) fn args_list_snapshots() -> Value {
    strict_typed_input_schema::<crate::character::ListSnapshotsInput>()
}

pub(crate) fn args_list_video_reference_images() -> Value {
    strict_typed_input_schema::<crate::image::ListVideoReferenceImagesInput>()
}

pub(crate) fn args_login_user() -> Value {
    typed_input_schema::<crate::auth::LoginInput>()
}

pub(crate) fn args_modify_profile_attribute() -> Value {
    strict_typed_input_schema::<crate::character::ModifyProfileAttributeInput>()
}

pub(crate) fn args_optimize_chapter_video_prompt() -> Value {
    json!({
        "type": "object",
        "properties": {
            "promptId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::video::OptimizeChapterVideoPromptInput>(),
        },
        "required": ["promptId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn args_optimize_manga_prompt() -> Value {
    strict_typed_input_schema::<crate::manga::OptimizeMangaPromptInput>()
}

pub(crate) fn args_optimize_session() -> Value {
    strict_typed_input_schema::<crate::optimize::OptimizeSessionInput>()
}

pub(crate) fn args_prepare_optimization_analysis_session() -> Value {
    strict_runtime_typed_input_schema::<crate::optimize::PrepareOptimizationAnalysisSessionInput>()
}

pub(crate) fn args_propose_optimization_patches() -> Value {
    strict_runtime_typed_input_schema::<crate::optimize::ProposeOptimizationPatchesInput>()
}

pub(crate) fn args_preview_canvas_node_request() -> Value {
    strict_typed_input_schema::<crate::canvas::PreviewCanvasNodeRequestInput>()
}

pub(crate) fn args_preview_chapter_video_request() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "promptId": { "type": "integer", "minimum": 1 },
            "configId": { "type": ["integer", "null"], "minimum": 1 },
            "promptOverride": raw_schema::<crate::video::PreviewChapterVideoPromptOverrideInput>(),
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["projectId", "chapterId", "promptId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn args_preview_reference_media_video_request() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::video::ReferenceMediaVideoInput>(),
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["projectId", "chapterId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn args_preview_video_provider_request_by_video_id() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "videoId": { "type": "integer", "minimum": 1 },
        },
        "required": ["projectId", "chapterId", "videoId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_preview_chapter_scene_request() -> Value {
    strict_runtime_typed_input_schema::<crate::chapter::GenerateChapterSceneInput>()
}

pub(crate) fn args_preview_galgame_pipeline_request() -> Value {
    strict_typed_input_schema::<crate::galgame::PreviewGalgamePipelineInput>()
}

pub(crate) fn args_preview_inspiration_request() -> Value {
    strict_runtime_typed_input_schema::<crate::inspiration::PreviewInspirationRequestInput>()
}

pub(crate) fn args_preview_object_profile_image_request() -> Value {
    strict_runtime_typed_input_schema::<crate::image_generation::ObjectProfileImageInput>()
}

pub(crate) fn args_preview_scene_image_request() -> Value {
    strict_runtime_typed_input_schema::<crate::image_generation::GenerateSceneImageInput>()
}

pub(crate) fn args_preview_page_image_request() -> Value {
    strict_runtime_typed_input_schema::<crate::image_generation::GeneratePageImageInput>()
}

pub(crate) fn args_preview_batch_images_request() -> Value {
    strict_runtime_typed_input_schema::<crate::image_generation::GenerateBatchImagesInput>()
}

pub(crate) fn args_preview_generate_age_variant_request() -> Value {
    strict_runtime_typed_input_schema::<crate::portrait::GenerateAgeVariantInput>()
}

pub(crate) fn args_preview_generate_missing_portraits_request() -> Value {
    strict_runtime_typed_input_schema::<crate::portrait::GenerateMissingPortraitsInput>()
}

pub(crate) fn args_preview_generate_portrait_prompt_request() -> Value {
    strict_runtime_typed_input_schema::<crate::portrait::GeneratePortraitPromptInput>()
}

pub(crate) fn args_preview_generate_portrait_request() -> Value {
    strict_runtime_typed_input_schema::<crate::portrait::GeneratePortraitInput>()
}

pub(crate) fn args_preview_regenerate_portrait_request() -> Value {
    strict_runtime_typed_input_schema::<crate::portrait::RegeneratePortraitInput>()
}

pub(crate) fn args_preview_generate_blueprint_request() -> Value {
    strict_runtime_typed_input_schema::<crate::blueprint::PreviewBlueprintGenerationInput>()
}

pub(crate) fn args_preview_refine_blueprint_request() -> Value {
    strict_runtime_typed_input_schema::<crate::blueprint::PreviewBlueprintRefinementInput>()
}

pub(crate) fn args_preview_reverse_engineer() -> Value {
    strict_runtime_typed_input_schema::<crate::import_reverse::ReverseEngineerInput>()
}

pub(crate) fn args_preview_resume_manga_pipeline_request() -> Value {
    strict_typed_input_schema::<crate::manga::MangaPipelineResumeInput>()
}

pub(crate) fn args_preview_start_manga_pipeline_request() -> Value {
    strict_typed_input_schema::<crate::manga::MangaPipelineStartInput>()
}

pub(crate) fn args_preview_sync_all_character_profiles_request() -> Value {
    typed_input_schema::<crate::character::SyncAllCharacterProfilesInput>()
}

pub(crate) fn args_preview_sync_character_from_chapters_request() -> Value {
    typed_input_schema::<crate::character::SyncCharacterProfileInput>()
}

pub(crate) fn args_preview_test_embedding_config_request() -> Value {
    json!({
        "type": "object",
        "properties": {
            "configId": { "type": "integer", "minimum": 1 },
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["configId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_preview_test_image_config_request() -> Value {
    json!({
        "type": "object",
        "properties": {
            "configId": {
                "type": "integer",
                "anyOf": [{ "const": -1 }, { "minimum": 1 }],
            },
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["configId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_prompt_name() -> Value {
    typed_schema::<crate::prompt::PromptNameArgs>()
}

pub(crate) fn args_prompt_remove_project_override() -> Value {
    strict_typed_input_schema::<crate::prompt::RemoveOverrideInput>()
}

pub(crate) fn args_prompt_render() -> Value {
    strict_typed_input_schema::<crate::prompt::RenderInput>()
}

pub(crate) fn args_prompt_set_project_override() -> Value {
    strict_typed_input_schema::<crate::prompt::SetOverrideInput>()
}

pub(crate) fn args_prompt_update() -> Value {
    strict_typed_input_schema::<crate::prompt::UpdatePromptInput>()
}

pub(crate) fn args_prompt_validate_variables() -> Value {
    strict_typed_input_schema::<crate::prompt::ValidateVarsInput>()
}

pub(crate) fn args_prompt_version_diff() -> Value {
    strict_typed_input_schema::<crate::prompt::VersionDiffInput>()
}

pub(crate) fn args_push_assembled_chapter_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "workflowMode": { "type": "string", "minLength": 1 },
            "input": raw_schema::<crate::video::PushAssembledChapterVideoInput>(),
        },
        "required": ["projectId", "chapterId", "workflowMode", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
