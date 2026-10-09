use super::model::*;
use super::policy::*;
use super::schemas::*;
use crate::agent::{
    AgentToolCostClass, AgentToolExposure, AgentToolReplayPolicy, AgentToolReversibility,
    AgentToolRisk,
};
use crate::package::{PackageCommandCategory, PackageId};
use serde_json::{Value, json};
macro_rules! command_specs {
    ($($name:literal => $args:ident, $result:ident, $domain:literal $(, $method:ident($value:expr))*;)+) => {
        pub const API_COMMANDS: &[&str] = &[$($name,)+];
        pub(super) const COMMAND_SPECS: &[CommandSpec] = &[
            $(CommandSpec::new($name, $args, $result, $domain)$(.$method($value))*,)+
        ];
    };
}

command_specs! {
    "accept_import_review" => args_accept_import_review, result_import_review, "import", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_IMPORT), runtime(RuntimeConfigPolicy::Full);
    "accept_project_invite" => args_invite_id, result_unit, "project", context(NO_CONTEXT);
    "activate_theme" => args_activate_theme, result_activate_theme, "settings", context(NO_CONTEXT);
    "add_image_config" => args_add_image_config, result_add_image_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_IMAGE);
    "add_profile_attribute" => args_add_profile_attribute, result_unit, "profile";
    "add_video_config" => args_add_video_config, result_add_video_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_VIDEO);
    "adopt_media_authoring_document" => args_adopt_authoring_document, result_authoring_document, "media_authoring", context(NO_CONTEXT);
    "advance_galgame_story_runtime" => args_advance_galgame_story_runtime, result_galgame_story_runtime, "galgame", context(PROJECT_CONTEXT);
    "apply_assembly_postprocess_edit" => args_apply_assembly_postprocess_edit, result_generated_video, "video", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "apply_implicit_update_decision" => args_apply_implicit_update_decision, result_apply_implicit_update_decision, "profile";
    "apply_optimization_patches" => args_apply_optimization_patches, result_apply_optimization_patches, "optimize", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "apply_source_sync_decisions" => args_apply_source_sync, result_source_sync_plan, "source-sync", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "assemble_chapter_videos" => args_assemble_chapter_videos, result_assembled_chapter_video_preview, "video", context(PROJECT_CHAPTER_CONTEXT), capabilities(CAPABILITIES_VIDEO);
    "attribute_history" => args_attribute_history, result_attribute_history, "profile", effect(READ_EFFECT);
    "batch_accept_reviews" => args_batch_accept_reviews, result_import_reviews, "import", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_IMPORT), runtime(RuntimeConfigPolicy::Full);
    "bind_galgame_media" => args_bind_galgame_media, result_bind_galgame_media, "galgame", context(NO_CONTEXT), references(REFERENCES_MEDIA_BINDING_IDS_MEDIA_ASSET_IDS);
    "bind_manga_reference_image" => args_bind_manga_reference_image, result_bind_manga_reference_image, "manga", context(NO_CONTEXT);
    "cancel_admin_task" => args_task_id, result_unit, "task", identity(ADMIN_IDENTITY), effect(DESTRUCTIVE_EFFECT);
    "cancel_embedding_model_download" => args_cancel_embedding_model_download, result_unit, "rag", effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_EMBEDDING);
    "cancel_galgame_pipeline" => args_cancel_galgame_pipeline, result_cancel_galgame_pipeline, "galgame", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), execution("workflow"), references(REFERENCES_WORKFLOW_RUN_IDS), idempotency("server_workflow");
    "cancel_generation_task" => args_cancel_generation_task, result_generation_task_handle, "task", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "cancel_import_analysis" => args_project_id, result_unit, "import", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_IMPORT);
    "cancel_manga_pipeline" => args_cancel_manga_pipeline, result_cancel_manga_pipeline, "manga", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), execution("workflow"), idempotency("server_workflow");
    "cancel_task" => args_cancel_task, result_unit, "task", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "cancel_workflow" => args_cancel_workflow, result_unit, "workflow", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), execution("workflow"), references(REFERENCES_WORKFLOW_RUN_IDS), idempotency("server_workflow");
    "change_user_password" => args_change_user_password, result_unit, "auth", context(NO_CONTEXT), secrets(CHANGE_PASSWORD_SECRET_POINTERS);
    "check_character_conflict" => args_check_character_conflict, result_character_conflicts, "profile", context(NO_CONTEXT), effect(READ_EFFECT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "check_implicit_update" => args_check_implicit_update, result_check_implicit_update, "profile", effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "check_import_dedup" => args_check_import_dedup, result_dedup, "import", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMPORT);
    "check_profile_sync_conflict" => args_check_profile_sync_conflict, result_check_profile_sync_conflict, "profile", effect(READ_EFFECT);
    "cleanup_orphan_project_data_dirs" => args_cleanup_orphan_project_data_dirs, result_cleanup_orphan_project_data_dirs, "project", identity(ADMIN_IDENTITY), effect(DESTRUCTIVE_EFFECT), references(REFERENCES_PROJECT_IDS);
    "commit_inserted_segment_draft" => args_commit_inserted_segment_draft, result_video_timeline, "video";
    "confirm_chapter_visual_plan" => args_chapter_visual_plan_record, result_confirmed_chapter_visual_plan, "chapter", effect(DESTRUCTIVE_EFFECT), references(REFERENCES_PROJECT_IDS);
    "confirm_reverse_engineering_candidate" => args_confirm_reverse_engineering_candidate, result_reverse_engineering_candidate, "workflow", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_WORKFLOW_RUN_IDS), runtime(RuntimeConfigPolicy::Full), idempotency("safe");
    "create_canvas_project" => args_create_canvas_project, result_create_project, "project", context(NO_CONTEXT), references(REFERENCES_PROJECT_IDS);
    "create_derived_project" => args_create_derived_project, result_create_project, "project", context(NO_CONTEXT), references(REFERENCES_PROJECT_IDS);
    "create_embedding_config" => args_create_embedding_config, result_create_embedding_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_EMBEDDING);
    "create_foreshadowing" => args_create_foreshadowing, result_foreshadowing_write, "profile", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "create_invite_code" => args_create_invite_code, result_invite_code, "collaboration", identity(ADMIN_IDENTITY);
    "create_llm_config" => args_create_llm_config, result_create_llm_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS);
    "create_object_profile" => args_create_object_profile, result_object_profile_mutation, "profile";
    "create_optimization_analysis_session" => args_create_optimization_analysis_session, result_optimization_session, "optimize", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "create_optimization_chat_session" => args_create_optimization_chat_session, result_optimization_session, "optimize", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "create_profile" => args_create_profile, result_create_profile, "profile", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "create_project" => args_create_project, result_create_project, "project", context(NO_CONTEXT), references(REFERENCES_PROJECT_IDS);
    "create_scene_profile" => args_create_scene_profile, result_scene_profile_mutation, "profile", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "create_standalone_comic_project" => args_create_standalone_comic_project, result_create_project, "project", context(NO_CONTEXT), references(REFERENCES_PROJECT_IDS);
    "create_theme" => args_create_theme, result_activate_theme, "settings", context(NO_CONTEXT);
    "create_user" => args_create_user, result_get_current_user, "auth", identity(ADMIN_IDENTITY), secrets(PASSWORD_SECRET_POINTERS);
    "decline_project_invite" => args_invite_id, result_unit, "project", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_chapter_scene" => args_delete_chapter_scene, result_unit, "chapter", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_chapter_shooting_script" => args_delete_chapter_shooting_script, result_unit, "chapter", effect(DESTRUCTIVE_EFFECT), references(REFERENCES_PROJECT_IDS);
    "delete_chapter_video_prompt" => args_delete_chapter_video_prompt, result_unit, "video", effect(DESTRUCTIVE_EFFECT);
    "delete_embedding_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_EMBEDDING);
    "delete_foreshadowing" => args_delete_foreshadowing, result_foreshadowing_write, "profile", effect(DESTRUCTIVE_EFFECT);
    "delete_galgame_line" => args_delete_galgame_line, result_unit, "galgame", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_image" => args_delete_image, result_unit, "asset", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_import_review" => args_import_review_id, result_unit, "import", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_IMPORT);
    "delete_inserted_segment" => args_delete_inserted_segment, result_video_timeline, "video", effect(DESTRUCTIVE_EFFECT);
    "delete_latest_chapter_outlines" => args_delete_latest_outline_records, result_outline_mutation, "chapter", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "delete_latest_parts" => args_delete_latest_outline_records, result_outline_mutation, "outline", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "delete_llm_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_local_embedding_model" => args_delete_local_embedding_model, result_unit, "rag", effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_EMBEDDING);
    "delete_manga_by_chapter" => args_cancel_manga_pipeline, result_unit, "manga", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_manga_page" => args_delete_manga_page, result_unit, "manga", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_manga_pipeline_artifact" => args_delete_manga_pipeline_artifact, result_unit, "manga", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_manga_prompt" => args_delete_manga_prompt, result_unit, "manga", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_media_authoring_document" => args_delete_authoring_document, result_unit, "media_authoring", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_object_profile" => args_delete_object_profile, result_object_profile_delete, "profile", effect(DESTRUCTIVE_EFFECT);
    "delete_optimize_session" => args_optimize_session, result_unit, "optimize", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_profile" => args_delete_profile, result_delete_profile, "profile", effect(DESTRUCTIVE_EFFECT);
    "delete_project" => args_project_id, result_unit, "project", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT), references(REFERENCES_PROJECT_IDS);
    "delete_scene_profile" => args_delete_scene_profile, result_scene_profile_delete, "profile", effect(DESTRUCTIVE_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "delete_theme" => args_activate_theme, result_unit, "settings", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_theme_resource" => args_delete_theme_resource, result_unit, "settings", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_video" => args_video_id, result_unit, "video", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "detach_derived_project" => args_source_sync_project, result_unit, "source-sync", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "detect_image_config_models" => args_detect_image_config_models, result_detect_image_config_models, "settings", context(NO_CONTEXT), effect(READ_EFFECT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_IMAGE);
    "detect_llm_config_models" => args_detect_image_config_models, result_detect_image_config_models, "settings", context(NO_CONTEXT), effect(READ_EFFECT), secrets(PROVIDER_SECRET_POINTERS);
    "detect_video_config_models" => args_detect_image_config_models, result_detect_image_config_models, "settings", context(NO_CONTEXT), effect(READ_EFFECT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_VIDEO);
    "diff_blueprint_versions" => args_diff_blueprint_versions, result_diff_blueprint_versions, "blueprint", context(NO_CONTEXT), effect(READ_EFFECT);
    "diff_profile_chapters" => args_diff_profile_chapters, result_diff_profile_chapters, "profile", effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "disable_user" => args_user_id, result_unit, "auth", identity(ADMIN_IDENTITY), effect(DESTRUCTIVE_EFFECT);
    "discard_chapter_visual_plan_draft" => args_chapter_visual_plan_record, result_unit, "chapter", effect(DESTRUCTIVE_EFFECT), references(REFERENCES_PROJECT_IDS);
    "discard_optimization_patch_set" => args_discard_optimization_patch_set, result_unit, "optimize", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "discard_pending_optimize_patch_sets" => args_optimize_session, result_unit, "optimize", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "discover_character_profiles" => args_discover_character_profiles, result_discover_character_profiles, "profile", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "download_embedding_model" => args_download_embedding_model, result_download_embedding_model, "rag", execution("task"), capabilities(CAPABILITIES_EMBEDDING);
    "duplicate_project" => args_duplicate_project, result_create_project, "project", context(NO_CONTEXT), references(REFERENCES_PROJECT_IDS);
    "edit_canvas_graph" => args_edit_canvas_graph, result_edit_canvas_graph, "canvas", context(NO_CONTEXT);
    "edit_chapter_shooting_script" => args_edit_chapter_shooting_script, result_edit_chapter_shooting_script, "chapter", references(REFERENCES_PROJECT_IDS);
    "edit_chapter_shooting_script_progress" => args_edit_chapter_shooting_script_progress, result_edit_chapter_shooting_script_progress, "chapter", references(REFERENCES_PROJECT_IDS);
    "edit_chapter_visual_plan_draft" => args_edit_chapter_visual_plan_draft, result_edit_chapter_visual_plan_draft, "chapter", idempotency("operation_id");
    "execute_attribute_deletion" => args_execute_attribute_deletion, result_unit, "profile", effect(DESTRUCTIVE_EFFECT);
    "export_all_configs_json" => args_export_all_configs_json, result_string, "export", identity(ADMIN_IDENTITY), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT), references(REFERENCES_ARTIFACTS);
    "export_batch_data" => args_export_batch_data, result_export_batch_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT);
    "export_canvas_project_data" => args_export_canvas_project_data, result_export_project_package_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT), references(REFERENCES_ARTIFACTS), capabilities(CAPABILITIES_VIDEO_EXPORT);
    "export_characters_json_data" => args_export_characters_json_data, result_export_characters_json_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT);
    "export_derived_project_data" => args_export_derived_project_data, result_export_project_package_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT), references(REFERENCES_ARTIFACTS), capabilities(CAPABILITIES_EXPORT);
    "export_epub_data" => args_export_epub_data, result_export_characters_json_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT);
    "export_galgame_webgal" => args_cancel_galgame_pipeline, result_export_characters_json_data, "galgame", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT), references(REFERENCES_ARTIFACTS), capabilities(CAPABILITIES_EXPORT);
    "export_markdown_data" => args_export_epub_data, result_export_characters_json_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT);
    "export_outline_json_data" => args_export_characters_json_data, result_export_characters_json_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT);
    "export_pdf_data" => args_export_characters_json_data, result_export_characters_json_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT);
    "export_project_bundle_data" => args_export_project_bundle_data, result_export_project_package_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT), references(REFERENCES_ARTIFACTS), capabilities(CAPABILITIES_EXPORT);
    "export_project_family_data" => args_export_project_family_data, result_export_project_package_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT), references(REFERENCES_ARTIFACTS), capabilities(CAPABILITIES_EXPORT);
    "export_theme" => args_activate_theme, result_string, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT), capabilities(CAPABILITIES_EXPORT);
    "export_theme_package" => args_activate_theme, result_theme_package_export, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT), capabilities(CAPABILITIES_EXPORT);
    "export_txt_data" => args_export_characters_json_data, result_export_characters_json_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT);
    "forward_project_status" => args_forward_project_status, result_create_project, "project", context(PROJECT_CONTEXT), references(REFERENCES_PROJECT_IDS);
    "generate_canvas_text_asset_draft" => args_canvas_text_asset_draft, result_canvas_text_asset_draft, "canvas", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full);
    "generate_chapter_analysis" => args_chapter_analysis_task, result_chapter_analysis_mutation, "chapter", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "generate_chapter_scene" => args_generate_chapter_scene, result_chapter_scene, "chapter", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "generate_media_adaptation_plan" => args_generate_adaptation_plan, result_authoring_document, "media_authoring", context(NO_CONTEXT), execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("server_workflow");
    "generate_object_profile_image_prompt" => args_generate_object_profile_image_prompt, result_object_profile_image_prompt, "profile", context(NO_CONTEXT), effect(READ_EFFECT);
    "generate_portrait_prompt" => args_generate_portrait_prompt, result_string, "prompt", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full);
    "generate_scene_profile_image_prompt" => args_generate_scene_profile_image_prompt, result_scene_profile_image_prompt, "profile", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_admin_overview" => empty_args_schema, result_get_admin_overview, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "get_blueprint" => args_project_id, result_get_blueprint, "blueprint", context(PROJECT_CONTEXT), effect(READ_EFFECT);
    "get_canvas_graph" => args_get_canvas_graph, result_get_canvas_graph, "canvas", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_canvas_node_contracts" => args_get_canvas_node_contracts, result_get_canvas_node_contracts, "canvas", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_chapter" => args_get_chapter, result_chapter_summary, "chapter", context(PROJECT_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "get_chapter_outlines_pacing" => args_project_id, result_chapter_outline_pacing, "chapter", context(PROJECT_CONTEXT), effect(READ_EFFECT);
    "get_chapter_shooting_script" => args_get_chapter_shooting_script, result_get_chapter_shooting_script, "chapter", effect(READ_EFFECT);
    "get_chapter_shooting_script_unit_progress" => args_get_chapter_shooting_script, result_get_chapter_shooting_script_unit_progress, "chapter", effect(READ_EFFECT);
    "get_chapter_video_timeline" => args_get_video_timeline, result_video_timeline, "video", effect(READ_EFFECT);
    "get_chapter_visual_plan" => args_chapter_visual_plan_source, result_chapter_visual_plan_set, "chapter", effect(READ_EFFECT), references(REFERENCES_PROJECT_IDS);
    "get_character_arc_status" => args_character_profile_value, result_optional_json_value, "profile", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_character_voice_fingerprint" => args_character_profile_value, result_optional_json_value, "profile", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_current_user" => empty_args_schema, result_get_current_user, "auth", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_embedding_model_download_info" => args_delete_local_embedding_model, result_get_embedding_model_download_info, "rag", effect(READ_EFFECT), capabilities(CAPABILITIES_EMBEDDING);
    "get_galgame_asset_plan" => args_cancel_galgame_pipeline, result_get_galgame_asset_plan, "galgame", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_GENERATION_SPEC_IDS);
    "get_galgame_pipeline_status" => args_cancel_galgame_pipeline, result_cancel_galgame_pipeline, "galgame", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow"), references(REFERENCES_WORKFLOW_RUN_IDS);
    "get_galgame_script" => args_cancel_galgame_pipeline, result_find_galgame_script, "galgame", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_GALGAME_SCRIPT_IDS);
    "get_galgame_story_runtime_state" => args_get_galgame_story_runtime_state, result_get_galgame_story_runtime_state, "galgame", context(PROJECT_CONTEXT), effect(READ_EFFECT);
    "get_generation_call_analytics" => args_get_generation_call_analytics, result_get_generation_call_analytics, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "get_generation_progress" => args_project_id, result_outline_generation_progress, "task", context(PROJECT_CONTEXT), effect(READ_EFFECT);
    "get_implicit_review" => args_get_implicit_review, result_get_implicit_review, "profile", effect(READ_EFFECT);
    "get_inspiration_history" => args_get_inspiration_history, result_inspiration_history, "inspiration", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_log_dir" => empty_args_schema, result_string, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "get_manga_pipeline_status" => args_cancel_manga_pipeline, result_get_manga_pipeline_status, "manga", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow");
    "get_media_authoring_workspace" => args_media_authoring, result_media_authoring_workspace, "media_authoring", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_model_usage_report" => args_get_model_usage_report, result_get_model_usage_report, "cost", effect(READ_EFFECT);
    "get_model_usage_request" => args_get_model_usage_request, result_get_model_usage_request, "cost", effect(READ_EFFECT);
    "get_optimize_session" => args_optimize_session, result_optimization_session, "optimize", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_package_operation" => args_get_package_operation, result_package_lifecycle, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "get_package_overview" => empty_args_schema, result_package_overview, "system", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_package_preferences" => empty_args_schema, result_package_preferences, "settings", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_primary_portrait" => args_get_primary_portrait, result_get_primary_portrait, "profile", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE);
    "get_profile_state_at_chapter" => args_get_profile_state_at_chapter, result_get_profile_state_at_chapter, "profile", effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "get_project" => args_project_id, result_create_project, "project", context(PROJECT_CONTEXT), effect(READ_EFFECT), references(REFERENCES_PROJECT_IDS);
    "get_project_asset_dependency_graph" => args_project_id, result_get_project_asset_dependency_graph, "project", context(PROJECT_CONTEXT), effect(READ_EFFECT), references(REFERENCES_PROJECT_IDS);
    "get_project_asset_dependency_graph_node_detail" => args_project_asset_dependency_graph_node_detail, result_get_project_asset_dependency_graph_node_detail, "project", context(PROJECT_CONTEXT), effect(READ_EFFECT), references(REFERENCES_PROJECT_IDS);
    "get_project_fact_feedback" => args_project_id, result_project_fact_feedback, "project", context(PROJECT_CONTEXT), effect(READ_EFFECT);
    "get_project_import_analysis_progress" => args_project_id, result_import_analysis_progress, "import", context(PROJECT_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMPORT);
    "get_project_status_machine" => args_project_id, result_get_project_status_machine, "project", context(PROJECT_CONTEXT), effect(READ_EFFECT);
    "get_queue_stats" => empty_args_schema, result_get_queue_stats, "system", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_reverse_engineering_candidate" => args_get_reverse_engineering_candidate, result_optional_reverse_engineering_candidate, "workflow", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_WORKFLOW_RUN_IDS);
    "get_snapshot" => args_get_snapshot, result_get_snapshot, "profile", effect(READ_EFFECT);
    "get_storage_paths" => empty_args_schema, result_get_storage_paths, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "get_task_history" => args_get_task_history, result_get_task_history, "task", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_TASK_IDS);
    "get_theme_preference" => empty_args_schema, result_theme_preference, "settings", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_token_summary" => args_user_id, result_json_value, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "get_usage_trends" => args_get_usage_trends, result_get_usage_trends, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "get_user_usage" => args_user_usage, result_get_user_usage, "auth", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "get_workflow_progress_events" => args_get_workflow_progress_events, result_get_workflow_progress_events, "workflow", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow"), references(REFERENCES_WORKFLOW_RUN_IDS);
    "get_workflow_run_detail" => args_get_workflow_run_detail, result_get_workflow_run_detail, "workflow", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow"), references(REFERENCES_WORKFLOW_RUN_IDS);
    "health_check" => empty_args_schema, result_json_value, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "import_all_configs" => args_import_all_configs, result_import_all_configs, "import", identity(ADMIN_IDENTITY), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT);
    "import_canvas_media" => args_import_canvas_media, result_import_canvas_media, "canvas", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "import_canvas_project_upload" => args_import_canvas_project_upload, result_create_project, "import", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_VIDEO_IMPORT);
    "import_chapter" => args_import_chapter, result_chapter_summary, "import", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS), capabilities(CAPABILITIES_IMPORT), runtime(RuntimeConfigPolicy::Full);
    "import_galgame_audio" => args_import_galgame_audio, result_bind_galgame_media, "galgame", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), references(REFERENCES_MEDIA_BINDING_IDS_MEDIA_ASSET_IDS), capabilities(CAPABILITIES_IMPORT);
    "import_project_bundle_upload" => args_import_project_bundle_upload, result_project_bundle_import_report, "import", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "import_project_family_upload" => args_import_project_family_upload, result_project_summaries, "import", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "import_project_text_asset_upload" => args_import_project_text_upload, result_save_canvas_text_asset, "import", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "import_project_text_upload" => args_import_project_text_upload, result_import_reviews, "import", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "import_theme" => args_create_theme, result_activate_theme, "import", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "import_theme_package" => args_theme_package_upload, result_activate_theme, "import", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "insert_galgame_line" => args_insert_galgame_line, result_insert_galgame_line, "galgame", context(PROJECT_CONTEXT), references(REFERENCES_GALGAME_LINE_IDS);
    "inspect_project_bundle_upload" => args_inspect_project_bundle_upload, result_project_bundle_import_report, "import", context(NO_CONTEXT), effect(READ_EFFECT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "invite_project_member" => args_invite_project_member, result_unit, "project", context(PROJECT_CONTEXT);
    "leave_project" => empty_args_schema, result_unit, "project", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "list_active_workflows" => empty_args_schema, result_list_active_workflows, "workflow", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow"), references(REFERENCES_TASK_IDS);
    "list_agent_skills" => empty_args_schema, result_agent_skill_settings, "agent", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_all_projects" => empty_args_schema, result_list_all_projects, "project", identity(ADMIN_IDENTITY), effect(READ_EFFECT), references(REFERENCES_PROJECT_IDS);
    "list_all_users" => empty_args_schema, result_users, "auth", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "list_behaviors" => args_list_behaviors, result_list_behaviors, "profile", effect(READ_EFFECT);
    "list_blueprint_versions" => args_list_blueprint_versions, result_list_blueprint_versions, "blueprint", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_canvas_text_asset_versions" => args_list_canvas_text_asset_versions, result_canvas_text_asset_versions, "canvas", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_canvas_text_assets" => args_list_canvas_text_assets, result_canvas_text_assets, "canvas", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_chapter_analyses" => args_project_id, result_chapter_analyses, "chapter", context(PROJECT_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "list_chapter_index" => args_project_id, result_chapter_index, "chapter", context(PROJECT_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "list_chapter_outlines" => args_project_id, result_chapter_outlines, "chapter", context(PROJECT_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "list_chapter_scenes" => args_list_chapter_scenes, result_chapter_scenes, "chapter", context(CHAPTER_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "list_chapter_video_prompts" => args_list_chapter_video_prompts, result_list_chapter_video_prompts, "video", context(PROJECT_CHAPTER_CONTEXT), effect(READ_EFFECT);
    "list_chapters" => args_project_id, result_chapter_summaries, "chapter", context(PROJECT_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "list_deletion_marks" => args_list_deletion_marks, result_list_deletion_marks, "profile", effect(READ_EFFECT);
    "list_embedding_config" => empty_args_schema, result_list_embedding_config, "settings", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_EMBEDDING);
    "list_foreshadowing" => args_list_foreshadowing, result_list_foreshadowing, "profile", effect(READ_EFFECT);
    "list_image_configs" => empty_args_schema, result_list_image_configs, "settings", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE);
    "list_images_by_chapter" => args_list_images_by_chapter, result_list_images_by_chapter, "chapter", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "list_images_by_project" => args_list_images_by_project, result_list_images_by_chapter, "asset", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_images_by_projects" => args_list_images_by_projects, result_list_images_by_chapter, "asset", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_import_queue" => args_import_project_reviews, result_import_reviews, "import", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMPORT);
    "list_invite_codes" => empty_args_schema, result_invite_codes, "collaboration", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "list_llm_config" => empty_args_schema, result_list_llm_config, "settings", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_local_embedding_models" => empty_args_schema, result_list_local_embedding_models, "rag", effect(READ_EFFECT), capabilities(CAPABILITIES_EMBEDDING);
    "list_manga_page_prompts" => args_cancel_manga_pipeline, result_list_manga_page_prompts, "manga", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_manga_pipeline_artifacts" => args_manga_chapter, result_list_manga_pipeline_artifacts, "manga", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_manga_pipeline_statuses" => args_list_manga_pipeline_statuses, result_list_manga_pipeline_statuses, "manga", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow");
    "preview_delete_manga_pipeline_artifact" => args_preview_delete_manga_pipeline_artifact, result_manga_pipeline_artifact_deletion_preview, "manga", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_model_provider_catalog" => empty_args_schema, result_list_model_provider_catalog, "system", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_my_project_invites" => empty_args_schema, result_list_my_project_invites, "project", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_object_profiles" => args_list_object_profiles, result_list_object_profiles, "profile", effect(READ_EFFECT);
    "list_optimize_patch_sets" => args_optimize_session, result_optimization_patch_sets, "optimize", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_optimize_sessions" => args_list_optimize_sessions, result_optimization_sessions, "optimize", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_optimize_turns" => args_optimize_session, result_optimization_turns, "optimize", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_orphan_project_data_dirs" => empty_args_schema, result_cleanup_orphan_project_data_dirs, "project", identity(ADMIN_IDENTITY), effect(READ_EFFECT), references(REFERENCES_PROJECT_IDS);
    "list_package_command_availability" => args_list_package_command_availability, result_package_command_availability, "system", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_parts" => args_project_id, result_part_outlines, "outline", context(PROJECT_CONTEXT), effect(READ_EFFECT);
    "list_pending_tasks" => empty_args_schema, result_list_pending_tasks, "task", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "list_portraits_by_project" => args_list_portraits_by_project, result_list_portraits, "profile", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE);
    "list_profile_materials" => args_list_profile_materials, result_list_profile_materials, "profile", effect(READ_EFFECT);
    "list_profiles" => args_list_profiles, result_list_profiles, "profile", effect(READ_EFFECT);
    "list_project_invites" => empty_args_schema, result_list_my_project_invites, "project", context(PROJECT_CONTEXT), effect(READ_EFFECT);
    "list_project_members" => empty_args_schema, result_list_project_members, "project", context(PROJECT_CONTEXT), effect(READ_EFFECT);
    "list_projects" => args_list_projects, result_list_projects, "project", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_PROJECT_IDS);
    "list_scene_profiles" => args_list_scene_profiles, result_list_scene_profiles, "profile", effect(READ_EFFECT);
    "list_sessions" => empty_args_schema, result_list_sessions, "auth", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_snapshots" => args_list_snapshots, result_list_snapshots, "profile", effect(READ_EFFECT);
    "list_source_snapshot_chapters" => args_project_id, result_source_snapshot_chapters, "chapter", context(PROJECT_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "list_themes" => empty_args_schema, result_list_themes, "settings", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_video_adaptation_references" => args_media_authoring, result_video_reference_candidates, "media_authoring", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_video_configs" => empty_args_schema, result_list_video_configs, "settings", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_VIDEO);
    "list_video_reference_images" => args_list_video_reference_images, result_list_images_by_chapter, "asset", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_videos_by_chapter" => args_list_videos_by_chapter, result_list_videos_by_chapter, "video", context(PROJECT_CHAPTER_CONTEXT), effect(READ_EFFECT);
    "list_workflow_runs" => args_list_workflow_runs, result_list_workflow_runs, "workflow", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow"), references(REFERENCES_WORKFLOW_RUN_IDS);
    "load_cached_assembled_chapter_video_preview" => args_assemble_chapter_videos, result_optional_assembled_chapter_video_preview, "video", context(PROJECT_CHAPTER_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_VIDEO);
    "load_video_playback_data" => args_video_id, result_video_playback_data, "video", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_VIDEO);
    "load_video_scrub_playback_data" => args_load_video_scrub_playback_data, result_video_playback_data, "video", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_VIDEO);
    "load_video_segment_plan_draft" => args_load_video_segment_plan_draft, result_video_segment_plan_draft, "video", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_VIDEO);
    "login_user" => args_login_user, result_login_user, "auth", identity(PUBLIC_IDENTITY), secrets(PASSWORD_SECRET_POINTERS);
    "modify_profile_attribute" => args_modify_profile_attribute, result_unit, "profile";
    "move_inserted_segment" => args_move_inserted_segment, result_video_timeline, "video";
    "oauth2_authorize_url" => args_oauth2_authorize_url, result_string, "auth", identity(PUBLIC_IDENTITY);
    "oauth2_available_providers" => args_oauth2_available_providers, result_oauth2_available_providers, "auth", identity(PUBLIC_IDENTITY), effect(READ_EFFECT);
    "oauth2_callback" => args_oauth2_callback, result_login_user, "auth", identity(PUBLIC_IDENTITY), secrets(OAUTH_CODE_SECRET_POINTERS);
    "optimize_chapter_video_prompt" => args_optimize_chapter_video_prompt, result_optimize_chapter_video_prompt, "video", secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full);
    "optimize_manga_prompt" => args_optimize_manga_prompt, result_bind_manga_reference_image, "manga", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full);
    "plan_galgame_assets" => args_cancel_galgame_pipeline, result_plan_galgame_assets, "galgame", context(NO_CONTEXT), references(REFERENCES_GENERATION_SPEC_IDS), idempotency("safe");
    "poll_video_job" => args_poll_video_job, result_generated_video, "video", context(NO_CONTEXT), effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_VIDEO), runtime(RuntimeConfigPolicy::Full);
    "prepare_chapter_generation_rag_embedding" => args_prepare_chapter_generation_rag_embedding, result_prepare_chapter_generation_rag_embedding, "rag", context(PROJECT_CONTEXT), effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_EMBEDDING_RAG), runtime(RuntimeConfigPolicy::Full);
    "prepare_optimization_analysis_session" => args_prepare_optimization_analysis_session, result_optimization_session, "optimize", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "prepare_standalone_comic_source" => args_prepare_standalone_comic_source, result_prepare_standalone_comic_source, "manga", context(NO_CONTEXT), idempotency("safe");
    "preview_apply_beat_mapping_request" => args_apply_beat_mapping, result_model_request_preview, "system", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_batch_generate_part_outlines_request" => args_batch_generate_part_outlines, result_model_request_preview, "outline", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_batch_images_request" => args_preview_batch_images_request, result_model_request_preview, "asset", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_canvas_node_request" => args_preview_canvas_node_request, result_model_request_preview, "canvas", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_canvas_text_asset_draft_request" => args_canvas_text_asset_draft, result_model_request_preview, "canvas", context(NO_CONTEXT), effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full);
    "preview_chapter_analysis_request" => args_chapter_analysis, result_model_request_preview, "chapter", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_chapter_prompt" => args_preview_chapter_prompt, result_model_request_preview, "chapter", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_chapter_scene_request" => args_preview_chapter_scene_request, result_model_request_preview, "chapter", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_chapter_video_request" => args_preview_chapter_video_request, result_model_request_preview, "video", context(PROJECT_CHAPTER_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_VIDEO), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_check_character_conflict_request" => args_preview_check_character_conflict, result_model_request_preview, "profile", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_check_implicit_update_request" => args_check_implicit_update, result_model_request_preview, "profile", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_continue_part_outlines_request" => args_continue_part_outlines, result_model_request_preview, "outline", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_create_optimization_analysis_session_request" => args_create_optimization_analysis_session, result_model_request_preview, "optimize", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_discover_character_profiles_request" => args_preview_discover_character_profiles, result_model_request_preview, "profile", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_galgame_pipeline_request" => args_preview_galgame_pipeline_request, result_model_request_preview, "galgame", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_age_variant_request" => args_preview_generate_age_variant_request, result_model_request_preview, "profile", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_blueprint_request" => args_preview_generate_blueprint_request, result_model_request_preview, "blueprint", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_chapter_outlines_by_count_request" => args_generate_chapter_outlines_by_count, result_model_request_preview, "chapter", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_missing_portraits_request" => args_preview_generate_missing_portraits_request, result_model_request_preview, "profile", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_part_chapter_outlines_request" => args_generate_part_chapter_outlines, result_model_request_preview, "chapter", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_parts_request" => args_generate_parts, result_model_request_preview, "outline", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_portrait_prompt_request" => args_preview_generate_portrait_prompt_request, result_model_request_preview, "prompt", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_portrait_request" => args_preview_generate_portrait_request, result_model_request_preview, "profile", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT_LLM_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_import_project_analysis_request" => args_project_id, result_model_request_preview, "import", context(PROJECT_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMPORT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_inspiration_request" => args_preview_inspiration_request, result_model_request_preview, "inspiration", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_media_adaptation_plan_request" => args_generate_adaptation_plan, result_model_request_preview, "media_authoring", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_delete_media_authoring_document" => args_preview_delete_authoring_document, result_authoring_document_deletion_impact, "media_authoring", context(NO_CONTEXT), effect(READ_EFFECT);
    "preview_object_profile_image_request" => args_preview_object_profile_image_request, result_model_request_preview, "profile", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_page_image_request" => args_preview_page_image_request, result_model_request_preview, "asset", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_prepare_optimization_analysis_session_request" => args_prepare_optimization_analysis_session, result_model_request_preview, "optimize", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_project_fact_feedback_request" => args_project_fact_feedback_preview, result_model_request_preview, "project", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_propose_optimization_patches_request" => args_propose_optimization_patches, result_model_request_preview, "optimize", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_reference_media_video_request" => args_preview_reference_media_video_request, result_model_request_preview, "video", context(PROJECT_CHAPTER_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_VIDEO), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_refine_blueprint_request" => args_preview_refine_blueprint_request, result_model_request_preview, "blueprint", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_regenerate_chapter_outline_request" => args_regenerate_chapter_outline, result_model_request_preview, "chapter", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_regenerate_last_part_request" => args_regenerate_last_part, result_model_request_preview, "outline", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_regenerate_part_outline_request" => args_regenerate_part_outline, result_model_request_preview, "outline", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_regenerate_parts_request" => args_regenerate_parts, result_model_request_preview, "outline", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_regenerate_portrait_request" => args_preview_regenerate_portrait_request, result_model_request_preview, "profile", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_resume_chapter_analysis_sync_request" => args_chapter_analysis, result_model_request_preview, "chapter", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_resume_manga_pipeline_request" => args_preview_resume_manga_pipeline_request, result_model_request_preview, "manga", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow"), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_reverse_engineer_request" => args_preview_reverse_engineer, result_model_request_preview, "workflow", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_rollback" => args_forward_project_status, result_preview_rollback, "project", context(PROJECT_CONTEXT), effect(READ_EFFECT);
    "preview_scene_image_request" => args_preview_scene_image_request, result_model_request_preview, "profile", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_send_optimization_message_request" => args_send_optimization_message, result_model_request_preview, "optimize", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_source_sync" => args_source_sync_project, result_source_sync_plan, "source-sync", context(NO_CONTEXT), effect(READ_EFFECT);
    "preview_start_manga_pipeline_request" => args_preview_start_manga_pipeline_request, result_model_request_preview, "manga", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow"), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_sync_all_character_profiles_request" => args_preview_sync_all_character_profiles_request, result_model_request_preview, "profile", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_sync_character_from_chapters_request" => args_preview_sync_character_from_chapters_request, result_model_request_preview, "chapter", effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_test_embedding_config_request" => args_preview_test_embedding_config_request, result_model_request_preview, "settings", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_EMBEDDING), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_test_image_config_request" => args_preview_test_image_config_request, result_model_request_preview, "settings", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_test_llm_config_request" => args_preview_test_image_config_request, result_model_request_preview, "settings", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_video_provider_request_by_video_id" => args_preview_video_provider_request_by_video_id, result_model_request_preview, "video", context(PROJECT_CHAPTER_CONTEXT), effect(READ_EFFECT);
    "preview_visual_plan_point_generation" => args_preview_visual_plan_point_generation, result_visual_plan_point_generation_preview, "chapter", effect(READ_EFFECT);
    "prompt_list" => empty_args_schema, result_prompt_list, "prompt", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_list_categories" => empty_args_schema, result_prompt_list_categories, "prompt", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_list_project_overrides" => args_project_id, result_prompt_list_project_overrides, "prompt", context(PROJECT_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_load_defaults" => empty_args_schema, result_prompt_load_defaults, "prompt", context(NO_CONTEXT), capabilities(CAPABILITIES_PROMPT);
    "prompt_remove_project_override" => args_prompt_remove_project_override, result_unit, "prompt", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_render" => args_prompt_render, result_string, "prompt", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_render_ab" => args_prompt_render, result_prompt_render_ab, "prompt", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_reset" => args_prompt_name, result_unit, "prompt", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_set_project_override" => args_prompt_set_project_override, result_unit, "prompt", context(NO_CONTEXT), capabilities(CAPABILITIES_PROMPT);
    "prompt_update" => args_prompt_update, result_prompt_update, "prompt", context(NO_CONTEXT), capabilities(CAPABILITIES_PROMPT);
    "prompt_validate_variables" => args_prompt_validate_variables, result_prompt_validate_variables, "prompt", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_version_diff" => args_prompt_version_diff, result_string, "prompt", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_version_history" => args_prompt_name, result_prompt_version_history, "prompt", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "push_assembled_chapter_video" => args_push_assembled_chapter_video, result_assembly_push, "video", context(PROJECT_CHAPTER_CONTEXT), capabilities(CAPABILITIES_VIDEO);
    "rag_enhanced_search" => args_rag_enhanced_search, result_rag_enhanced_search, "rag", context(PROJECT_CONTEXT), effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_RAG), runtime(RuntimeConfigPolicy::Full);
    "rag_enhanced_search_with_vectors" => args_rag_enhanced_search_with_vectors, result_rag_enhanced_search, "rag", context(PROJECT_CONTEXT), effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_RAG), runtime(RuntimeConfigPolicy::Full);
    "rag_ingest_all" => args_rag_ingest_all, result_rag_ingest, "rag", context(PROJECT_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_RAG), runtime(RuntimeConfigPolicy::Full);
    "rag_prepare_enhanced_search" => args_rag_enhanced_search, result_rag_prepare_enhanced_search, "rag", context(PROJECT_CONTEXT), effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_RAG), runtime(RuntimeConfigPolicy::Full);
    "rag_prepare_ingest_all" => args_rag_prepare_ingest_all, result_rag_prepare_ingest_all, "rag", context(PROJECT_CONTEXT), effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_RAG), runtime(RuntimeConfigPolicy::Full);
    "rag_upload_ingest_embeddings" => args_rag_upload_ingest_embeddings, result_rag_ingest, "rag", context(PROJECT_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_EMBEDDING_RAG), runtime(RuntimeConfigPolicy::Full);
    "read_canvas_graph" => args_read_canvas_graph, result_read_canvas_graph, "canvas", effect(READ_EFFECT), context(NO_CONTEXT);
    "read_chapter_visual_plan" => args_read_chapter_visual_plan, result_read_chapter_visual_plan, "chapter", effect(READ_EFFECT);
    "read_image" => args_read_image, result_read_image, "image", context(NO_CONTEXT), effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "read_recent_logs" => args_read_recent_logs, result_read_recent_logs, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "read_visual_plan_prompt_candidate" => args_read_visual_plan_prompt_candidate, result_visual_plan_prompt_candidate, "chapter", effect(READ_EFFECT);
    "record_optimize_agent_event" => args_record_optimize_agent_event, result_unit, "optimize", context(NO_CONTEXT);
    "refresh_token" => args_refresh_token, result_refresh_token, "auth", identity(PUBLIC_IDENTITY), secrets(REFRESH_TOKEN_SECRET_POINTERS);
    "register_user" => args_register_user, result_login_user, "auth", identity(PUBLIC_IDENTITY), secrets(PASSWORD_SECRET_POINTERS);
    "reject_import_review" => args_import_review_id, result_import_review, "import", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_IMPORT);
    "remove_image_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_IMAGE);
    "remove_project_member" => args_member_user_id, result_unit, "project", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "remove_video_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_VIDEO);
    "reorder_chapter_scenes" => args_reorder_chapter_scenes, result_chapter_scenes, "chapter", context(NO_CONTEXT), references(REFERENCES_CHAPTER_IDS);
    "reorder_galgame_lines" => args_reorder_galgame_lines, result_reorder_galgame_lines, "galgame", context(PROJECT_CONTEXT), references(REFERENCES_GALGAME_LINE_IDS);
    "request_attribute_deletion" => args_request_attribute_deletion, result_request_attribute_deletion, "profile", effect(DESTRUCTIVE_EFFECT);
    "reset_deletion_marks" => args_reset_deletion_marks, result_reset_deletion_marks, "profile", effect(DESTRUCTIVE_EFFECT);
    "reset_project_generated_data" => args_project_id, result_project_generated_data_reset, "project", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT), references(REFERENCES_PROJECT_IDS);
    "respond_agent_editor" => args_respond_agent_editor, result_agent_editor_reply, "agent";
    "restore_blueprint_version" => args_restore_blueprint_version, result_blueprint_mutation, "blueprint", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "resume_chapter_analysis_sync" => args_chapter_analysis_task, result_chapter_summary, "chapter", context(NO_CONTEXT), execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::Full), idempotency("server_workflow");
    "resume_manga_pipeline" => args_preview_resume_manga_pipeline_request, result_generation_task_handle, "manga", context(NO_CONTEXT), execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("server_workflow");
    "resume_workflow_run" => args_resume_workflow_run, result_workflow_run, "workflow", context(NO_CONTEXT), execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_WORKFLOW_RUN_IDS), runtime(RuntimeConfigPolicy::Full), idempotency("server_workflow");
    "reverse_engineer" => args_reverse_engineer, result_reverse_engineering_candidate, "workflow", context(NO_CONTEXT), execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_WORKFLOW_RUN_IDS), runtime(RuntimeConfigPolicy::Full);
    "review_agent_delegation" => args_review_agent_delegation, result_agent_delegation, "agent", idempotency("same_review");
    "revoke_project_invite" => args_invite_id, result_unit, "project", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "revoke_session" => args_revoke_session, result_unit, "auth", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "rollback_project_status" => args_forward_project_status, result_create_project, "project", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT), references(REFERENCES_PROJECT_IDS);
    "rollback_to_chapter" => args_rollback_to_chapter, result_rollback_to_chapter, "profile", effect(DESTRUCTIVE_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "run_canvas_graph" => args_run_canvas_graph, result_run_canvas_graph, "canvas", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "run_canvas_node" => args_run_canvas_node, result_run_canvas_graph, "canvas", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "run_galgame_asset_generation" => args_run_galgame_asset_generation, result_run_galgame_asset_generation, "galgame", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_GENERATION_SPEC_IDS_MEDIA_ASSET_IDS), capabilities(CAPABILITIES_PROMPT_IMAGE), runtime(RuntimeConfigPolicy::Full);
    "run_import_project_analysis" => args_project_id, result_import_analysis_summary, "import", context(PROJECT_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_IMPORT), runtime(RuntimeConfigPolicy::Full);
    "run_project_fact_feedback" => args_project_fact_feedback_task, result_project_fact_feedback, "project", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "save_blueprint" => args_save_blueprint, result_blueprint_mutation, "blueprint", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "save_canvas_graph" => args_save_canvas_graph, result_save_canvas_graph, "canvas", context(NO_CONTEXT);
    "save_canvas_text_asset" => args_save_canvas_text_asset, result_save_canvas_text_asset, "canvas", context(NO_CONTEXT);
    "save_chapter_outline" => args_save_chapter_outline, result_save_chapter_outline, "outline", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "save_chapter_visual_plan_draft" => args_save_chapter_visual_plan_draft, result_chapter_visual_plan, "chapter", references(REFERENCES_PROJECT_IDS), idempotency("operation_id");
    "save_galgame_story_graph" => args_save_galgame_story_graph, result_save_galgame_story_graph, "galgame", context(PROJECT_CONTEXT);
    "save_media_authoring_document" => args_save_authoring_document, result_authoring_document, "media_authoring", context(NO_CONTEXT);
    "save_manga_pipeline_artifact" => args_save_manga_pipeline_artifact, result_manga_pipeline_artifact, "manga", context(NO_CONTEXT);
    "save_part_outline" => args_save_part_outline, result_save_part_outline, "outline", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "select_segment_video" => args_select_segment_video, result_generated_video, "video", context(PROJECT_CHAPTER_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "set_default_embedding_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), capabilities(CAPABILITIES_EMBEDDING);
    "set_default_image_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), capabilities(CAPABILITIES_IMAGE);
    "set_default_llm_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), capabilities(CAPABILITIES_LLM);
    "set_default_video_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), capabilities(CAPABILITIES_VIDEO);
    "set_log_level" => args_set_log_level, result_unit, "system", identity(ADMIN_IDENTITY);
    "set_primary_portrait" => args_set_primary_portrait, result_unit, "profile", context(NO_CONTEXT), capabilities(CAPABILITIES_IMAGE);
    "set_profile_primary_material" => args_set_profile_primary_material, result_profile_material_group, "profile";
    "start_blueprint_generation" => args_start_blueprint_generation, result_generation_task_handle, "blueprint", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_blueprint_refinement" => args_start_blueprint_refinement, result_generation_task_handle, "blueprint", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_chapter_generation" => args_start_chapter_generation, result_generation_task_handle, "chapter", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_chapter_shooting_script_generation" => args_start_chapter_shooting_script_generation, result_generation_task_handle, "chapter", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_galgame_pipeline" => args_start_galgame_pipeline, result_find_galgame_script, "galgame", context(NO_CONTEXT), execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_GALGAME_SCRIPT_IDS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("server_workflow");
    "start_galgame_story_runtime" => args_start_galgame_story_runtime, result_galgame_story_runtime, "galgame", context(PROJECT_CONTEXT);
    "start_image_generation" => args_start_image_generation, result_generation_task_handle, "asset", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_inspiration_generation" => args_start_inspiration_generation, result_generation_task_handle, "inspiration", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_manga_pipeline" => args_preview_start_manga_pipeline_request, result_generation_task_handle, "manga", context(NO_CONTEXT), execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("server_workflow");
    "start_optimize_analysis_generation" => args_start_optimize_analysis_generation, result_generation_task_handle, "system", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_optimize_chat_generation" => args_start_optimize_chat_generation, result_generation_task_handle, "system", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_optimize_patch_generation" => args_start_optimize_patch_generation, result_generation_task_handle, "system", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_outline_generation" => args_start_outline_generation, result_generation_task_handle, "outline", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_package_lifecycle" => args_start_package_lifecycle, result_package_lifecycle, "system", identity(ADMIN_IDENTITY), effect(DESTRUCTIVE_EFFECT), idempotency("operation_id");
    "start_portrait_generation" => args_start_portrait_generation, result_generation_task_handle, "profile", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_PROMPT_LLM_IMAGE), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_video_generation" => args_start_video_generation, result_generation_task_handle, "video", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_VIDEO), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_visual_plan_prompt_generation" => args_start_visual_plan_prompt_generation, result_generation_task_handle, "chapter", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "sync_all_character_profiles" => args_sync_all_character_profiles, result_sync_all_character_profiles, "profile", execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "sync_blueprint_materials" => args_project_id, result_blueprint_mutation, "blueprint", context(PROJECT_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full), idempotency("safe");
    "sync_character_from_chapters" => args_sync_character_from_chapters, result_sync_character_from_chapters, "chapter", execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "sync_outline_materials" => args_project_id, result_outline_mutation, "outline", context(PROJECT_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full), idempotency("safe");
    "sync_scene_profiles" => args_sync_scene_profiles, result_unit, "profile", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "test_embedding_config" => args_preview_test_embedding_config_request, result_unit, "settings", context(NO_CONTEXT), effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_EMBEDDING), runtime(RuntimeConfigPolicy::Full);
    "test_image_config" => args_preview_test_image_config_request, result_unit, "settings", context(NO_CONTEXT), effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::Full);
    "test_llm_config" => args_preview_test_image_config_request, result_unit, "settings", context(NO_CONTEXT), effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "trim_video" => args_trim_video, result_generated_video, "video", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_VIDEO);
    "unbind_galgame_media" => args_unbind_galgame_media, result_unbind_galgame_media, "galgame", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "update_agent_skill_configuration" => args_update_agent_skill_configuration, result_agent_skill_profile, "agent", context(NO_CONTEXT);
    "update_assembly_push_status" => args_update_assembly_push_status, result_generated_video, "video", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "update_chapter_content" => args_update_chapter_content, result_chapter_summary, "chapter", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::Full);
    "update_chapter_outline" => args_update_chapter_outline, result_update_chapter_outline, "chapter", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::Full);
    "update_chapter_scene" => args_update_chapter_scene, result_chapter_scene, "chapter", context(NO_CONTEXT), references(REFERENCES_CHAPTER_IDS);
    "update_chapter_video_prompt" => args_update_chapter_video_prompt, result_optimize_chapter_video_prompt, "video";
    "update_chapter_video_segment_plan" => args_update_chapter_video_segment_plan, result_optimize_chapter_video_prompt, "video";
    "update_chapter_video_segment_recipe" => args_update_chapter_video_segment_recipe, result_video_segment_execution_recipes, "video", capabilities(CAPABILITIES_VIDEO);
    "update_character_profile" => args_update_character_profile, result_update_character_profile, "profile", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "update_embedding_config" => args_update_embedding_config, result_create_embedding_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_EMBEDDING);
    "update_foreshadowing" => args_update_foreshadowing, result_foreshadowing_write, "profile", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "update_galgame_choices" => args_update_galgame_choices, result_update_galgame_choices, "galgame", context(PROJECT_CONTEXT), references(REFERENCES_GALGAME_CHOICE_IDS);
    "update_galgame_line" => args_update_galgame_line, result_insert_galgame_line, "galgame", context(PROJECT_CONTEXT), references(REFERENCES_GALGAME_LINE_IDS);
    "update_galgame_scene" => args_update_galgame_scene, result_update_galgame_scene, "galgame", context(PROJECT_CONTEXT), references(REFERENCES_GALGAME_SCENE_IDS);
    "update_image" => args_update_image, result_update_image, "asset", context(NO_CONTEXT);
    "update_image_config" => args_update_image_config, result_add_image_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_IMAGE);
    "update_llm_config" => args_update_llm_config, result_create_llm_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS);
    "update_manga_prompt" => args_update_manga_prompt, result_bind_manga_reference_image, "manga", context(NO_CONTEXT);
    "update_object_profile" => args_update_object_profile, result_object_profile_mutation, "profile";
    "update_package_preferences" => args_update_package_preferences, result_package_preferences, "settings", context(NO_CONTEXT);
    "update_project" => args_update_project, result_create_project, "project", context(PROJECT_CONTEXT), references(REFERENCES_PROJECT_IDS);
    "update_project_member_role" => args_update_project_member_role, result_unit, "project", context(PROJECT_CONTEXT);
    "update_scene_profile" => args_update_scene_profile, result_scene_profile_mutation, "profile", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "update_theme" => args_update_theme, result_activate_theme, "settings", context(NO_CONTEXT);
    "update_theme_preference" => args_update_theme_preference, result_theme_preference, "settings", context(NO_CONTEXT);
    "update_user_profile" => args_update_user_profile, result_get_current_user, "auth", context(NO_CONTEXT);
    "update_user_role" => args_update_user_role, result_get_current_user, "auth", identity(ADMIN_IDENTITY);
    "update_video" => args_update_video, result_generated_video, "video", context(NO_CONTEXT);
    "update_video_config" => args_update_video_config, result_add_video_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_VIDEO);
    "update_video_segment_plan_draft_recipe" => args_update_video_segment_plan_draft_recipe, result_unit, "video", capabilities(CAPABILITIES_VIDEO);
    "update_world_setting" => args_update_world_setting, result_blueprint_mutation, "blueprint", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "upload_assembly_insert_video" => args_upload_assembly_insert_video, result_generated_video, "video", context(PROJECT_CHAPTER_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT);
    "upload_chapter_video" => args_upload_chapter_video, result_generated_video, "video", context(PROJECT_CHAPTER_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT);
    "upload_image" => args_upload_image, result_update_image, "asset", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT);
    "upload_portrait" => args_upload_portrait, result_portrait_summary, "profile", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT);
    "upload_profile_material_image" => args_upload_profile_material_image, result_profile_material_group, "profile", file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT);
    "upload_reference_media_video" => args_upload_reference_media_video, result_generated_video, "video", context(PROJECT_CHAPTER_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT);
    "upload_theme_resource" => args_theme_resource_upload, result_theme_resource, "settings", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT);
    "validate_chapter_visual_plan" => args_validate_chapter_visual_plan, result_validate_chapter_visual_plan, "chapter", effect(READ_EFFECT);
    "web_fetch" => args_web_fetch, result_web_fetch, "web-search", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "web_search" => args_web_search, result_web_search, "web-search", context(NO_CONTEXT), effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
}

