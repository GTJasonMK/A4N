use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const COMMAND_MANIFEST_VERSION: u32 = 2;
pub const HOST_IDENTITY: &str = "a4n-server";

pub const API_COMMANDS: &[&str] = &[
    "accept_import_review",
    "accept_project_invite",
    "activate_image_config",
    "activate_theme",
    "activate_video_config",
    "add_image_config",
    "add_profile_attribute",
    "add_video_config",
    "apply_assembly_postprocess_edit",
    "apply_implicit_update_decision",
    "apply_optimization_patches",
    "apply_source_sync_decisions",
    "assemble_chapter_videos",
    "attribute_history",
    "batch_accept_reviews",
    "bind_galgame_media",
    "bind_manga_reference_image",
    "branch_project",
    "cancel_admin_task",
    "cancel_embedding_model_download",
    "cancel_galgame_pipeline",
    "cancel_generation_task",
    "cancel_import_analysis",
    "cancel_manga_pipeline",
    "cancel_task",
    "cancel_workflow",
    "change_user_password",
    "check_character_conflict",
    "check_implicit_update",
    "check_import_dedup",
    "check_profile_sync_conflict",
    "cleanup_orphan_project_data_dirs",
    "create_derived_project",
    "create_embedding_config",
    "create_freeform_video_project",
    "create_invite_code",
    "create_llm_config",
    "create_object_profile",
    "create_optimization_analysis_session",
    "create_optimization_chat_session",
    "create_profile",
    "create_project",
    "create_scene_profile",
    "create_theme",
    "create_user",
    "decline_project_invite",
    "delete_chapter_scene",
    "delete_chapter_video_prompt",
    "delete_embedding_config",
    "delete_galgame_line",
    "delete_image",
    "delete_import_review",
    "delete_latest_chapter_outlines",
    "delete_latest_parts",
    "delete_llm_config",
    "delete_local_embedding_model",
    "delete_manga_by_chapter",
    "delete_manga_prompt",
    "delete_object_profile",
    "delete_optimize_session",
    "delete_profile",
    "delete_project",
    "delete_scene_profile",
    "delete_theme",
    "delete_video",
    "detach_derived_project",
    "detect_image_config_models",
    "detect_llm_config_models",
    "detect_video_config_models",
    "diff_blueprint_versions",
    "diff_profile_chapters",
    "disable_user",
    "discover_character_profiles",
    "download_embedding_model",
    "duplicate_project",
    "edit_chapter_shooting_script",
    "execute_attribute_deletion",
    "export_all_configs_json",
    "export_batch_data",
    "export_characters_json_data",
    "export_derived_project_data",
    "export_epub_data",
    "export_galgame_webgal",
    "export_markdown_data",
    "export_outline_json_data",
    "export_pdf_data",
    "export_project_family_data",
    "export_theme",
    "export_txt_data",
    "export_video_studio_project_data",
    "finish_chapter_generation_rag_context",
    "forward_project_status",
    "generate_chapter_analysis",
    "generate_chapter_scene",
    "generate_chapter_shooting_script",
    "generate_object_profile_image_prompt",
    "generate_portrait_prompt",
    "generate_scene_profile_image_prompt",
    "get_admin_overview",
    "get_blueprint",
    "get_canvas_graph",
    "get_canvas_node_contracts",
    "get_chapter_outlines_pacing",
    "get_chapter_shooting_script",
    "get_character_arc_status",
    "get_character_voice_fingerprint",
    "get_current_user",
    "get_embedding_model_download_info",
    "get_galgame_pipeline_status",
    "get_galgame_script",
    "get_generation_call_analytics",
    "get_generation_progress",
    "get_implicit_review",
    "get_import_analysis_status",
    "get_import_review",
    "get_inspiration_history",
    "get_llm_max_tokens_config",
    "get_log_dir",
    "get_manga_pipeline_status",
    "get_optimize_session",
    "get_primary_portrait",
    "get_profile_state_at_chapter",
    "get_project",
    "get_project_asset_dependency_graph",
    "get_project_asset_dependency_graph_node_detail",
    "get_project_import_analysis_progress",
    "get_project_status_history",
    "get_queue_stats",
    "get_snapshot",
    "get_storage_paths",
    "get_task_history",
    "get_theme_css_variables",
    "get_token_summary",
    "get_usage_trends",
    "get_user",
    "get_user_usage",
    "get_workflow_progress_events",
    "get_workflow_run_detail",
    "health_check",
    "import_all_configs",
    "import_canvas_media",
    "import_chapter",
    "import_galgame_audio",
    "import_project_family_upload",
    "import_project_full_upload",
    "import_theme",
    "import_uploaded_file",
    "import_video_studio_project_upload",
    "insert_galgame_line",
    "invite_project_member",
    "leave_project",
    "list_active_workflows",
    "list_all_projects",
    "list_all_users",
    "list_behaviors",
    "list_blueprint_versions",
    "list_chapter_outlines",
    "list_chapter_scenes",
    "list_chapter_video_prompts",
    "list_chapters",
    "list_deletion_marks",
    "list_embedding_backends",
    "list_embedding_config",
    "list_image_configs",
    "list_images_by_chapter",
    "list_images_by_project",
    "list_import_queue",
    "list_invite_codes",
    "list_llm_config",
    "list_local_embedding_models",
    "list_manga_panels",
    "list_my_project_invites",
    "list_object_profiles",
    "list_optimize_patch_sets",
    "list_optimize_sessions",
    "list_optimize_turns",
    "list_orphan_project_data_dirs",
    "list_parts",
    "list_pending_tasks",
    "list_portraits_by_project",
    "list_profile_materials",
    "list_profiles",
    "list_project_invites",
    "list_project_members",
    "list_projects",
    "list_scene_profiles",
    "list_sessions",
    "list_snapshots",
    "list_source_snapshot_chapters",
    "list_themes",
    "list_user_themes",
    "list_video_configs",
    "list_video_reference_images",
    "list_videos_by_chapter",
    "list_workflow_runs",
    "load_cached_assembled_chapter_video_preview",
    "load_video_playback_data",
    "load_video_segment_plan_draft",
    "login_user",
    "logout_user",
    "modify_import_review",
    "modify_profile_attribute",
    "oauth2_authorize_url",
    "oauth2_callback",
    "optimize_chapter_video_prompt",
    "optimize_manga_prompt",
    "plan_galgame_assets",
    "poll_video_job",
    "prepare_chapter_generation_rag_embedding",
    "prepare_optimization_analysis_session",
    "preview_apply_beat_mapping_request",
    "preview_batch_generate_part_outlines_request",
    "preview_batch_images_request",
    "preview_canvas_node_request",
    "preview_chapter_analysis_request",
    "preview_chapter_prompt",
    "preview_chapter_scene_request",
    "preview_chapter_video_request",
    "preview_check_character_conflict_request",
    "preview_check_implicit_update_request",
    "preview_continue_part_outlines_request",
    "preview_create_optimization_analysis_session_request",
    "preview_discover_character_profiles_request",
    "preview_galgame_pipeline_request",
    "preview_generate_age_variant_request",
    "preview_generate_blueprint_request",
    "preview_generate_chapter_outlines_by_count_request",
    "preview_generate_missing_portraits_request",
    "preview_generate_part_chapter_outlines_request",
    "preview_generate_parts_request",
    "preview_generate_portrait_prompt_request",
    "preview_generate_portrait_request",
    "preview_import_project_analysis_request",
    "preview_inspiration_request",
    "preview_object_profile_image_request",
    "preview_page_image_request",
    "preview_prepare_optimization_analysis_session_request",
    "preview_propose_optimization_patches_request",
    "preview_reference_media_video_request",
    "preview_refine_blueprint_request",
    "preview_regenerate_chapter_outline_request",
    "preview_regenerate_last_part_request",
    "preview_regenerate_part_outline_request",
    "preview_regenerate_parts_request",
    "preview_regenerate_portrait_request",
    "preview_resume_chapter_analysis_sync_request",
    "preview_resume_manga_pipeline_request",
    "preview_reverse_engineer_blueprint_request",
    "preview_reverse_engineer_outline_request",
    "preview_reverse_engineer_part_outline_request",
    "preview_rollback",
    "preview_scene_image_request",
    "preview_send_optimization_message_request",
    "preview_source_sync",
    "preview_start_manga_pipeline_request",
    "preview_sync_all_character_profiles_request",
    "preview_sync_character_from_chapters_request",
    "preview_test_embedding_config_request",
    "preview_test_image_config_request",
    "preview_test_llm_config_request",
    "preview_video_provider_request_by_video_id",
    "prompt_dependency_graph",
    "prompt_get_dependencies",
    "prompt_get_reverse_dependencies",
    "prompt_list",
    "prompt_list_categories",
    "prompt_list_project_overrides",
    "prompt_load_defaults",
    "prompt_remove_project_override",
    "prompt_render",
    "prompt_render_ab",
    "prompt_reset",
    "prompt_set_project_override",
    "prompt_update",
    "prompt_validate_variables",
    "prompt_version_diff",
    "prompt_version_history",
    "push_assembled_chapter_video",
    "rag_enhanced_search",
    "rag_enhanced_search_with_vectors",
    "rag_ingest_all",
    "rag_prepare_enhanced_search",
    "rag_prepare_ingest_all",
    "rag_upload_ingest_embeddings",
    "read_recent_logs",
    "record_optimize_agent_event",
    "refresh_token",
    "register_user",
    "reject_import_review",
    "reject_pending_optimize_patch_sets",
    "remove_image_config",
    "remove_project_member",
    "remove_video_config",
    "reorder_chapter_scenes",
    "reorder_galgame_lines",
    "request_attribute_deletion",
    "reset_deletion_marks",
    "reset_project_generated_data",
    "restore_blueprint_version",
    "resume_chapter_analysis_sync",
    "resume_manga_pipeline",
    "resume_workflow_run",
    "reverse_engineer_blueprint",
    "reverse_engineer_blueprint_and_apply",
    "reverse_engineer_outline",
    "reverse_engineer_outline_and_apply",
    "reverse_engineer_part_outline",
    "reverse_engineer_part_outline_and_apply",
    "revoke_project_invite",
    "rollback_project_status",
    "rollback_to_chapter",
    "run_canvas_graph",
    "run_canvas_node",
    "run_galgame_asset_generation",
    "run_import_project_analysis",
    "save_canvas_graph",
    "save_canvas_text_asset",
    "score_image",
    "select_segment_video",
    "set_default_embedding_config",
    "set_default_llm_config",
    "set_log_level",
    "set_primary_portrait",
    "set_profile_primary_material",
    "start_blueprint_generation",
    "start_blueprint_refinement",
    "start_chapter_generation",
    "start_galgame_pipeline",
    "start_image_generation",
    "start_import_analysis",
    "start_inspiration_generation",
    "start_manga_pipeline",
    "start_optimize_analysis_generation",
    "start_optimize_chat_generation",
    "start_optimize_patch_generation",
    "start_outline_generation",
    "start_portrait_generation",
    "start_video_generation",
    "sync_all_character_profiles",
    "sync_character_from_chapters",
    "sync_scene_profiles",
    "test_embedding_config",
    "test_image_config",
    "test_llm_config",
    "test_video_config",
    "toggle_image_favorite",
    "toggle_video_favorite",
    "trim_video",
    "unbind_galgame_media",
    "update_assembly_push_status",
    "update_chapter_content",
    "update_chapter_outline",
    "update_chapter_scene",
    "update_chapter_video_prompt",
    "update_chapter_video_segment_recipe",
    "update_embedding_config",
    "update_galgame_choices",
    "update_galgame_line",
    "update_galgame_scene",
    "update_image",
    "update_image_config",
    "update_import_status",
    "update_llm_config",
    "update_llm_max_tokens_config",
    "update_manga_prompt",
    "update_object_profile",
    "update_project",
    "update_project_member_role",
    "update_scene_profile",
    "update_theme",
    "update_user_profile",
    "update_user_role",
    "update_video",
    "update_video_config",
    "update_video_segment_plan_draft_recipe",
    "update_world_setting",
    "upload_assembly_insert_video",
    "upload_chapter_video",
    "upload_image",
    "upload_portrait",
    "upload_profile_material_image",
    "upload_reference_media_video",
];

