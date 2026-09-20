use super::builders::*;
use serde_json::{Value, json};
pub(crate) fn args_reorder_chapter_scenes() -> Value {
    strict_typed_input_schema::<crate::chapter::ReorderChapterScenesInput>()
}

pub(crate) fn args_read_recent_logs() -> Value {
    typed_input_schema::<crate::logging::ReadLogsInput>()
}

pub(crate) fn args_list_workflow_runs() -> Value {
    strict_typed_input_schema::<crate::workflow::ListWorkflowRunsInput>()
}

pub(crate) fn args_refresh_token() -> Value {
    typed_input_schema::<crate::auth::RefreshTokenInput>()
}

pub(crate) fn args_revoke_session() -> Value {
    typed_input_schema::<crate::auth::RevokeSessionInput>()
}

pub(crate) fn args_register_user() -> Value {
    typed_input_schema::<crate::auth::RegisterInput>()
}

pub(crate) fn args_reorder_galgame_lines() -> Value {
    project_typed_input_schema::<crate::galgame::ReorderGalgameLinesInput>()
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

pub(crate) fn args_resume_workflow_run() -> Value {
    strict_runtime_typed_input_schema::<crate::workflow::ResumeWorkflowRunInput>()
}

pub(crate) fn args_reverse_engineer() -> Value {
    optional_task_runtime_typed_input_schema::<crate::import_reverse::ReverseEngineerInput>()
}

pub(crate) fn args_run_canvas_graph() -> Value {
    strict_typed_input_schema::<crate::canvas::RunCanvasGraphInput>()
}

pub(crate) fn args_run_canvas_node() -> Value {
    strict_typed_input_schema::<crate::canvas::RunCanvasNodeInput>()
}

pub(crate) fn args_run_galgame_asset_generation() -> Value {
    strict_typed_input_schema::<crate::galgame::RunGalgameAssetGenerationInput>()
}

pub(crate) fn args_save_blueprint() -> Value {
    strict_runtime_typed_input_schema::<crate::blueprint::SaveBlueprintInput>()
}

pub(crate) fn args_save_canvas_graph() -> Value {
    strict_typed_input_schema::<crate::canvas::SaveCanvasGraphInput>()
}

pub(crate) fn args_read_canvas_graph() -> Value {
    strict_typed_input_schema::<crate::canvas::ReadCanvasGraphInput>()
}
pub(crate) fn result_read_canvas_graph() -> Value {
    typed_schema::<crate::canvas::CanvasGraphReadResult>()
}
pub(crate) fn args_edit_canvas_graph() -> Value {
    strict_typed_input_schema::<crate::canvas::EditCanvasGraphInput>()
}
pub(crate) fn result_edit_canvas_graph() -> Value {
    typed_schema::<crate::canvas::CanvasGraphEditResult>()
}

pub(crate) fn args_save_canvas_text_asset() -> Value {
    strict_typed_input_schema::<crate::canvas::SaveCanvasTextAssetInput>()
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

pub(crate) fn args_select_segment_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "videoId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::video::SelectSegmentVideoInput>(),
        },
        "required": ["projectId", "chapterId", "videoId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn args_set_log_level() -> Value {
    typed_input_schema::<crate::logging::SetLogLevelInput>()
}

pub(crate) fn args_set_primary_portrait() -> Value {
    strict_typed_input_schema::<crate::portrait::SetPrimaryPortraitInput>()
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

pub(crate) fn args_start_galgame_pipeline() -> Value {
    strict_typed_input_schema::<crate::galgame::StartGalgamePipelineInput>()
}

pub(crate) fn args_start_image_generation() -> Value {
    strict_runtime_typed_input_schema::<crate::image_generation::StartImageGenerationInput>()
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

pub(crate) fn args_start_package_lifecycle() -> Value {
    strict_typed_input_schema::<crate::package::StartPackageLifecycleInput>()
}

pub(crate) fn args_start_outline_generation() -> Value {
    strict_runtime_typed_input_schema::<crate::outline::StartOutlineGenerationInput>()
}

pub(crate) fn args_start_portrait_generation() -> Value {
    strict_runtime_typed_input_schema::<crate::portrait::StartPortraitGenerationInput>()
}

pub(crate) fn args_start_video_generation() -> Value {
    strict_runtime_typed_input_schema::<crate::video::StartVideoGenerationInput>()
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

pub(crate) fn args_unbind_galgame_media() -> Value {
    strict_typed_input_schema::<crate::galgame::UnbindGalgameMediaInput>()
}

pub(crate) fn args_update_chapter_video_prompt() -> Value {
    json!({
        "type": "object",
        "properties": {
            "promptId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::video::UpdateChapterVideoPromptInput>(),
        },
        "required": ["promptId", "input"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn args_update_chapter_video_segment_plan() -> Value {
    json!({
        "type": "object",
        "properties": {
            "promptId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::video::UpdateChapterVideoSegmentPlanInput>(),
        },
        "required": ["promptId", "input"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn args_update_chapter_video_segment_recipe() -> Value {
    json!({
        "type": "object",
        "properties": {
            "promptId": { "type": "integer", "minimum": 1 },
            "recipe": raw_schema::<crate::video::VideoSegmentExecutionRecipeV1>(),
        },
        "required": ["promptId", "recipe"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
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

pub(crate) fn args_update_embedding_config() -> Value {
    config_typed_input_schema::<crate::config::UpdateEmbeddingInput>()
}

pub(crate) fn args_update_galgame_choices() -> Value {
    project_typed_input_schema::<crate::galgame::UpdateGalgameChoicesInput>()
}

pub(crate) fn args_update_galgame_line() -> Value {
    project_typed_input_schema::<crate::galgame::UpdateGalgameLineInput>()
}

pub(crate) fn args_update_galgame_scene() -> Value {
    project_typed_input_schema::<crate::galgame::UpdateGalgameSceneInput>()
}

pub(crate) fn args_update_image() -> Value {
    strict_typed_input_schema::<crate::image::UpdateGeneratedImageInput>()
}

pub(crate) fn args_update_image_config() -> Value {
    config_typed_input_schema::<crate::config::UpdateImageConfigInput>()
}

pub(crate) fn args_update_llm_config() -> Value {
    config_typed_input_schema::<crate::config::UpdateLlmInput>()
}

pub(crate) fn args_web_search() -> Value {
    typed_schema::<crate::web_search::WebSearchInput>()
}

pub(crate) fn args_web_fetch() -> Value {
    typed_schema::<crate::web_fetch::WebFetchInput>()
}

pub(crate) fn args_read_image() -> Value {
    typed_schema::<crate::image_read::ReadImageInput>()
}

pub(crate) fn args_update_manga_prompt() -> Value {
    strict_typed_input_schema::<crate::manga::UpdateMangaPromptInput>()
}

pub(crate) fn args_update_package_preferences() -> Value {
    strict_typed_input_schema::<crate::package::UpdatePackagePreferencesInput>()
}

pub(crate) fn args_update_project() -> Value {
    typed_input_schema::<crate::project::UpdateProjectInput>()
}

pub(crate) fn args_update_project_member_role() -> Value {
    json!({
        "type": "object",
        "properties": {
            "memberUserId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::collaboration::UpdateProjectMemberRoleInput>(),
        },
        "required": ["memberUserId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
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

pub(crate) fn args_member_user_id() -> Value {
    json!({
        "type": "object",
        "properties": { "memberUserId": { "type": "integer", "minimum": 1 } },
        "required": ["memberUserId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_update_theme() -> Value {
    json!({
        "type": "object",
        "properties": {
            "themeId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::theme::UpdateThemeInput>(),
        },
        "required": ["themeId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn args_update_user_profile() -> Value {
    typed_input_schema::<crate::auth::UpdateProfileInput>()
}

pub(crate) fn args_update_user_role() -> Value {
    json!({
        "type": "object",
        "properties": {
            "userId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::admin::UpdateUserRoleInput>(),
        },
        "required": ["userId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn args_trim_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "videoId": { "type": "integer", "minimum": 1 },
            "startSecs": { "type": "number", "minimum": 0 },
            "endSecs": { "type": "number", "minimum": 0 },
        },
        "required": ["videoId", "startSecs", "endSecs"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn args_update_assembly_push_status() -> Value {
    json!({
        "type": "object",
        "properties": {
            "videoId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::video::UpdateAssemblyPushStatusInput>(),
        },
        "required": ["videoId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn args_update_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "videoId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::video::UpdateGeneratedVideoInput>(),
        },
        "required": ["videoId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn args_update_video_config() -> Value {
    config_typed_input_schema::<crate::config::UpdateVideoConfigInput>()
}

pub(crate) fn args_update_video_segment_plan_draft_recipe() -> Value {
    strict_typed_input_schema::<crate::video::UpdateVideoSegmentPlanDraftRecipeInput>()
}

pub(crate) fn args_update_world_setting() -> Value {
    strict_runtime_typed_input_schema::<crate::blueprint::UpdateWorldSettingInput>()
}

pub(crate) fn args_upload_assembly_insert_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "workflowMode": { "type": "string", "minLength": 1 },
            "input": raw_schema::<crate::video::UploadAssemblyInsertVideoInput>(),
        },
        "required": ["projectId", "chapterId", "workflowMode", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn args_upload_chapter_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::video::UploadChapterVideoInput>(),
        },
        "required": ["projectId", "chapterId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn args_upload_image() -> Value {
    strict_typed_input_schema::<crate::image::UploadImageInput>()
}

pub(crate) fn args_upload_portrait() -> Value {
    strict_typed_input_schema::<crate::portrait::UploadPortraitInput>()
}

pub(crate) fn args_upload_profile_material_image() -> Value {
    strict_typed_input_schema::<crate::profile::UploadProfileMaterialImageInput>()
}

pub(crate) fn args_upload_reference_media_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::video::UploadReferenceMediaVideoInput>(),
        },
        "required": ["projectId", "chapterId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}