pub fn list(filter: Option<&str>) -> Value {
    let needle = filter.map(str::trim).filter(|value| !value.is_empty());
    let matches = |value: &str| {
        needle.is_none_or(|needle| value.to_lowercase().contains(&needle.to_lowercase()))
    };
    let api_commands = API_COMMANDS
        .iter()
        .copied()
        .filter(|command| matches(command))
        .collect::<Vec<_>>();
    let runtime_equivalents = RUNTIME_EQUIVALENTS
        .iter()
        .filter(|(desktop, cli)| matches(desktop) || matches(cli))
        .map(|(desktop, cli)| {
            json!({
                "desktop_runtime_command": desktop,
                "cli": cli
            })
        })
        .collect::<Vec<_>>();
    json!({
        "total_api_commands": API_COMMANDS.len(),
        "matched_api_commands": api_commands.len(),
        "api_commands": api_commands,
        "runtime_equivalents": runtime_equivalents
    })
}

pub fn command_manifest(
    filter: Option<&str>,
    domain_filter: Option<&str>,
    effect_filter: Option<&str>,
) -> CommandManifest {
    let needle = filter.map(str::trim).filter(|value| !value.is_empty());
    let domain_filter = domain_filter
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let effect_filter = effect_filter
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let commands = COMMAND_SPECS
        .iter()
        .filter(|spec| {
            needle.is_none_or(|needle| spec.name.to_lowercase().contains(&needle.to_lowercase()))
                && domain_filter.is_none_or(|filter| spec.domain == filter)
                && effect_filter.is_none_or(|filter| spec.effect == filter)
        })
        .map(descriptor)
        .collect::<Vec<_>>();
    CommandManifest {
        manifest_version: COMMAND_MANIFEST_VERSION,
        total_commands: API_COMMANDS.len(),
        matched_commands: commands.len(),
        commands,
        runtime_equivalents: runtime_equivalents(),
    }
}