pub const RUNTIME_EQUIVALENTS: &[(&str, &str)] = &[
    ("get_generation_task_snapshot", "task snapshot"),
    ("get_generation_task_summary", "task get"),
    ("list_active_generation_tasks", "task active"),
    ("list_generation_task_history", "task history"),
    (
        "local_cancel_embedding_model_download",
        "call cancel_embedding_model_download",
    ),
    (
        "local_delete_embedding_model",
        "call delete_local_embedding_model",
    ),
    (
        "local_download_embedding_model",
        "call download_embedding_model",
    ),
    (
        "local_embed_texts",
        "call rag_enhanced_search / call rag_ingest_all（由 Server 执行 embedding）",
    ),
    (
        "local_get_embedding_model_download_info",
        "call get_embedding_model_download_info",
    ),
    (
        "local_list_embedding_models",
        "call list_local_embedding_models",
    ),
    (
        "local_test_embedding_config",
        "call test_embedding_config（使用 Server 配置）",
    ),
    ("subscribe_generation_task", "task watch"),
    ("unsubscribe_generation_task", "结束 task watch（Ctrl-C）"),
];

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

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct CommandManifest {
    pub manifest_version: u32,
    pub total_commands: usize,
    pub matched_commands: usize,
    pub commands: Vec<CommandDescriptor>,
    pub runtime_equivalents: Vec<RuntimeEquivalent>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct CommandDescriptor {
    pub name: String,
    pub kind: String,
    pub domain: String,
    pub summary: String,
    pub auth: String,
    pub effect: String,
    pub execution: String,
    pub args_schema: Value,
    pub result_schema: Value,
    pub secret_pointers: Vec<String>,
    pub file_inputs: Vec<String>,
    pub file_outputs: Vec<String>,
    pub result_references: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub idempotency: String,
    pub invoke: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct RuntimeEquivalent {
    pub desktop_runtime_command: String,
    pub cli: String,
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
    let commands = API_COMMANDS
        .iter()
        .copied()
        .filter(|command| {
            needle.is_none_or(|needle| command.to_lowercase().contains(&needle.to_lowercase()))
                && domain_filter.is_none_or(|filter| domain(command) == filter)
                && effect_filter.is_none_or(|filter| effect(command) == filter)
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
    API_COMMANDS
        .binary_search(&command)
        .ok()
        .map(|index| descriptor(API_COMMANDS[index]))
}

pub fn command_idempotency(command: &str) -> Option<&'static str> {
    API_COMMANDS
        .binary_search(&command)
        .is_ok()
        .then(|| idempotency(command))
}

#[derive(Clone, Copy)]
struct DescriptorOverride {
    domain: Option<&'static str>,
    effect: Option<&'static str>,
    execution: Option<&'static str>,
    context_fields: Option<&'static [&'static str]>,
    secret_pointers: Option<&'static [&'static str]>,
    idempotency: Option<&'static str>,
}

const DEFAULT_OVERRIDE: DescriptorOverride = DescriptorOverride {
    domain: None,
    effect: None,
    execution: None,
    context_fields: None,
    secret_pointers: None,
    idempotency: None,
};

const NO_CONTEXT: &[&str] = &[];
const USER_CONTEXT: &[&str] = &["userId"];
const PROJECT_CONTEXT: &[&str] = &["projectId"];
const CHAPTER_CONTEXT: &[&str] = &["chapterId"];
const PROJECT_CHAPTER_CONTEXT: &[&str] = &["projectId", "chapterId"];
const CALLER_CONTEXT: &[&str] = &["callerUserId"];
const LOGIN_SECRET_POINTERS: &[&str] = &["/input/password"];
const CHANGE_PASSWORD_SECRET_POINTERS: &[&str] = &["/input/old_password", "/input/new_password"];
const PROVIDER_SECRET_POINTERS: &[&str] = &["/input/api_key_encrypted", "/input/api_key"];
const RUNTIME_SNAPSHOT_SECRET_POINTERS: &[&str] = &[
    "/runtime_config_snapshot/system_llm_api_key_encrypted",
    "/runtime_config_snapshot/system_image_api_key_encrypted",
    "/runtime_config_snapshot/system_video_api_key_encrypted",
];

fn descriptor_override(command: &str) -> DescriptorOverride {
    match command {
        "login_user" | "register_user" => DescriptorOverride {
            domain: Some("auth"),
            context_fields: Some(NO_CONTEXT),
            secret_pointers: Some(LOGIN_SECRET_POINTERS),
            ..DEFAULT_OVERRIDE
        },
        "refresh_token" => DescriptorOverride {
            domain: Some("auth"),
            context_fields: Some(NO_CONTEXT),
            secret_pointers: Some(&["/input/refresh_token"]),
            ..DEFAULT_OVERRIDE
        },
        "oauth2_callback" => DescriptorOverride {
            domain: Some("auth"),
            context_fields: Some(NO_CONTEXT),
            secret_pointers: Some(&["/input/code"]),
            ..DEFAULT_OVERRIDE
        },
        "oauth2_authorize_url" | "get_current_user" | "logout_user" => DescriptorOverride {
            domain: Some("auth"),
            context_fields: Some(NO_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "get_user" | "list_sessions" | "update_user_profile" => DescriptorOverride {
            domain: Some("auth"),
            context_fields: Some(USER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "change_user_password" => DescriptorOverride {
            domain: Some("auth"),
            context_fields: Some(USER_CONTEXT),
            secret_pointers: Some(CHANGE_PASSWORD_SECRET_POINTERS),
            ..DEFAULT_OVERRIDE
        },
        "create_user" => DescriptorOverride {
            domain: Some("auth"),
            context_fields: Some(CALLER_CONTEXT),
            secret_pointers: Some(LOGIN_SECRET_POINTERS),
            ..DEFAULT_OVERRIDE
        },
        "get_admin_overview"
        | "get_usage_trends"
        | "health_check"
        | "list_all_projects"
        | "list_pending_tasks"
        | "list_orphan_project_data_dirs" => DescriptorOverride {
            context_fields: Some(CALLER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "list_all_users" | "list_invite_codes" | "create_invite_code" => DescriptorOverride {
            context_fields: Some(CALLER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "get_user_usage" | "get_token_summary" | "disable_user" | "update_user_role" => {
            DescriptorOverride {
                context_fields: Some(CALLER_CONTEXT),
                ..DEFAULT_OVERRIDE
            }
        }
        "list_projects"
        | "create_project"
        | "create_derived_project"
        | "create_freeform_video_project" => DescriptorOverride {
            domain: Some("project"),
            context_fields: Some(NO_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "get_project"
        | "update_project"
        | "delete_project"
        | "duplicate_project"
        | "reset_project_generated_data"
        | "rollback_project_status" => DescriptorOverride {
            domain: Some("project"),
            context_fields: Some(PROJECT_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "detach_derived_project" | "preview_source_sync" | "apply_source_sync_decisions" => {
            DescriptorOverride {
                domain: Some("source-sync"),
                context_fields: Some(NO_CONTEXT),
                ..DEFAULT_OVERRIDE
            }
        }
        "list_chapters"
        | "list_chapter_scenes"
        | "list_chapter_video_prompts"
        | "list_images_by_project"
        | "list_portraits_by_project" => DescriptorOverride {
            context_fields: Some(PROJECT_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "update_chapter_content"
        | "update_chapter_outline"
        | "delete_chapter_scene"
        | "generate_chapter_analysis"
        | "generate_chapter_scene"
        | "generate_chapter_shooting_script"
        | "get_chapter_shooting_script"
        | "list_images_by_chapter"
        | "list_videos_by_chapter"
        | "rollback_to_chapter" => DescriptorOverride {
            context_fields: Some(CHAPTER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "import_chapter"
        | "start_chapter_generation"
        | "start_blueprint_generation"
        | "start_blueprint_refinement"
        | "start_image_generation"
        | "start_inspiration_generation"
        | "start_optimize_analysis_generation"
        | "start_optimize_chat_generation"
        | "start_optimize_patch_generation"
        | "start_outline_generation"
        | "start_portrait_generation"
        | "start_video_generation" => DescriptorOverride {
            execution: Some("task"),
            context_fields: Some(NO_CONTEXT),
            secret_pointers: Some(RUNTIME_SNAPSHOT_SECRET_POINTERS),
            idempotency: Some("client_request_id"),
            ..DEFAULT_OVERRIDE
        },
        "cancel_generation_task"
        | "cancel_task"
        | "get_generation_progress"
        | "get_task_history" => DescriptorOverride {
            domain: Some("task"),
            context_fields: Some(NO_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "list_workflow_runs" | "list_active_workflows" => DescriptorOverride {
            domain: Some("workflow"),
            execution: Some("workflow"),
            context_fields: Some(PROJECT_CHAPTER_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "get_workflow_progress_events" | "get_workflow_run_detail" => DescriptorOverride {
            domain: Some("workflow"),
            effect: Some("read"),
            execution: Some("workflow"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "cancel_workflow" | "resume_workflow_run" => DescriptorOverride {
            domain: Some("workflow"),
            execution: Some("workflow"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("server_workflow"),
            ..DEFAULT_OVERRIDE
        },
        "start_galgame_pipeline"
        | "resume_chapter_analysis_sync"
        | "resume_manga_pipeline"
        | "start_manga_pipeline" => DescriptorOverride {
            execution: Some("workflow"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("server_workflow"),
            ..DEFAULT_OVERRIDE
        },
        "get_galgame_pipeline_status"
        | "get_import_analysis_status"
        | "get_manga_pipeline_status" => DescriptorOverride {
            effect: Some("read"),
            execution: Some("workflow"),
            context_fields: Some(PROJECT_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "create_embedding_config"
        | "update_embedding_config"
        | "create_llm_config"
        | "update_llm_config"
        | "add_image_config"
        | "update_image_config"
        | "add_video_config"
        | "update_video_config" => DescriptorOverride {
            domain: Some("settings"),
            context_fields: Some(NO_CONTEXT),
            secret_pointers: Some(PROVIDER_SECRET_POINTERS),
            ..DEFAULT_OVERRIDE
        },
        "detect_image_config_models"
        | "detect_llm_config_models"
        | "detect_video_config_models" => DescriptorOverride {
            domain: Some("settings"),
            effect: Some("read"),
            context_fields: Some(NO_CONTEXT),
            secret_pointers: Some(PROVIDER_SECRET_POINTERS),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "test_embedding_config"
        | "test_image_config"
        | "test_llm_config"
        | "test_video_config"
        | "preview_test_embedding_config_request"
        | "preview_test_image_config_request"
        | "preview_test_llm_config_request" => DescriptorOverride {
            domain: Some("settings"),
            effect: Some("read"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "list_embedding_config"
        | "list_image_configs"
        | "list_llm_config"
        | "list_video_configs" => DescriptorOverride {
            domain: Some("settings"),
            effect: Some("read"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        command if command.starts_with("export_") => DescriptorOverride {
            domain: Some("export"),
            effect: Some("read"),
            execution: Some("download"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "import_all_configs"
        | "import_project_family_upload"
        | "import_project_full_upload"
        | "import_uploaded_file"
        | "import_video_studio_project_upload" => DescriptorOverride {
            domain: Some("import"),
            context_fields: Some(NO_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "get_canvas_graph"
        | "get_canvas_node_contracts"
        | "save_canvas_graph"
        | "save_canvas_text_asset" => DescriptorOverride {
            domain: Some("canvas"),
            context_fields: Some(PROJECT_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "import_canvas_media" | "run_canvas_graph" | "run_canvas_node" => DescriptorOverride {
            domain: Some("canvas"),
            context_fields: Some(NO_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        _ => DEFAULT_OVERRIDE,
    }
}

fn descriptor(command: &str) -> CommandDescriptor {
    CommandDescriptor {
        name: command.to_string(),
        kind: "host_api".into(),
        domain: domain(command).into(),
        summary: command.replace('_', " "),
        auth: auth(command).into(),
        effect: effect(command).into(),
        execution: execution(command).into(),
        args_schema: args_schema(command),
        result_schema: result_schema(command),
        secret_pointers: secret_pointers(command),
        file_inputs: file_inputs(command),
        file_outputs: file_outputs(command),
        result_references: result_references(command),
        required_capabilities: required_capabilities(command)
            .into_iter()
            .map(str::to_string)
            .collect(),
        idempotency: idempotency(command).into(),
        invoke: format!("a4n call {command} --args @input.json"),
    }
}

fn args_schema(command: &str) -> Value {
    let schema = match command {
        "login_user" => typed_input_schema::<crate::auth::LoginInput>(),
        "register_user" => typed_input_schema::<crate::auth::RegisterInput>(),
        "create_user" => typed_input_schema::<crate::admin::CreateUserInput>(),
        "refresh_token" => typed_input_schema::<crate::auth::RefreshTokenInput>(),
        "change_user_password" => typed_input_schema::<crate::auth::ChangePasswordInput>(),
        "update_user_profile" => typed_input_schema::<crate::auth::UpdateProfileInput>(),
        "create_project" => typed_input_schema::<crate::project::CreateProjectInput>(),
        "create_derived_project" => {
            typed_input_schema::<crate::project::CreateDerivedProjectInput>()
        }
        "create_freeform_video_project" => {
            typed_input_schema::<crate::project::CreateFreeformVideoProjectInput>()
        }
        "update_project" => typed_input_schema::<crate::project::UpdateProjectInput>(),
        "start_blueprint_generation" => {
            typed_input_schema::<crate::blueprint::StartBlueprintGenerationInput>()
        }
        "start_blueprint_refinement" => {
            typed_input_schema::<crate::blueprint::StartBlueprintRefinementInput>()
        }
        "start_chapter_generation" => {
            typed_input_schema::<crate::chapter::StartChapterGenerationInput>()
        }
        "start_image_generation" => {
            typed_input_schema::<crate::image_generation::StartImageGenerationInput>()
        }
        "start_inspiration_generation" => {
            typed_input_schema::<crate::inspiration::StartInspirationGenerationInput>()
        }
        "start_outline_generation" => {
            typed_input_schema::<crate::outline::StartOutlineGenerationInput>()
        }
        "start_portrait_generation" => {
            typed_input_schema::<crate::portrait::StartPortraitGenerationInput>()
        }
        "start_video_generation" => typed_input_schema::<crate::video::StartVideoGenerationInput>(),
        "start_galgame_pipeline" => {
            typed_input_schema::<crate::galgame::StartGalgamePipelineInput>()
        }
        "create_embedding_config" => typed_input_schema::<crate::config::CreateEmbeddingInput>(),
        "update_embedding_config" => typed_input_schema::<crate::config::UpdateEmbeddingInput>(),
        "create_llm_config" => typed_input_schema::<crate::config::CreateLlmInput>(),
        "update_llm_config" => typed_input_schema::<crate::config::UpdateLlmInput>(),
        "add_image_config" => typed_input_schema::<crate::config::ImageConfigInput>(),
        "update_image_config" => typed_input_schema::<crate::config::UpdateImageConfigInput>(),
        "add_video_config" => typed_input_schema::<crate::config::VideoConfigInput>(),
        "update_video_config" => typed_input_schema::<crate::config::UpdateVideoConfigInput>(),
        "detect_image_config_models"
        | "detect_llm_config_models"
        | "detect_video_config_models" => {
            typed_input_schema::<crate::config::ModelDiscoveryInput>()
        }
        "import_canvas_media" => typed_input_schema::<crate::canvas::ImportCanvasMediaInput>(),
        "import_project_full_upload" => {
            typed_input_schema::<crate::project_transfer::ImportProjectFullUploadInput>()
        }
        "import_project_family_upload" => {
            typed_input_schema::<crate::project_transfer::ImportProjectFamilyUploadInput>()
        }
        "import_video_studio_project_upload" => {
            typed_input_schema::<crate::project_transfer::ImportVideoStudioProjectUploadInput>()
        }
        "export_project_family_data" => {
            typed_input_schema::<crate::project_transfer::ExportProjectFamilyDataInput>()
        }
        "export_derived_project_data" => {
            typed_input_schema::<crate::project_transfer::ExportDerivedProjectDataInput>()
        }
        "export_video_studio_project_data" => {
            typed_input_schema::<crate::project_transfer::ExportVideoStudioProjectDataInput>()
        }
        "select_segment_video" => typed_input_schema::<crate::video::SelectSegmentVideoInput>(),
        "push_assembled_chapter_video" => {
            typed_input_schema::<crate::video::PushAssembledChapterVideoInput>()
        }
        "apply_assembly_postprocess_edit" => {
            typed_input_schema::<crate::video::ApplyAssemblyPostprocessEditInput>()
        }
        "upload_chapter_video" => typed_input_schema::<crate::video::UploadChapterVideoInput>(),
        "upload_reference_media_video" => {
            typed_input_schema::<crate::video::UploadReferenceMediaVideoInput>()
        }
        "upload_assembly_insert_video" => {
            typed_input_schema::<crate::video::UploadAssemblyInsertVideoInput>()
        }
        "get_project"
        | "delete_project"
        | "duplicate_project"
        | "reset_project_generated_data"
        | "rollback_project_status" => json!({
            "type": "object",
            "properties": { "projectId": { "type": "integer", "minimum": 1 } },
            "required": ["projectId"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        "list_chapters" | "list_chapter_scenes" | "list_chapter_video_prompts" => json!({
            "type": "object",
            "properties": { "projectId": { "type": "integer", "minimum": 1 } },
            "required": ["projectId"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        _ => json!({
            "type": "object",
            "additionalProperties": true,
            "x-a4n-schema-quality": "opaque",
        }),
    };
    with_context_fields(schema, context_fields(command))
}

fn result_schema(command: &str) -> Value {
    match command {
        "login_user" | "register_user" => typed_schema::<crate::auth::AuthResult>(),
        "refresh_token" => typed_schema::<crate::auth::TokenPairSummary>(),
        "get_current_user" | "update_user_profile" => typed_schema::<crate::auth::UserSummary>(),
        "list_projects" => typed_schema::<crate::project::PaginatedProjects>(),
        "get_project" | "create_project" | "update_project" => {
            typed_schema::<crate::project::ProjectSummary>()
        }
        "start_blueprint_generation"
        | "start_blueprint_refinement"
        | "start_chapter_generation"
        | "start_image_generation"
        | "start_inspiration_generation"
        | "start_optimize_analysis_generation"
        | "start_optimize_chat_generation"
        | "start_optimize_patch_generation"
        | "start_outline_generation"
        | "start_portrait_generation"
        | "start_video_generation" => {
            typed_schema::<crate::generation_tasks::GenerationTaskHandle>()
        }
        "list_embedding_config" => typed_schema::<Vec<crate::config::EmbeddingConfigSummary>>(),
        "list_image_configs" => typed_schema::<Vec<crate::config::ImageConfigSummary>>(),
        "list_llm_config" => typed_schema::<Vec<crate::config::LlmConfigSummary>>(),
        "list_video_configs" => typed_schema::<Vec<crate::config::VideoConfigSummary>>(),
        "create_embedding_config" | "update_embedding_config" => {
            typed_schema::<crate::config::EmbeddingConfigSummary>()
        }
        "create_llm_config" | "update_llm_config" => {
            typed_schema::<crate::config::LlmConfigSummary>()
        }
        "add_image_config" | "update_image_config" => {
            typed_schema::<crate::config::ImageConfigSummary>()
        }
        "add_video_config" | "update_video_config" => {
            typed_schema::<crate::config::VideoConfigSummary>()
        }
        "get_galgame_pipeline_status" => {
            typed_schema::<crate::galgame::GalgamePipelineStatusSummary>()
        }
        "import_canvas_media" => typed_schema::<crate::canvas::ImportedCanvasMedia>(),
        "run_canvas_graph" => typed_schema::<crate::canvas::CanvasRunSummary>(),
        _ => json!({ "x-a4n-schema-quality": "opaque" }),
    }
}

fn typed_input_schema<T: JsonSchema>() -> Value {
    json!({
        "type": "object",
        "properties": { "input": raw_schema::<T>() },
        "required": ["input"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed-input",
    })
}

fn typed_schema<T: JsonSchema>() -> Value {
    let mut schema = raw_schema::<T>();
    schema
        .as_object_mut()
        .expect("schemars root schema must be an object")
        .insert("x-a4n-schema-quality".into(), Value::String("typed".into()));
    schema
}

fn raw_schema<T: JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T)).expect("JSON schema must serialize")
}

fn with_context_fields(mut schema: Value, fields: Vec<&'static str>) -> Value {
    schema
        .as_object_mut()
        .expect("command args schema must be an object")
        .insert(
            "x-a4n-context-fields".into(),
            serde_json::to_value(fields).expect("context fields must serialize"),
        );
    schema
}

fn runtime_equivalents() -> Vec<RuntimeEquivalent> {
    RUNTIME_EQUIVALENTS
        .iter()
        .map(|(desktop, cli)| RuntimeEquivalent {
            desktop_runtime_command: (*desktop).into(),
            cli: (*cli).into(),
        })
        .collect()
}

fn context_fields(command: &str) -> Vec<&'static str> {
    if let Some(fields) = descriptor_override(command).context_fields {
        return fields.to_vec();
    }
    let mut fields = Vec::new();
    if auth(command) != "public" {
        fields.push("userId");
    }
    if command.contains("project") || command.contains("canvas") {
        fields.push("projectId");
    }
    if command.contains("chapter") || command.contains("scene") {
        fields.push("chapterId");
    }
    if command.contains("snapshot") || command.contains("source_sync") {
        fields.push("snapshotId");
    }
    fields
}

fn secret_pointers(command: &str) -> Vec<String> {
    let pointers = descriptor_override(command)
        .secret_pointers
        .unwrap_or_default();
    pointers.iter().map(|value| (*value).into()).collect()
}

fn file_inputs(command: &str) -> Vec<String> {
    if command.contains("import") || command.contains("upload") {
        vec!["使用 --set-base64/--set-text 将本地文件写入实际输入字段".into()]
    } else {
        Vec::new()
    }
}

fn file_outputs(command: &str) -> Vec<String> {
    if command.starts_with("export_") {
        vec!["使用 --save-base64/--save-text 原子保存响应字段".into()]
    } else {
        Vec::new()
    }
}

fn result_references(command: &str) -> Vec<String> {
    let mut references = Vec::new();
    match domain(command) {
        "project" => references.push("project_ids".into()),
        "chapter" => references.push("chapter_ids".into()),
        "workflow" => references.push("workflow_run_ids".into()),
        _ => {}
    }
    if execution(command) == "task" {
        references.push("task_ids".into());
    }
    if execution(command) == "download" {
        references.push("artifacts".into());
    }
    references
}

fn auth(command: &str) -> &'static str {
    match command {
        "login_user"
        | "register_user"
        | "refresh_token"
        | "oauth2_authorize_url"
        | "oauth2_callback" => "public",
        command
            if command.starts_with("get_admin_")
                || command.starts_with("list_all_")
                || command.starts_with("create_invite_code")
                || command.starts_with("create_user")
                || command.starts_with("disable_user")
                || command.starts_with("update_user_role")
                || command.starts_with("cancel_admin_") =>
        {
            "admin"
        }
        _ => "user",
    }
}

fn effect(command: &str) -> &'static str {
    if let Some(effect) = descriptor_override(command).effect {
        return effect;
    }
    if is_destructive(command) {
        return "destructive";
    }
    if [
        "attribute_history",
        "check_",
        "diff_",
        "get_",
        "health_",
        "list_",
        "load_",
        "poll_",
        "preview_",
        "prompt_get_",
        "prompt_list",
        "prompt_render",
        "prompt_validate_",
        "read_",
    ]
    .iter()
    .any(|prefix| command.starts_with(prefix))
    {
        "read"
    } else {
        "write"
    }
}

fn execution(command: &str) -> &'static str {
    if let Some(execution) = descriptor_override(command).execution {
        return execution;
    }
    if command.contains("workflow") || command.contains("pipeline") {
        "workflow"
    } else if command.starts_with("start_")
        || command.starts_with("generate_")
        || command.starts_with("run_")
        || command.starts_with("sync_")
        || command.starts_with("download_embedding_model")
    {
        "task"
    } else if command.starts_with("export_") {
        "download"
    } else {
        "immediate"
    }
}

fn domain(command: &str) -> &'static str {
    if let Some(domain) = descriptor_override(command).domain {
        return domain;
    }
    for (needle, domain) in [
        ("galgame", "galgame"),
        ("manga", "manga"),
        ("canvas", "canvas"),
        ("video", "video"),
        ("source_sync", "source-sync"),
        ("rag_", "rag"),
        ("embedding", "rag"),
        ("prompt_", "prompt"),
        ("import", "import"),
        ("export", "export"),
        ("workflow", "workflow"),
        ("task", "task"),
        ("chapter", "chapter"),
        ("outline", "outline"),
        ("part", "outline"),
        ("blueprint", "blueprint"),
        ("inspiration", "inspiration"),
        ("portrait", "profile"),
        ("profile", "profile"),
        ("character", "profile"),
        ("object", "profile"),
        ("scene", "profile"),
        ("image", "asset"),
        ("project", "project"),
        ("theme", "settings"),
        ("config", "settings"),
        ("user", "auth"),
        ("session", "auth"),
        ("oauth", "auth"),
        ("invite", "collaboration"),
        ("member", "collaboration"),
    ] {
        if command.contains(needle) {
            return domain;
        }
    }
    "system"
}

fn required_capabilities(command: &str) -> Vec<&'static str> {
    let mut capabilities = Vec::new();
    for (needle, capability) in [
        ("prompt", "prompt"),
        ("embedding", "embedding"),
        ("rag_", "rag"),
        ("image", "image"),
        ("portrait", "image"),
        ("video", "video"),
        ("import", "import"),
        ("export", "export"),
    ] {
        if command.contains(needle) && !capabilities.contains(&capability) {
            capabilities.push(capability);
        }
    }
    if execution(command) == "task"
        && !capabilities.iter().any(|capability| {
            matches!(
                *capability,
                "image" | "video" | "embedding" | "rag" | "import" | "export"
            )
        })
    {
        capabilities.push("llm");
    }
    capabilities
}

fn idempotency(command: &str) -> &'static str {
    if let Some(idempotency) = descriptor_override(command).idempotency {
        return idempotency;
    }
    if effect(command) == "read" {
        "safe"
    } else if command.starts_with("start_") {
        "client_request_id"
    } else if command.contains("workflow") || command.contains("pipeline") {
        "server_workflow"
    } else {
        "none"
    }
}

pub fn is_destructive(command: &str) -> bool {
    const PREFIXES: &[&str] = &[
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
    ];
    PREFIXES.iter().any(|prefix| command.starts_with(prefix))
        || DESTRUCTIVE_EXCEPTIONS.binary_search(&command).is_ok()
}

const DESTRUCTIVE_EXCEPTIONS: &[&str] = &[
    "apply_assembly_postprocess_edit",
    "apply_optimization_patches",
    "apply_source_sync_decisions",
    "execute_attribute_deletion",
    "leave_project",
    "prompt_remove_project_override",
    "prompt_reset",
    "request_attribute_deletion",
    "restore_blueprint_version",
    "reverse_engineer_blueprint_and_apply",
    "reverse_engineer_outline_and_apply",
    "reverse_engineer_part_outline_and_apply",
    "select_segment_video",
    "update_assembly_push_status",
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        API_COMMANDS, DESTRUCTIVE_EXCEPTIONS, RUNTIME_EQUIVALENTS, command_descriptor,
        is_destructive,
    };

    #[test]
    fn catalogs_are_sorted_and_unique() {
        assert!(API_COMMANDS.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            API_COMMANDS.len(),
            API_COMMANDS.iter().copied().collect::<HashSet<_>>().len()
        );
        assert_eq!(
            RUNTIME_EQUIVALENTS.len(),
            RUNTIME_EQUIVALENTS
                .iter()
                .map(|(command, _)| *command)
                .collect::<HashSet<_>>()
                .len()
        );
    }

    #[test]
    fn destructive_commands_require_explicit_authorization() {
        for command in [
            "delete_project",
            "rollback_project_status",
            "cancel_generation_task",
            "detach_derived_project",
            "leave_project",
        ] {
            assert!(is_destructive(command), "{command}");
        }
        for command in ["list_projects", "create_project", "get_project"] {
            assert!(!is_destructive(command), "{command}");
        }
        assert!(
            DESTRUCTIVE_EXCEPTIONS
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        assert!(
            DESTRUCTIVE_EXCEPTIONS
                .iter()
                .all(|command| API_COMMANDS.binary_search(command).is_ok())
        );
        assert!(is_destructive("delete_future_resource"));
    }

    #[test]
    fn high_risk_descriptors_use_explicit_agent_metadata() {
        let provider = command_descriptor("create_llm_config").unwrap();
        assert_eq!(provider.domain, "settings");
        assert_eq!(provider.args_schema["x-a4n-schema-quality"], "typed-input");
        assert!(
            provider
                .secret_pointers
                .contains(&"/input/api_key_encrypted".to_string())
        );

        let generation = command_descriptor("start_chapter_generation").unwrap();
        assert_eq!(generation.execution, "task");
        assert_eq!(generation.idempotency, "client_request_id");
        assert_eq!(
            generation.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert_eq!(generation.result_schema["x-a4n-schema-quality"], "typed");

        let project = command_descriptor("get_project").unwrap();
        assert_eq!(
            project.args_schema["x-a4n-context-fields"],
            serde_json::json!(["projectId"])
        );
        assert_eq!(project.args_schema["x-a4n-schema-quality"], "typed");

        let export = command_descriptor("export_project_family_data").unwrap();
        assert_eq!(export.effect, "read");
        assert_eq!(export.execution, "download");
        assert_eq!(export.idempotency, "safe");

        let admin_create = command_descriptor("create_user").unwrap();
        assert_eq!(
            admin_create.args_schema["x-a4n-context-fields"],
            serde_json::json!(["callerUserId"])
        );
        assert!(admin_create.args_schema["properties"]["input"]["properties"]["role"].is_object());
        assert!(
            admin_create.args_schema["properties"]["input"]["properties"]
                .get("invite_code")
                .is_none()
        );

        for command in [
            "get_user",
            "update_user_profile",
            "change_user_password",
            "list_sessions",
        ] {
            assert_eq!(
                command_descriptor(command).unwrap().args_schema["x-a4n-context-fields"],
                serde_json::json!(["userId"]),
                "{command}"
            );
        }
        for command in ["list_all_users", "list_invite_codes", "create_invite_code"] {
            assert_eq!(
                command_descriptor(command).unwrap().args_schema["x-a4n-context-fields"],
                serde_json::json!(["callerUserId"]),
                "{command}"
            );
        }

        for command in [
            "apply_assembly_postprocess_edit",
            "apply_optimization_patches",
            "apply_source_sync_decisions",
            "prompt_remove_project_override",
            "prompt_reset",
            "restore_blueprint_version",
            "reverse_engineer_blueprint_and_apply",
            "reverse_engineer_outline_and_apply",
            "reverse_engineer_part_outline_and_apply",
        ] {
            assert_eq!(
                command_descriptor(command).unwrap().effect,
                "destructive",
                "{command}"
            );
        }
    }
}
