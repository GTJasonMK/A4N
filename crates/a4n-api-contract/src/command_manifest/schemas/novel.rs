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
pub(crate) fn args_accept_import_review() -> Value {
    strict_typed_input_schema::<crate::import_review::AcceptImportReviewInput>()
}
pub(crate) fn args_add_profile_attribute() -> Value {
    strict_typed_input_schema::<crate::character::AddProfileAttributeInput>()
}
pub(crate) fn args_apply_optimization_patches() -> Value {
    strict_runtime_typed_input_schema::<crate::optimize::ApplyOptimizationPatchesInput>()
}
pub(crate) fn args_discard_optimization_patch_set() -> Value {
    strict_runtime_typed_input_schema::<crate::optimize::DiscardOptimizationPatchSetInput>()
}
pub(crate) fn args_apply_implicit_update_decision() -> Value {
    strict_typed_input_schema::<crate::character::ApplyImplicitUpdateDecisionInput>()
}
pub(crate) fn args_attribute_history() -> Value {
    strict_typed_input_schema::<crate::character::AttributeHistoryInput>()
}
pub(crate) fn args_batch_accept_reviews() -> Value {
    strict_typed_input_schema::<crate::import_review::BatchAcceptImportReviewsInput>()
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
pub(crate) fn args_create_freeform_video_project() -> Value {
    strict_typed_input_schema::<crate::project::CreateFreeformVideoProjectInput>()
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
pub(crate) fn args_delete_chapter_scene() -> Value {
    json!({
        "type": "object",
        "properties": { "sceneId": { "type": "integer", "minimum": 1 } },
        "required": ["sceneId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
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
pub(crate) fn args_delete_profile() -> Value {
    json!({
        "type": "object",
        "properties": { "profileId": { "type": "integer", "minimum": 1 } },
        "required": ["profileId"],
        "additionalProperties": true,
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
pub(crate) fn args_duplicate_project() -> Value {
    typed_input_schema::<crate::project::DuplicateProjectInput>()
}
pub(crate) fn args_diff_blueprint_versions() -> Value {
    strict_typed_input_schema::<crate::blueprint::DiffBlueprintVersionsInput>()
}
pub(crate) fn args_diff_profile_chapters() -> Value {
    strict_typed_input_schema::<crate::character::DiffProfileChaptersInput>()
}
pub(crate) fn args_execute_attribute_deletion() -> Value {
    strict_typed_input_schema::<crate::character::ExecuteAttributeDeletionInput>()
}
pub(crate) fn args_export_derived_project_data() -> Value {
    strict_typed_input_schema::<crate::project_transfer::ExportDerivedProjectDataInput>()
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
pub(crate) fn args_generate_chapter_scene() -> Value {
    optional_task_runtime_typed_input_schema::<crate::chapter::GenerateChapterSceneInput>()
}
pub(crate) fn args_get_implicit_review() -> Value {
    strict_typed_input_schema::<crate::character::GetImplicitReviewInput>()
}
pub(crate) fn args_get_inspiration_history() -> Value {
    strict_typed_input_schema::<crate::inspiration::InspirationHistoryInput>()
}
pub(crate) fn args_get_profile_state_at_chapter() -> Value {
    strict_typed_input_schema::<crate::character::GetProfileStateAtChapterInput>()
}
pub(crate) fn args_get_snapshot() -> Value {
    strict_typed_input_schema::<crate::character::GetSnapshotInput>()
}
pub(crate) fn args_get_reverse_engineering_candidate() -> Value {
    strict_typed_input_schema::<crate::import_reverse::GetReverseEngineeringCandidateInput>()
}
pub(crate) fn args_import_chapter() -> Value {
    strict_typed_input_schema::<crate::chapter::ImportChapterInput>()
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
pub(crate) fn args_list_deletion_marks() -> Value {
    strict_typed_input_schema::<crate::character::ListDeletionMarksInput>()
}
pub(crate) fn args_list_optimize_sessions() -> Value {
    strict_typed_input_schema::<crate::optimize::ListOptimizeSessionsInput>()
}
pub(crate) fn args_list_object_profiles() -> Value {
    strict_typed_input_schema::<crate::profile::ListObjectProfilesInput>()
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
pub(crate) fn args_list_snapshots() -> Value {
    strict_typed_input_schema::<crate::character::ListSnapshotsInput>()
}
pub(crate) fn args_modify_profile_attribute() -> Value {
    strict_typed_input_schema::<crate::character::ModifyProfileAttributeInput>()
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
pub(crate) fn args_preview_chapter_scene_request() -> Value {
    strict_runtime_typed_input_schema::<crate::chapter::GenerateChapterSceneInput>()
}
pub(crate) fn args_preview_inspiration_request() -> Value {
    strict_runtime_typed_input_schema::<crate::inspiration::PreviewInspirationRequestInput>()
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
pub(crate) fn args_preview_sync_all_character_profiles_request() -> Value {
    typed_input_schema::<crate::character::SyncAllCharacterProfilesInput>()
}
pub(crate) fn args_preview_sync_character_from_chapters_request() -> Value {
    typed_input_schema::<crate::character::SyncCharacterProfileInput>()
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
pub(crate) fn args_reorder_chapter_scenes() -> Value {
    strict_typed_input_schema::<crate::chapter::ReorderChapterScenesInput>()
}
pub(crate) fn args_request_attribute_deletion() -> Value {
    strict_typed_input_schema::<crate::character::RequestAttributeDeletionInput>()
}
pub(crate) fn args_record_optimize_agent_event() -> Value {
    strict_typed_input_schema::<crate::optimize::RecordOptimizeAgentEventInput>()
}
pub(crate) fn args_reset_deletion_marks() -> Value {
    strict_typed_input_schema::<crate::character::ResetDeletionMarksInput>()
}
pub(crate) fn args_restore_blueprint_version() -> Value {
    strict_runtime_typed_input_schema::<crate::blueprint::RestoreBlueprintVersionInput>()
}
pub(crate) fn args_rollback_to_chapter() -> Value {
    strict_typed_input_schema::<crate::character::RollbackToChapterInput>()
}
pub(crate) fn args_reverse_engineer() -> Value {
    optional_task_runtime_typed_input_schema::<crate::import_reverse::ReverseEngineerInput>()
}
pub(crate) fn args_save_blueprint() -> Value {
    strict_runtime_typed_input_schema::<crate::blueprint::SaveBlueprintInput>()
}
pub(crate) fn args_save_chapter_outline() -> Value {
    strict_typed_input_schema::<crate::outline::SaveChapterOutlineInput>()
}
pub(crate) fn args_save_part_outline() -> Value {
    strict_typed_input_schema::<crate::outline::SavePartOutlineInput>()
}
pub(crate) fn args_send_optimization_message() -> Value {
    strict_runtime_typed_input_schema::<crate::optimize::SendOptimizationMessageInput>()
}
pub(crate) fn args_set_profile_primary_material() -> Value {
    strict_typed_input_schema::<crate::profile::SetProfilePrimaryMaterialInput>()
}
pub(crate) fn args_start_blueprint_generation() -> Value {
    strict_runtime_typed_input_schema::<crate::blueprint::StartBlueprintGenerationInput>()
}
pub(crate) fn args_start_blueprint_refinement() -> Value {
    strict_runtime_typed_input_schema::<crate::blueprint::StartBlueprintRefinementInput>()
}
pub(crate) fn args_start_chapter_generation() -> Value {
    strict_runtime_typed_input_schema::<crate::chapter::StartChapterGenerationInput>()
}
pub(crate) fn args_start_inspiration_generation() -> Value {
    strict_runtime_typed_input_schema::<crate::inspiration::StartInspirationGenerationInput>()
}
pub(crate) fn args_start_optimize_analysis_generation() -> Value {
    strict_runtime_typed_input_schema::<crate::optimize::StartOptimizeAnalysisGenerationInput>()
}
pub(crate) fn args_start_optimize_chat_generation() -> Value {
    strict_runtime_typed_input_schema::<crate::optimize::StartOptimizeChatGenerationInput>()
}
pub(crate) fn args_start_optimize_patch_generation() -> Value {
    strict_runtime_typed_input_schema::<crate::optimize::StartOptimizePatchGenerationInput>()
}
pub(crate) fn args_start_outline_generation() -> Value {
    strict_runtime_typed_input_schema::<crate::outline::StartOutlineGenerationInput>()
}
pub(crate) fn args_sync_all_character_profiles() -> Value {
    required_task_runtime_typed_input_schema::<crate::character::SyncAllCharacterProfilesInput>()
}
pub(crate) fn args_sync_scene_profiles() -> Value {
    optional_task_runtime_typed_input_schema::<crate::profile::SyncSceneProfilesInput>()
}
pub(crate) fn args_sync_character_from_chapters() -> Value {
    required_task_runtime_typed_input_schema::<crate::character::SyncCharacterProfileInput>()
}
pub(crate) fn args_update_character_profile() -> Value {
    strict_runtime_typed_input_schema::<crate::character::UpdateCharacterProfileInput>()
}
pub(crate) fn args_update_chapter_content() -> Value {
    strict_typed_input_schema::<crate::chapter::UpdateChapterContentInput>()
}
pub(crate) fn args_update_chapter_scene() -> Value {
    strict_typed_input_schema::<crate::chapter::UpdateChapterSceneInput>()
}
pub(crate) fn args_update_project() -> Value {
    typed_input_schema::<crate::project::UpdateProjectInput>()
}
pub(crate) fn args_update_object_profile() -> Value {
    strict_typed_input_schema::<crate::profile::UpdateObjectProfileInput>()
}
pub(crate) fn args_update_foreshadowing() -> Value {
    strict_runtime_typed_input_schema::<crate::profile::UpdateForeshadowingInput>()
}
pub(crate) fn args_update_scene_profile() -> Value {
    strict_runtime_typed_input_schema::<crate::profile::UpdateSceneProfileInput>()
}
pub(crate) fn args_update_world_setting() -> Value {
    strict_runtime_typed_input_schema::<crate::blueprint::UpdateWorldSettingInput>()
}
pub(crate) fn args_upload_profile_material_image() -> Value {
    strict_typed_input_schema::<crate::profile::UploadProfileMaterialImageInput>()
}
pub(crate) fn result_character_conflicts() -> Value {
    typed_schema::<Vec<crate::character::CharacterConflictSummary>>()
}
pub(crate) fn result_discover_character_profiles() -> Value {
    typed_schema::<crate::character::DiscoverCharacterProfilesResult>()
}
pub(crate) fn result_project_generated_data_reset() -> Value {
    typed_schema::<crate::project::ProjectGeneratedDataResetResponse>()
}
pub(crate) fn result_prepare_chapter_generation_rag_embedding() -> Value {
    typed_schema::<crate::chapter::PreparedGenerationRagEmbeddingRequest>()
}
pub(crate) fn result_source_snapshot_chapters() -> Value {
    typed_schema::<Vec<crate::project::SourceSnapshotChapterSummary>>()
}
pub(crate) fn result_source_sync_plan() -> Value {
    typed_schema::<crate::source_sync::SourceSyncPlanSummary>()
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
pub(crate) fn result_diff_blueprint_versions() -> Value {
    typed_schema::<crate::blueprint::BlueprintDiffSummary>()
}
pub(crate) fn result_diff_profile_chapters() -> Value {
    typed_schema::<crate::character::SnapshotDeltaSummary>()
}
pub(crate) fn result_get_blueprint() -> Value {
    typed_schema::<Option<crate::blueprint::BlueprintSummary>>()
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
pub(crate) fn result_get_project_asset_dependency_graph() -> Value {
    typed_schema::<crate::project::AssetDependencyGraph>()
}
pub(crate) fn result_get_project_asset_dependency_graph_node_detail() -> Value {
    typed_schema::<crate::project::AssetGraphNodeDetail>()
}
pub(crate) fn result_get_project_status_machine() -> Value {
    typed_schema::<crate::project::ProjectStatusMachineSummary>()
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
pub(crate) fn result_list_blueprint_versions() -> Value {
    typed_schema::<Vec<crate::blueprint::BlueprintVersionSummary>>()
}
pub(crate) fn result_list_behaviors() -> Value {
    typed_schema::<Vec<crate::character::BehaviorRecordSummary>>()
}
pub(crate) fn result_list_deletion_marks() -> Value {
    typed_schema::<Vec<crate::character::DeletionMarkSummary>>()
}
pub(crate) fn result_inspiration_history() -> Value {
    typed_schema::<Vec<crate::inspiration::InspirationMessageSummary>>()
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
pub(crate) fn result_list_profiles() -> Value {
    typed_schema::<Vec<crate::character::CharacterProfileSummary>>()
}
pub(crate) fn result_list_snapshots() -> Value {
    typed_schema::<Vec<crate::character::SnapshotReferenceSummary>>()
}
pub(crate) fn result_project_summaries() -> Value {
    typed_schema::<Vec<crate::project::ProjectSummary>>()
}
pub(crate) fn result_list_projects() -> Value {
    typed_schema::<crate::project::PaginatedProjects>()
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
pub(crate) fn result_outline_generation_progress() -> Value {
    typed_schema::<crate::outline::OutlineGenerationProgressSummary>()
}
pub(crate) fn result_outline_mutation() -> Value {
    typed_schema::<crate::outline::OutlineMutationResult>()
}
pub(crate) fn result_part_outlines() -> Value {
    typed_schema::<Vec<crate::outline::PartOutlineSummary>>()
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
pub(crate) fn result_profile_material_group() -> Value {
    typed_schema::<crate::profile::ProfileMaterialGroup>()
}
pub(crate) fn result_request_attribute_deletion() -> Value {
    typed_schema::<crate::character::DeletionMarkSummary>()
}
pub(crate) fn result_reset_deletion_marks() -> Value {
    typed_schema::<u64>()
}
pub(crate) fn result_blueprint_mutation() -> Value {
    typed_schema::<crate::blueprint::BlueprintMutationResult>()
}
pub(crate) fn result_rollback_to_chapter() -> Value {
    typed_schema::<crate::character::RollbackResultSummary>()
}
pub(crate) fn result_save_chapter_outline() -> Value {
    typed_schema::<crate::outline::SaveChapterOutlineResult>()
}
pub(crate) fn result_save_part_outline() -> Value {
    typed_schema::<crate::outline::SavePartOutlineResult>()
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
pub(crate) fn result_update_character_profile() -> Value {
    typed_schema::<crate::character::CharacterProfileMutationResult>()
}