pub fn capabilities(
    filter: Option<&str>,
    domain_filter: Option<&str>,
    effect_filter: Option<&str>,
) -> Value {
    serde_json::to_value(command_manifest(filter, domain_filter, effect_filter))
        .expect("command manifest must serialize")
}

pub fn describe(command: &str) -> Option<Value> {
    command_descriptor(command)
        .map(|descriptor| {
            serde_json::to_value(descriptor).expect("command descriptor must serialize")
        })
        .or_else(|| {
            RUNTIME_EQUIVALENTS
                .iter()
                .find(|(desktop, _)| *desktop == command)
                .map(|(desktop, cli)| {
                    json!({
                        "name": desktop,
                        "kind": "desktop_runtime_equivalent",
                        "cli_equivalent": cli,
                    })
                })
        })
}

pub fn command_descriptor(command: &str) -> Option<CommandDescriptor> {
    command_spec(command).map(descriptor)
}

pub fn command_runtime_config_policy(command: &str) -> RuntimeConfigPolicy {
    command_spec(command)
        .map(|spec| spec.runtime_config_policy)
        .unwrap_or(RuntimeConfigPolicy::None)
}

pub fn is_destructive(command: &str) -> bool {
    command_spec(command).map_or_else(
        || {
            [
                "cancel_",
                "cleanup_",
                "decline_",
                "delete_",
                "detach_",
                "disable_",
                "reject_",
                "remove_",
                "reset_",
                "revoke_",
                "rollback_",
                "trim_",
                "unbind_",
            ]
            .iter()
            .any(|prefix| command.starts_with(prefix))
        },
        |spec| spec.effect == "destructive",
    )
}

fn command_spec(command: &str) -> Option<&'static CommandSpec> {
    API_COMMANDS
        .binary_search(&command)
        .ok()
        .map(|index| &COMMAND_SPECS[index])
}

fn descriptor(spec: &CommandSpec) -> CommandDescriptor {
    let agent_policy = default_agent_policy(spec).map(|policy| agent_command_policy(&policy));
    let (package_id, package_category) = package_policy(spec);
    CommandDescriptor {
        name: spec.name.to_string(),
        kind: "host_api".into(),
        domain: spec.domain.into(),
        summary: match spec.name {
            "read_chapter_visual_plan" => "Read the timeline outline, an image point, a shot segment, or bounded source text. source_chapter_id is the database chapter primary key, while chapter_number in the result is the user-facing chapter sequence; never substitute one for the other. Points own images and time; segments can span several points. Pin record_id and expected_revision. Does not read an editor's unsaved draft.".into(),
            "review_agent_delegation" => "Explicitly accept a child's delivered resources or request revision, recording the reason and exact inspected versions. Acceptance requires successful parent read/verification executions for every delivered resource; delegate results and child completion text are not evidence. A final reply alone never accepts a delegation.".into(),
            "read_canvas_graph" => "Read a paged graph index, or selected node IDs with incident edges and neighbor IDs. Use expected_version when continuing a read. Never creates a graph; node configs are returned only when requested.".into(),
            "edit_canvas_graph" => "Apply 1..20 typed node-config or connection edits at expected_version. Reuses graph/port/reference validation and transactional CAS. Unrelated nodes and edges keep their IDs; returns actual changed records and the saved version. Does not run the graph.".into(),
            "edit_chapter_visual_plan_draft" => "Edit a saved director timeline with typed operations: image points, shot segments, shared boundaries, proportional retiming, image bindings, scene/prompt fields, single-point removal, remove_points batches and a combined remove operation for point_ids plus segment_ids. Combined removal preserves fixed endpoints; a draft may temporarily contain zero segments, while confirmed plans still require a complete timeline. Partial segment removal must leave a valid contiguous timeline or be handled by merge operations. Inserting a point does not split a shot. End images are opt-in. Requires record_id and expected_revision; returns the updated plan and actual changes. Does not confirm the plan.".into(),
            "validate_chapter_visual_plan" => "Check a saved plan at the specified revision with draft or confirmed structure rules. Returns the first diagnostic with its path; never confirms the plan or assesses visual quality.".into(),
            "read_image" => "Read one to four persisted project images or video frames with the current vision-capable LLM. Use this to inspect composition, continuity, artifacts and whether a frame should be regenerated; this command only returns an assessment and never writes or regenerates an image.".into(),
            _ => spec.name.replace('_', " "),
        },
        auth: spec.auth.into(),
        effect: spec.effect.into(),
        execution: spec.execution.into(),
        args_schema: with_context_fields((spec.args_schema)(), spec.context_fields),
        result_schema: (spec.result_schema)(),
        secret_pointers: owned_strings(spec.secret_pointers),
        file_inputs: owned_strings(spec.file_inputs),
        file_outputs: owned_strings(spec.file_outputs),
        result_references: owned_strings(spec.result_references),
        required_capabilities: owned_strings(spec.required_capabilities),
        runtime_config_policy: spec.runtime_config_policy,
        idempotency: spec.idempotency.into(),
        invoke: format!("a4n call {} --args @input.json", spec.name),
        page_scopes: vec![spec.domain.into()],
        required_ui_context: owned_strings(spec.context_fields),
        approval_policy: if spec.effect == "destructive" {
            "user_confirmation".into()
        } else {
            "agent_allowed".into()
        },
        navigation_hint: page_navigation_hint(spec.domain),
        agent_policy,
        package_id,
        package_category,
    }
}

fn package_policy(spec: &CommandSpec) -> (PackageId, PackageCommandCategory) {
    let package_id = if spec.domain == "export" || spec.name.starts_with("export_") {
        PackageId::Export
    } else if spec.domain == "import" || spec.name.starts_with("import_") {
        PackageId::ExternalImport
    } else if spec.domain == "manga" {
        PackageId::Manga
    } else if spec.domain == "video" {
        PackageId::Video
    } else if spec.domain == "galgame" {
        PackageId::Galgame
    } else if spec.domain == "canvas" {
        PackageId::Canvas
    } else if spec.domain == "agent" {
        PackageId::Agent
    } else if spec.domain == "rag"
        || spec.required_capabilities.contains(&"embedding")
        || spec.required_capabilities.contains(&"rag")
    {
        PackageId::VectorRag
    } else if spec.domain == "prompt"
        || spec.required_capabilities.contains(&"llm")
        || spec.required_capabilities.contains(&"prompt")
        || spec.required_capabilities.contains(&"image")
    {
        PackageId::AiWorkflow
    } else if spec.domain == "profile"
        && (spec.name.contains("history")
            || spec.name.contains("snapshot")
            || spec.name.contains("state_at"))
    {
        PackageId::LineageView
    } else if spec.domain == "auth"
        || matches!(
            spec.domain,
            "settings" | "system" | "task" | "collaboration" | "logging"
        )
    {
        PackageId::Platform
    } else {
        PackageId::NovelManagement
    };

    let category = if spec.name.starts_with("cancel_")
        || spec.name.starts_with("resume_")
        || spec.name.starts_with("restore_")
        || spec.name.starts_with("rollback_")
        || spec.name.starts_with("reset_")
        || spec.name.starts_with("cleanup_")
    {
        PackageCommandCategory::Maintenance
    } else if spec.execution == "task"
        || spec.execution == "workflow"
        || spec.name.starts_with("start_")
        || spec.name.contains("generation")
        || spec.name.contains("generate")
    {
        PackageCommandCategory::Generation
    } else {
        PackageCommandCategory::Normal
    };
    (package_id, category)
}

fn default_agent_policy(spec: &CommandSpec) -> Option<AgentPolicySpec> {
    // Existing manifest auth/secret metadata remains the single visibility gate.
    // Commands that need runtime secrets or an admin identity are not model tools.
    // Read-only web/image inspection tools are the intentional exception: their
    // provider credentials and limits arrive through the per-run runtime
    // snapshot, while the command itself remains user-scoped and non-mutating.
    let is_runtime_read_tool = matches!(spec.name, "read_image" | "web_search" | "web_fetch");
    if spec.auth != "user"
        || (!is_runtime_read_tool
            && (!spec.secret_pointers.is_empty()
                || spec.runtime_config_policy != RuntimeConfigPolicy::None))
        || matches!(spec.domain, "auth" | "settings" | "system")
        || matches!(
            spec.name,
            "list_agent_skills" | "update_agent_skill_configuration"
        )
        || spec.name == "respond_agent_editor"
    {
        return None;
    }
    match spec.effect {
        "read" => Some(AGENT_READ),
        "destructive" => Some(AGENT_GUARDED_WRITE),
        "write" => Some(AgentPolicySpec {
            exposure: AgentToolExposure::Discoverable,
            risk: AgentToolRisk::Medium,
            reversibility: AgentToolReversibility::Compensatable,
            replay_policy: if spec.idempotency == "none" {
                AgentToolReplayPolicy::Never
            } else {
                AgentToolReplayPolicy::Idempotent
            },
            cost_class: AgentToolCostClass::None,
            tags: NO_CONTEXT,
        }),
        _ => None,
    }
}

fn agent_command_policy(spec: &AgentPolicySpec) -> AgentCommandPolicy {
    AgentCommandPolicy {
        exposure: spec.exposure,
        risk: spec.risk,
        reversibility: spec.reversibility,
        replay_policy: spec.replay_policy,
        cost_class: spec.cost_class,
        tags: owned_strings(spec.tags),
    }
}

fn page_navigation_hint(domain: &str) -> Option<String> {
    match domain {
        "settings" => Some("/settings".into()),
        "project" | "chapter" | "outline" | "blueprint" | "profile" | "asset" | "inspiration"
        | "manga" | "galgame" | "video" => Some("/".into()),
        _ => None,
    }
}

fn owned_strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

pub(crate) fn runtime_equivalents() -> Vec<RuntimeEquivalent> {
    RUNTIME_EQUIVALENTS
        .iter()
        .map(|(desktop, cli)| RuntimeEquivalent {
            desktop_runtime_command: (*desktop).into(),
            cli: (*cli).into(),
        })
        .collect()
}
