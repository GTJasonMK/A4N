use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::generation_tasks::generation_start_path;

pub const COMMAND_MANIFEST_VERSION: u32 = 3;
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
    "apply_import_analysis_blueprint_candidate",
    "apply_import_analysis_part_outline_candidate",
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
    "commit_inserted_segment_draft",
    "confirm_reverse_engineering_candidate",
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
    "delete_inserted_segment",
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
    "forward_project_status",
    "generate_chapter_analysis",
    "generate_chapter_scene",
    "generate_chapter_shooting_script",
    "generate_next_chapter_shooting_script_unit",
    "generate_object_profile_image_prompt",
    "generate_portrait_prompt",
    "generate_scene_profile_image_prompt",
    "get_admin_overview",
    "get_blueprint",
    "get_canvas_graph",
    "get_canvas_node_contracts",
    "get_chapter_outlines_pacing",
    "get_chapter_shooting_script",
    "get_chapter_shooting_script_unit_progress",
    "get_chapter_video_timeline",
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
    "get_project_status_machine",
    "get_queue_stats",
    "get_snapshot",
    "get_storage_paths",
    "get_task_history",
    "get_theme_css_variables",
    "get_token_summary",
    "get_usage_guide",
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
    "list_embedding_config",
    "list_image_configs",
    "list_images_by_chapter",
    "list_images_by_project",
    "list_import_queue",
    "list_invite_codes",
    "list_llm_config",
    "list_local_embedding_models",
    "list_manga_panels",
    "list_model_provider_catalog",
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
    "list_video_configs",
    "list_video_reference_images",
    "list_videos_by_chapter",
    "list_workflow_runs",
    "load_cached_assembled_chapter_video_preview",
    "load_video_playback_data",
    "load_video_scrub_playback_data",
    "load_video_segment_plan_draft",
    "login_user",
    "logout_user",
    "modify_import_review",
    "modify_profile_attribute",
    "move_inserted_segment",
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
    "reverse_engineer_outline",
    "reverse_engineer_part_outline",
    "revoke_project_invite",
    "rollback_project_status",
    "rollback_to_chapter",
    "run_canvas_graph",
    "run_canvas_node",
    "run_galgame_asset_generation",
    "run_import_project_analysis",
    "save_blueprint",
    "save_canvas_graph",
    "save_canvas_text_asset",
    "save_chapter_outline",
    "save_part_outline",
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
    "sync_blueprint_materials",
    "sync_character_from_chapters",
    "sync_outline_materials",
    "sync_scene_profiles",
    "test_embedding_config",
    "test_image_config",
    "test_llm_config",
    "test_video_config",
    "trim_video",
    "unbind_galgame_media",
    "update_assembly_push_status",
    "update_chapter_content",
    "update_chapter_outline",
    "update_chapter_scene",
    "update_chapter_video_prompt",
    "update_chapter_video_segment_recipe",
    "update_character_profile",
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
    "update_usage_guide",
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
    pub runtime_config_policy: RuntimeConfigPolicy,
    pub idempotency: String,
    pub invoke: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeConfigPolicy {
    None,
    WithoutSecrets,
    Full,
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

pub fn command_runtime_config_policy(command: &str) -> RuntimeConfigPolicy {
    if matches!(command, "preview_rollback" | "preview_source_sync") {
        return RuntimeConfigPolicy::None;
    }
    let uses_runtime_config = RUNTIME_CONFIG_EXACT_COMMANDS
        .binary_search(&command)
        .is_ok()
        || command.starts_with("generate_")
        || command.starts_with("preview_")
        || command.starts_with("reverse_engineer")
        || command.starts_with("sync_")
        || generation_start_path(command).is_some()
        || command == "start_manga_pipeline"
        || command == "resume_manga_pipeline"
        || command.starts_with("optimize_")
        || command.starts_with("create_optimization")
        || command.starts_with("prepare_optimization")
        || matches!(
            command,
            "test_llm_config" | "test_image_config" | "test_video_config" | "test_embedding_config"
        );
    if !uses_runtime_config {
        RuntimeConfigPolicy::None
    } else if command.starts_with("preview_") {
        RuntimeConfigPolicy::WithoutSecrets
    } else {
        RuntimeConfigPolicy::Full
    }
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
    "/runtime_config_snapshot/llm/api_key_encrypted",
    "/runtime_config_snapshot/image/api_key_encrypted",
    "/runtime_config_snapshot/video/api_key_encrypted",
    "/runtime_config_snapshot/embedding/api_key_encrypted",
    "/runtime_config_snapshot/system_llm_api_key_encrypted",
    "/runtime_config_snapshot/system_image_api_key_encrypted",
    "/runtime_config_snapshot/system_video_api_key_encrypted",
];

const RUNTIME_CONFIG_EXACT_COMMANDS: &[&str] = &[
    "apply_source_sync_decisions",
    "check_character_conflict",
    "check_implicit_update",
    "discover_character_profiles",
    "poll_video_job",
    "prepare_chapter_generation_rag_embedding",
    "rag_enhanced_search",
    "rag_enhanced_search_with_vectors",
    "rag_ingest_all",
    "rag_prepare_enhanced_search",
    "rag_prepare_ingest_all",
    "rag_upload_ingest_embeddings",
    "resume_chapter_analysis_sync",
    "resume_workflow_run",
    "run_canvas_graph",
    "run_canvas_node",
    "run_galgame_asset_generation",
    "run_import_project_analysis",
    "start_galgame_pipeline",
    "start_import_analysis",
];

fn descriptor_override(command: &str) -> DescriptorOverride {
    if generation_start_path(command).is_some() {
        return DescriptorOverride {
            execution: Some("task"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("client_request_id"),
            ..DEFAULT_OVERRIDE
        };
    }
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
        "get_generation_call_analytics"
        | "get_log_dir"
        | "get_storage_paths"
        | "read_recent_logs"
        | "set_log_level" => DescriptorOverride {
            context_fields: Some(CALLER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "cleanup_orphan_project_data_dirs" => DescriptorOverride {
            context_fields: Some(NO_CONTEXT),
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
        "accept_project_invite" | "decline_project_invite" | "list_my_project_invites" => {
            DescriptorOverride {
                domain: Some("project"),
                context_fields: Some(USER_CONTEXT),
                ..DEFAULT_OVERRIDE
            }
        }
        "get_project"
        | "update_project"
        | "delete_project"
        | "duplicate_project"
        | "forward_project_status"
        | "get_project_status_machine"
        | "preview_rollback"
        | "reset_project_generated_data"
        | "rollback_project_status" => DescriptorOverride {
            domain: Some("project"),
            context_fields: Some(PROJECT_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "apply_import_analysis_blueprint_candidate" => DescriptorOverride {
            domain: Some("blueprint"),
            context_fields: Some(PROJECT_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "sync_blueprint_materials" => DescriptorOverride {
            domain: Some("blueprint"),
            execution: Some("immediate"),
            context_fields: Some(PROJECT_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "sync_outline_materials" => DescriptorOverride {
            domain: Some("outline"),
            execution: Some("immediate"),
            context_fields: Some(PROJECT_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "save_chapter_outline" | "save_part_outline" => DescriptorOverride {
            domain: Some("outline"),
            context_fields: Some(USER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "apply_import_analysis_part_outline_candidate" => DescriptorOverride {
            domain: Some("outline"),
            context_fields: Some(PROJECT_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "confirm_reverse_engineering_candidate" => DescriptorOverride {
            domain: Some("workflow"),
            execution: Some("workflow"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("safe"),
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
        | "list_images_by_project"
        | "list_portraits_by_project"
        | "upload_image" => DescriptorOverride {
            context_fields: Some(PROJECT_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "list_chapter_video_prompts" => DescriptorOverride {
            context_fields: Some(PROJECT_CHAPTER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "delete_chapter_video_prompt"
        | "optimize_chapter_video_prompt"
        | "update_chapter_video_prompt" => DescriptorOverride {
            context_fields: Some(USER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "list_video_reference_images" => DescriptorOverride {
            domain: Some("asset"),
            context_fields: Some(PROJECT_CHAPTER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "list_chapter_scenes" => DescriptorOverride {
            context_fields: Some(CHAPTER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "update_chapter_scene" | "delete_chapter_scene" | "reorder_chapter_scenes" => {
            DescriptorOverride {
                context_fields: Some(NO_CONTEXT),
                ..DEFAULT_OVERRIDE
            }
        }
        "update_chapter_content"
        | "update_chapter_outline"
        | "generate_chapter_analysis"
        | "generate_chapter_scene"
        | "list_images_by_chapter"
        | "list_videos_by_chapter"
        | "rollback_to_chapter" => DescriptorOverride {
            context_fields: Some(CHAPTER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "generate_chapter_shooting_script" | "generate_next_chapter_shooting_script_unit" => {
            DescriptorOverride {
                execution: Some("workflow"),
                context_fields: Some(USER_CONTEXT),
                idempotency: Some("server_workflow"),
                ..DEFAULT_OVERRIDE
            }
        }
        "edit_chapter_shooting_script"
        | "get_chapter_shooting_script"
        | "get_chapter_shooting_script_unit_progress" => DescriptorOverride {
            context_fields: Some(USER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "import_chapter" => DescriptorOverride {
            execution: Some("task"),
            context_fields: Some(NO_CONTEXT),
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
        "start_galgame_pipeline" => DescriptorOverride {
            domain: Some("galgame"),
            execution: Some("workflow"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("server_workflow"),
            ..DEFAULT_OVERRIDE
        },
        "cancel_galgame_pipeline" => DescriptorOverride {
            domain: Some("galgame"),
            execution: Some("workflow"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("server_workflow"),
            ..DEFAULT_OVERRIDE
        },
        "preview_galgame_pipeline_request" => DescriptorOverride {
            domain: Some("galgame"),
            effect: Some("read"),
            execution: Some("immediate"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "get_galgame_pipeline_status" => DescriptorOverride {
            domain: Some("galgame"),
            effect: Some("read"),
            execution: Some("workflow"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "get_galgame_script" => DescriptorOverride {
            domain: Some("galgame"),
            effect: Some("read"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "plan_galgame_assets" => DescriptorOverride {
            domain: Some("galgame"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "run_galgame_asset_generation"
        | "bind_galgame_media"
        | "unbind_galgame_media"
        | "import_galgame_audio" => DescriptorOverride {
            domain: Some("galgame"),
            context_fields: Some(NO_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "update_galgame_line"
        | "insert_galgame_line"
        | "update_galgame_scene"
        | "delete_galgame_line"
        | "reorder_galgame_lines"
        | "update_galgame_choices" => DescriptorOverride {
            domain: Some("galgame"),
            context_fields: Some(PROJECT_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "export_galgame_webgal" => DescriptorOverride {
            domain: Some("galgame"),
            effect: Some("read"),
            execution: Some("download"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "resume_chapter_analysis_sync" => DescriptorOverride {
            execution: Some("workflow"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("server_workflow"),
            ..DEFAULT_OVERRIDE
        },
        "get_import_analysis_status" => DescriptorOverride {
            effect: Some("read"),
            execution: Some("workflow"),
            context_fields: Some(PROJECT_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "start_manga_pipeline" | "resume_manga_pipeline" => DescriptorOverride {
            domain: Some("manga"),
            execution: Some("workflow"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("server_workflow"),
            ..DEFAULT_OVERRIDE
        },
        "cancel_manga_pipeline" => DescriptorOverride {
            domain: Some("manga"),
            execution: Some("workflow"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("server_workflow"),
            ..DEFAULT_OVERRIDE
        },
        "preview_start_manga_pipeline_request" | "preview_resume_manga_pipeline_request" => {
            DescriptorOverride {
                domain: Some("manga"),
                effect: Some("read"),
                context_fields: Some(NO_CONTEXT),
                idempotency: Some("safe"),
                ..DEFAULT_OVERRIDE
            }
        }
        "get_manga_pipeline_status" | "list_manga_panels" => DescriptorOverride {
            domain: Some("manga"),
            effect: Some("read"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "bind_manga_reference_image"
        | "delete_manga_by_chapter"
        | "delete_manga_prompt"
        | "optimize_manga_prompt"
        | "update_manga_prompt" => DescriptorOverride {
            domain: Some("manga"),
            context_fields: Some(NO_CONTEXT),
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
        "remove_image_config"
        | "activate_image_config"
        | "remove_video_config"
        | "activate_video_config"
        | "set_default_embedding_config"
        | "delete_embedding_config"
        | "set_default_llm_config"
        | "delete_llm_config"
        | "update_llm_max_tokens_config" => DescriptorOverride {
            domain: Some("settings"),
            context_fields: Some(NO_CONTEXT),
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
        | "get_usage_guide"
        | "list_image_configs"
        | "list_llm_config"
        | "list_video_configs" => DescriptorOverride {
            domain: Some("settings"),
            effect: Some("read"),
            context_fields: Some(NO_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "list_themes" | "get_theme_css_variables" => DescriptorOverride {
            domain: Some("settings"),
            effect: Some("read"),
            context_fields: Some(USER_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "activate_theme" | "create_theme" | "delete_theme" | "update_theme" => DescriptorOverride {
            domain: Some("settings"),
            context_fields: Some(USER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "update_usage_guide" => DescriptorOverride {
            domain: Some("settings"),
            context_fields: Some(CALLER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "export_all_configs_json" => DescriptorOverride {
            domain: Some("export"),
            effect: Some("read"),
            execution: Some("download"),
            context_fields: Some(CALLER_CONTEXT),
            idempotency: Some("safe"),
            ..DEFAULT_OVERRIDE
        },
        "export_theme" => DescriptorOverride {
            domain: Some("export"),
            effect: Some("read"),
            execution: Some("download"),
            context_fields: Some(USER_CONTEXT),
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
        "import_all_configs" => DescriptorOverride {
            domain: Some("import"),
            context_fields: Some(CALLER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "import_theme" => DescriptorOverride {
            domain: Some("import"),
            context_fields: Some(USER_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "import_project_family_upload"
        | "import_project_full_upload"
        | "import_uploaded_file"
        | "import_video_studio_project_upload" => DescriptorOverride {
            domain: Some("import"),
            context_fields: Some(NO_CONTEXT),
            ..DEFAULT_OVERRIDE
        },
        "get_canvas_graph"
        | "get_canvas_node_contracts"
        | "import_canvas_media"
        | "preview_canvas_node_request"
        | "run_canvas_graph"
        | "run_canvas_node"
        | "save_canvas_graph"
        | "save_canvas_text_asset" => DescriptorOverride {
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
        runtime_config_policy: command_runtime_config_policy(command),
        idempotency: idempotency(command).into(),
        invoke: format!("a4n call {command} --args @input.json"),
    }
}

fn args_schema(command: &str) -> Value {
    let schema = match command {
        "login_user" => typed_input_schema::<crate::auth::LoginInput>(),
        "register_user" => typed_input_schema::<crate::auth::RegisterInput>(),
        "create_user" => typed_input_schema::<crate::admin::CreateUserInput>(),
        "get_usage_trends" => typed_input_schema::<crate::admin::UsageTrendsInput>(),
        "get_generation_call_analytics" => {
            typed_input_schema::<crate::admin::GenerationCallAnalyticsInput>()
        }
        "read_recent_logs" => typed_input_schema::<crate::logging::ReadLogsInput>(),
        "set_log_level" => typed_input_schema::<crate::logging::SetLogLevelInput>(),
        "cleanup_orphan_project_data_dirs" => {
            typed_input_schema::<crate::storage::CleanupOrphanProjectDataDirsInput>()
        }
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
        "create_profile" => typed_input_schema::<crate::character::CreateProfileInput>(),
        "update_character_profile" => {
            typed_input_schema::<crate::character::UpdateCharacterProfileInput>()
        }
        "list_profiles" => typed_input_schema::<crate::character::ListProfilesInput>(),
        "sync_character_from_chapters" => {
            required_task_typed_input_schema::<crate::character::SyncCharacterProfileInput>()
        }
        "preview_sync_character_from_chapters_request" => {
            typed_input_schema::<crate::character::SyncCharacterProfileInput>()
        }
        "sync_all_character_profiles" => {
            required_task_typed_input_schema::<crate::character::SyncAllCharacterProfilesInput>()
        }
        "preview_sync_all_character_profiles_request" => {
            typed_input_schema::<crate::character::SyncAllCharacterProfilesInput>()
        }
        "delete_profile" => json!({
            "type": "object",
            "properties": { "profileId": { "type": "integer", "minimum": 1 } },
            "required": ["profileId"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        "invite_project_member" => {
            typed_input_schema::<crate::collaboration::InviteProjectMemberInput>()
        }
        "update_project_member_role" => {
            typed_input_schema::<crate::collaboration::UpdateProjectMemberRoleInput>()
        }
        "start_blueprint_generation" => {
            typed_input_schema::<crate::blueprint::StartBlueprintGenerationInput>()
        }
        "start_blueprint_refinement" => {
            typed_input_schema::<crate::blueprint::StartBlueprintRefinementInput>()
        }
        "preview_generate_blueprint_request" => {
            typed_input_schema::<crate::blueprint::PreviewBlueprintGenerationInput>()
        }
        "preview_refine_blueprint_request" => {
            typed_input_schema::<crate::blueprint::PreviewBlueprintRefinementInput>()
        }
        "list_blueprint_versions" => {
            typed_input_schema::<crate::blueprint::ListBlueprintVersionsInput>()
        }
        "diff_blueprint_versions" => {
            typed_input_schema::<crate::blueprint::DiffBlueprintVersionsInput>()
        }
        "restore_blueprint_version" => {
            typed_input_schema::<crate::blueprint::RestoreBlueprintVersionInput>()
        }
        "save_blueprint" => typed_input_schema::<crate::blueprint::SaveBlueprintInput>(),
        "save_chapter_outline" => typed_input_schema::<crate::outline::SaveChapterOutlineInput>(),
        "save_part_outline" => typed_input_schema::<crate::outline::SavePartOutlineInput>(),
        "update_world_setting" => typed_input_schema::<crate::blueprint::UpdateWorldSettingInput>(),
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
        "start_manga_pipeline" | "preview_start_manga_pipeline_request" => {
            typed_input_schema::<crate::manga::MangaPipelineStartInput>()
        }
        "resume_manga_pipeline" | "preview_resume_manga_pipeline_request" => {
            typed_input_schema::<crate::manga::MangaPipelineResumeInput>()
        }
        "cancel_manga_pipeline"
        | "delete_manga_by_chapter"
        | "get_manga_pipeline_status"
        | "list_manga_panels" => typed_input_schema::<crate::manga::MangaChapterInput>(),
        "bind_manga_reference_image" => {
            typed_input_schema::<crate::manga::BindMangaReferenceImageInput>()
        }
        "update_manga_prompt" => typed_input_schema::<crate::manga::UpdateMangaPromptInput>(),
        "optimize_manga_prompt" => typed_input_schema::<crate::manga::OptimizeMangaPromptInput>(),
        "delete_manga_prompt" => typed_input_schema::<crate::manga::DeleteMangaPromptInput>(),
        "get_galgame_script"
        | "cancel_galgame_pipeline"
        | "get_galgame_pipeline_status"
        | "plan_galgame_assets"
        | "export_galgame_webgal" => typed_input_schema::<crate::galgame::GalgameProjectInput>(),
        "preview_galgame_pipeline_request" => {
            typed_input_schema::<crate::galgame::PreviewGalgamePipelineInput>()
        }
        "run_galgame_asset_generation" => {
            typed_input_schema::<crate::galgame::RunGalgameAssetGenerationInput>()
        }
        "bind_galgame_media" => typed_input_schema::<crate::galgame::BindGalgameMediaInput>(),
        "unbind_galgame_media" => typed_input_schema::<crate::galgame::UnbindGalgameMediaInput>(),
        "import_galgame_audio" => typed_input_schema::<crate::galgame::ImportGalgameAudioInput>(),
        "update_galgame_line" => {
            project_typed_input_schema::<crate::galgame::UpdateGalgameLineInput>()
        }
        "insert_galgame_line" => {
            project_typed_input_schema::<crate::galgame::InsertGalgameLineInput>()
        }
        "update_galgame_scene" => {
            project_typed_input_schema::<crate::galgame::UpdateGalgameSceneInput>()
        }
        "delete_galgame_line" => {
            project_typed_input_schema::<crate::galgame::DeleteGalgameLineInput>()
        }
        "reorder_galgame_lines" => {
            project_typed_input_schema::<crate::galgame::ReorderGalgameLinesInput>()
        }
        "update_galgame_choices" => {
            project_typed_input_schema::<crate::galgame::UpdateGalgameChoicesInput>()
        }
        "create_embedding_config" => typed_input_schema::<crate::config::CreateEmbeddingInput>(),
        "update_embedding_config" => typed_input_schema::<crate::config::UpdateEmbeddingInput>(),
        "create_llm_config" => typed_input_schema::<crate::config::CreateLlmInput>(),
        "update_llm_config" => typed_input_schema::<crate::config::UpdateLlmInput>(),
        "update_llm_max_tokens_config" => typed_input_schema::<crate::config::LlmMaxTokensConfig>(),
        "set_default_llm_config"
        | "delete_llm_config"
        | "set_default_embedding_config"
        | "delete_embedding_config"
        | "remove_image_config"
        | "activate_image_config"
        | "remove_video_config"
        | "activate_video_config" => json!({
            "type": "object",
            "properties": { "configId": { "type": "integer", "minimum": 1 } },
            "required": ["configId"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        "test_embedding_config" | "preview_test_embedding_config_request" => json!({
            "type": "object",
            "properties": {
                "configId": { "type": "integer", "minimum": 1 },
                "runtime_config_snapshot": { "type": ["object", "null"] },
            },
            "required": ["configId"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        "test_llm_config"
        | "test_image_config"
        | "test_video_config"
        | "preview_test_llm_config_request"
        | "preview_test_image_config_request" => json!({
            "type": "object",
            "properties": {
                "configId": {
                    "type": "integer",
                    "anyOf": [{ "const": -1 }, { "minimum": 1 }],
                },
                "runtime_config_snapshot": { "type": ["object", "null"] },
            },
            "required": ["configId"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        "add_image_config" => typed_input_schema::<crate::config::ImageConfigInput>(),
        "update_image_config" => typed_input_schema::<crate::config::UpdateImageConfigInput>(),
        "add_video_config" => typed_input_schema::<crate::config::VideoConfigInput>(),
        "update_video_config" => typed_input_schema::<crate::config::UpdateVideoConfigInput>(),
        "update_usage_guide" => typed_input_schema::<crate::config::UsageGuideDocument>(),
        "edit_chapter_shooting_script" => {
            typed_input_schema::<crate::shooting_script::EditChapterShootingScriptInput>()
        }
        "get_chapter_shooting_script" | "get_chapter_shooting_script_unit_progress" => {
            typed_input_schema::<crate::shooting_script::ChapterShootingScriptSourceInput>()
        }
        "generate_chapter_shooting_script" | "generate_next_chapter_shooting_script_unit" => {
            json!({
                "type": "object",
                "properties": {
                    "input": raw_schema::<crate::shooting_script::ChapterShootingScriptSourceInput>(),
                    "taskId": { "type": ["string", "null"], "minLength": 1 },
                },
                "required": ["input"],
                "additionalProperties": true,
                "x-a4n-schema-quality": "typed-input",
            })
        }
        "list_themes" => empty_args_schema(),
        "create_theme" => typed_input_schema::<crate::theme::CreateThemeInput>(),
        "import_theme" => typed_input_schema::<crate::theme::CreateThemeInput>(),
        "activate_theme" | "delete_theme" | "export_theme" | "get_theme_css_variables" => {
            json!({
                "type": "object",
                "properties": { "themeId": { "type": "integer", "minimum": 1 } },
                "required": ["themeId"],
                "additionalProperties": true,
                "x-a4n-schema-quality": "typed",
            })
        }
        "update_theme" => json!({
            "type": "object",
            "properties": {
                "themeId": { "type": "integer", "minimum": 1 },
                "input": raw_schema::<crate::theme::UpdateThemeInput>(),
            },
            "required": ["themeId", "input"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed-input",
        }),
        "get_embedding_model_download_info" | "delete_local_embedding_model" => {
            typed_input_schema::<crate::embedding::EmbeddingModelInput>()
        }
        "download_embedding_model" => {
            typed_input_schema::<crate::embedding::DownloadEmbeddingModelInput>()
        }
        "cancel_embedding_model_download" => {
            typed_input_schema::<crate::embedding::CancelEmbeddingModelDownloadInput>()
        }
        "cancel_workflow" => typed_input_schema::<crate::workflow::CancelWorkflowInput>(),
        "get_workflow_progress_events" => {
            typed_input_schema::<crate::workflow::WorkflowProgressEventsInput>()
        }
        "resume_workflow_run" => typed_input_schema::<crate::workflow::ResumeWorkflowRunInput>(),
        "list_image_configs"
        | "list_video_configs"
        | "list_llm_config"
        | "list_embedding_config"
        | "list_local_embedding_models"
        | "get_llm_max_tokens_config" => empty_args_schema(),
        "list_images_by_project" => json!({
            "type": "object",
            "properties": { "projectId": { "type": "integer", "minimum": 1 } },
            "required": ["projectId"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        "list_images_by_chapter" => typed_input_schema::<crate::image::ListImagesByChapterInput>(),
        "list_video_reference_images" => {
            typed_input_schema::<crate::image::ListVideoReferenceImagesInput>()
        }
        "update_image" => json!({
            "type": "object",
            "properties": {
                "imageId": { "type": "integer", "minimum": 1 },
                "input": raw_schema::<crate::image::UpdateGeneratedImageInput>(),
            },
            "required": ["imageId", "input"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed-input",
        }),
        "upload_image" => json!({
            "type": "object",
            "properties": {
                "projectId": { "type": "integer", "minimum": 1 },
                "input": raw_schema::<crate::image::UploadImageInput>(),
            },
            "required": ["projectId", "input"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed-input",
        }),
        "delete_image" => json!({
            "type": "object",
            "properties": { "imageId": { "type": "integer", "minimum": 1 } },
            "required": ["imageId"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        "list_videos_by_chapter" => json!({
            "type": "object",
            "properties": {
                "projectId": { "type": "integer", "minimum": 1 },
                "chapterId": { "type": "integer", "minimum": 1 },
            },
            "required": ["projectId", "chapterId"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        "update_video" => json!({
            "type": "object",
            "properties": {
                "videoId": { "type": "integer", "minimum": 1 },
                "input": raw_schema::<crate::video::UpdateGeneratedVideoInput>(),
            },
            "required": ["videoId", "input"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed-input",
        }),
        "delete_video" => json!({
            "type": "object",
            "properties": { "videoId": { "type": "integer", "minimum": 1 } },
            "required": ["videoId"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        "select_segment_video" => json!({
            "type": "object",
            "properties": {
                "projectId": { "type": "integer", "minimum": 1 },
                "chapterId": { "type": "integer", "minimum": 1 },
                "videoId": { "type": "integer", "minimum": 1 },
                "input": raw_schema::<crate::video::SelectSegmentVideoInput>(),
            },
            "required": ["projectId", "chapterId", "videoId", "input"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed-input",
        }),
        "upload_chapter_video" => json!({
            "type": "object",
            "properties": {
                "projectId": { "type": "integer", "minimum": 1 },
                "chapterId": { "type": "integer", "minimum": 1 },
                "input": raw_schema::<crate::video::UploadChapterVideoInput>(),
            },
            "required": ["projectId", "chapterId", "input"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed-input",
        }),
        "upload_reference_media_video" => json!({
            "type": "object",
            "properties": {
                "projectId": { "type": "integer", "minimum": 1 },
                "chapterId": { "type": "integer", "minimum": 1 },
                "input": raw_schema::<crate::video::UploadReferenceMediaVideoInput>(),
            },
            "required": ["projectId", "chapterId", "input"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed-input",
        }),
        "upload_assembly_insert_video" => json!({
            "type": "object",
            "properties": {
                "projectId": { "type": "integer", "minimum": 1 },
                "chapterId": { "type": "integer", "minimum": 1 },
                "workflowMode": { "type": "string", "minLength": 1 },
                "input": raw_schema::<crate::video::UploadAssemblyInsertVideoInput>(),
            },
            "required": ["projectId", "chapterId", "workflowMode", "input"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed-input",
        }),
        "detect_image_config_models"
        | "detect_llm_config_models"
        | "detect_video_config_models" => {
            typed_input_schema::<crate::config::ModelDiscoveryInput>()
        }
        "get_canvas_graph" => typed_input_schema::<crate::canvas::GetCanvasGraphInput>(),
        "get_canvas_node_contracts" => {
            typed_input_schema::<crate::canvas::GetCanvasNodeContractsInput>()
        }
        "save_canvas_graph" => typed_input_schema::<crate::canvas::SaveCanvasGraphInput>(),
        "save_canvas_text_asset" => typed_input_schema::<crate::canvas::SaveCanvasTextAssetInput>(),
        "preview_canvas_node_request" => {
            typed_input_schema::<crate::canvas::PreviewCanvasNodeRequestInput>()
        }
        "run_canvas_graph" => typed_input_schema::<crate::canvas::RunCanvasGraphInput>(),
        "run_canvas_node" => typed_input_schema::<crate::canvas::RunCanvasNodeInput>(),
        "import_canvas_media" => typed_input_schema::<crate::canvas::ImportCanvasMediaInput>(),
        "prompt_list" | "prompt_list_categories" | "prompt_load_defaults" => {
            typed_schema::<crate::prompt::PromptEmptyArgs>()
        }
        "prompt_get_dependencies"
        | "prompt_get_reverse_dependencies"
        | "prompt_reset"
        | "prompt_version_history" => typed_schema::<crate::prompt::PromptNameArgs>(),
        "prompt_list_project_overrides" => typed_schema::<crate::prompt::PromptProjectArgs>(),
        "prompt_render" | "prompt_render_ab" => typed_input_schema::<crate::prompt::RenderInput>(),
        "prompt_update" => typed_input_schema::<crate::prompt::UpdatePromptInput>(),
        "prompt_set_project_override" => typed_input_schema::<crate::prompt::SetOverrideInput>(),
        "prompt_remove_project_override" => {
            typed_input_schema::<crate::prompt::RemoveOverrideInput>()
        }
        "prompt_version_diff" => typed_input_schema::<crate::prompt::VersionDiffInput>(),
        "prompt_validate_variables" => typed_input_schema::<crate::prompt::ValidateVarsInput>(),
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
        "push_assembled_chapter_video" => {
            typed_input_schema::<crate::video::PushAssembledChapterVideoInput>()
        }
        "apply_assembly_postprocess_edit" => {
            typed_input_schema::<crate::video::ApplyAssemblyPostprocessEditInput>()
        }
        "get_project"
        | "delete_project"
        | "duplicate_project"
        | "get_project_status_machine"
        | "reset_project_generated_data"
        | "sync_blueprint_materials"
        | "sync_outline_materials" => json!({
            "type": "object",
            "properties": { "projectId": { "type": "integer", "minimum": 1 } },
            "required": ["projectId"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        "forward_project_status" | "rollback_project_status" => json!({
            "type": "object",
            "properties": {
                "projectId": { "type": "integer", "minimum": 1 },
                "target": raw_schema::<crate::project::ProjectStatus>(),
            },
            "required": ["projectId", "target"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        "preview_rollback" => json!({
            "type": "object",
            "properties": {
                "projectId": { "type": "integer", "minimum": 1 },
                "target": raw_schema::<crate::project::ProjectStatus>(),
            },
            "required": ["projectId", "target"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        "list_chapters" => json!({
            "type": "object",
            "properties": { "projectId": { "type": "integer", "minimum": 1 } },
            "required": ["projectId"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        "list_chapter_video_prompts" => json!({
            "type": "object",
            "properties": {
                "projectId": { "type": "integer", "minimum": 1 },
                "chapterId": { "type": "integer", "minimum": 1 },
            },
            "required": ["projectId", "chapterId"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        "update_chapter_video_prompt" => json!({
            "type": "object",
            "properties": {
                "promptId": { "type": "integer", "minimum": 1 },
                "input": raw_schema::<crate::video::UpdateChapterVideoPromptInput>(),
            },
            "required": ["promptId", "input"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed-input",
        }),
        "optimize_chapter_video_prompt" => json!({
            "type": "object",
            "properties": {
                "promptId": { "type": "integer", "minimum": 1 },
                "input": raw_schema::<crate::video::OptimizeChapterVideoPromptInput>(),
            },
            "required": ["promptId", "input"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed-input",
        }),
        "delete_chapter_video_prompt" => {
            typed_input_schema::<crate::video::DeleteChapterVideoPromptInput>()
        }
        "list_chapter_scenes" => json!({
            "type": "object",
            "properties": { "chapterId": { "type": "integer", "minimum": 1 } },
            "required": ["chapterId"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        }),
        "get_chapter_video_timeline" => typed_input_schema::<crate::video::GetVideoTimelineInput>(),
        "commit_inserted_segment_draft" => {
            typed_input_schema::<crate::video::CommitInsertedSegmentDraftInput>()
        }
        "move_inserted_segment" => typed_input_schema::<crate::video::MoveInsertedSegmentInput>(),
        "delete_inserted_segment" => {
            typed_input_schema::<crate::video::DeleteInsertedSegmentInput>()
        }
        "export_txt_data"
        | "export_outline_json_data"
        | "export_characters_json_data"
        | "export_pdf_data" => {
            required_task_typed_input_schema::<crate::export::ExportProjectInput>()
        }
        "export_markdown_data" | "export_epub_data" => {
            required_task_typed_input_schema::<crate::export::ExportPublicationInput>()
        }
        "export_batch_data" => {
            required_task_typed_input_schema::<crate::export::BatchExportInput>()
        }
        _ => json!({
            "type": "object",
            "additionalProperties": true,
            "x-a4n-schema-quality": "opaque",
        }),
    };
    with_context_fields(schema, context_fields(command))
}

fn collaboration_mutation_returns_unit(command: &str) -> bool {
    matches!(
        command,
        "accept_project_invite"
            | "decline_project_invite"
            | "invite_project_member"
            | "leave_project"
            | "remove_project_member"
            | "revoke_project_invite"
            | "update_project_member_role"
    )
}

fn is_novel_export_data_command(command: &str) -> bool {
    matches!(
        command,
        "export_batch_data"
            | "export_characters_json_data"
            | "export_epub_data"
            | "export_markdown_data"
            | "export_outline_json_data"
            | "export_pdf_data"
            | "export_txt_data"
    )
}

fn result_references_are_empty(command: &str) -> bool {
    collaboration_mutation_returns_unit(command)
        || is_novel_export_data_command(command)
        || matches!(
            command,
            "get_project_status_machine"
                | "generate_chapter_shooting_script"
                | "generate_next_chapter_shooting_script_unit"
                | "get_chapter_shooting_script"
                | "get_chapter_shooting_script_unit_progress"
                | "download_embedding_model"
                | "list_my_project_invites"
                | "list_project_invites"
                | "list_project_members"
                | "preview_rollback"
                | "export_theme"
        )
}

fn result_schema(command: &str) -> Value {
    if generation_start_path(command).is_some() {
        return typed_schema::<crate::generation_tasks::GenerationTaskHandle>();
    }
    if collaboration_mutation_returns_unit(command) {
        return typed_schema::<()>();
    }
    match command {
        "login_user" | "register_user" | "oauth2_callback" => {
            typed_schema::<crate::auth::AuthResult>()
        }
        "refresh_token" => typed_schema::<crate::auth::TokenPairSummary>(),
        "get_current_user" | "update_user_profile" => typed_schema::<crate::auth::UserSummary>(),
        "list_sessions" => typed_schema::<Vec<crate::auth::SessionSummary>>(),
        "logout_user" => typed_schema::<()>(),
        "get_user_usage" => typed_schema::<Vec<crate::admin::UsageLogSummary>>(),
        "get_usage_trends" => typed_schema::<Vec<crate::admin::DailyUsagePoint>>(),
        "get_admin_overview" => typed_schema::<crate::admin::AdminOverviewSummary>(),
        "get_generation_call_analytics" => typed_schema::<crate::admin::GenerationCallAnalytics>(),
        "get_storage_paths" => typed_schema::<crate::storage::StoragePathsSummary>(),
        "list_orphan_project_data_dirs" | "cleanup_orphan_project_data_dirs" => {
            typed_schema::<crate::storage::OrphanProjectDataDirsSummary>()
        }
        "get_log_dir" => typed_schema::<String>(),
        "read_recent_logs" => typed_schema::<crate::logging::LogReadResult>(),
        "set_log_level" => typed_schema::<()>(),
        "list_pending_tasks" => typed_schema::<Vec<crate::admin::TaskQueueSummary>>(),
        "get_queue_stats" => typed_schema::<crate::task::QueueStatsSummary>(),
        "get_task_history" => typed_schema::<Vec<crate::task::TaskInfoSummary>>(),
        "list_active_workflows" => typed_schema::<Vec<crate::workflow::WorkflowStateSummary>>(),
        "get_workflow_progress_events" => {
            typed_schema::<crate::workflow::WorkflowProgressEventsResponse>()
        }
        "cancel_workflow" => typed_schema::<()>(),
        "list_all_projects" => typed_schema::<Vec<crate::admin::AdminProjectSummary>>(),
        "list_projects" => typed_schema::<crate::project::PaginatedProjects>(),
        "list_project_members" => typed_schema::<Vec<crate::collaboration::ProjectMemberSummary>>(),
        "list_project_invites" | "list_my_project_invites" => {
            typed_schema::<Vec<crate::collaboration::ProjectInviteSummary>>()
        }
        "get_project"
        | "create_project"
        | "update_project"
        | "forward_project_status"
        | "rollback_project_status" => typed_schema::<crate::project::ProjectSummary>(),
        "get_project_status_machine" => {
            typed_schema::<crate::project::ProjectStatusMachineSummary>()
        }
        "get_blueprint" => typed_schema::<Option<crate::blueprint::BlueprintSummary>>(),
        "list_blueprint_versions" => {
            typed_schema::<Vec<crate::blueprint::BlueprintVersionSummary>>()
        }
        "diff_blueprint_versions" => typed_schema::<crate::blueprint::BlueprintDiffSummary>(),
        "restore_blueprint_version"
        | "save_blueprint"
        | "sync_blueprint_materials"
        | "update_world_setting" => typed_schema::<crate::blueprint::BlueprintMutationResult>(),
        "sync_outline_materials" => typed_schema::<crate::outline::OutlineMutationResult>(),
        "save_chapter_outline" => typed_schema::<crate::outline::SaveChapterOutlineResult>(),
        "save_part_outline" => typed_schema::<crate::outline::SavePartOutlineResult>(),
        "create_profile" => typed_schema::<crate::character::CharacterProfileCreateResult>(),
        "update_character_profile" => {
            typed_schema::<crate::character::CharacterProfileMutationResult>()
        }
        "list_profiles" => typed_schema::<Vec<crate::character::CharacterProfileSummary>>(),
        "delete_profile" => typed_schema::<crate::character::CharacterProfileDeleteResult>(),
        "sync_character_from_chapters" => {
            typed_schema::<crate::character::SyncCharacterProfileResult>()
        }
        "sync_all_character_profiles" => {
            typed_schema::<crate::character::SyncAllCharacterProfilesResult>()
        }
        "preview_sync_character_from_chapters_request"
        | "preview_sync_all_character_profiles_request" => {
            typed_schema::<crate::request_preview::ModelRequestPreview>()
        }
        "preview_rollback" => typed_schema::<crate::project::ProjectRollbackPreview>(),
        "list_embedding_config" => typed_schema::<Vec<crate::config::EmbeddingConfigSummary>>(),
        "list_local_embedding_models" => {
            typed_schema::<Vec<crate::embedding::LocalEmbeddingModelSummary>>()
        }
        "get_embedding_model_download_info" => {
            typed_schema::<crate::embedding::EmbeddingModelDownloadInfoSummary>()
        }
        "download_embedding_model" => {
            typed_schema::<crate::embedding::LocalEmbeddingModelSummary>()
        }
        "cancel_embedding_model_download" | "delete_local_embedding_model" => typed_schema::<()>(),
        "list_image_configs" => typed_schema::<Vec<crate::config::ImageConfigSummary>>(),
        "list_images_by_project" | "list_images_by_chapter" | "list_video_reference_images" => {
            typed_schema::<Vec<crate::image::GeneratedImageSummary>>()
        }
        "update_image" | "upload_image" => typed_schema::<crate::image::GeneratedImageSummary>(),
        "delete_image" => typed_schema::<()>(),
        "list_videos_by_chapter" => typed_schema::<Vec<crate::video::GeneratedVideoSummary>>(),
        "select_segment_video"
        | "update_video"
        | "upload_assembly_insert_video"
        | "upload_chapter_video"
        | "upload_reference_media_video" => typed_schema::<crate::video::GeneratedVideoSummary>(),
        "delete_video" => typed_schema::<()>(),
        "list_chapter_video_prompts" => {
            typed_schema::<Vec<crate::video::ChapterVideoPromptSummary>>()
        }
        "optimize_chapter_video_prompt" | "update_chapter_video_prompt" => {
            typed_schema::<crate::video::ChapterVideoPromptSummary>()
        }
        "delete_chapter_video_prompt" => typed_schema::<()>(),
        "list_llm_config" => typed_schema::<Vec<crate::config::LlmConfigSummary>>(),
        "set_default_llm_config"
        | "delete_llm_config"
        | "set_default_embedding_config"
        | "delete_embedding_config"
        | "remove_image_config"
        | "activate_image_config"
        | "remove_video_config"
        | "activate_video_config"
        | "test_embedding_config"
        | "test_llm_config"
        | "test_image_config"
        | "test_video_config" => typed_schema::<()>(),
        "get_llm_max_tokens_config" | "update_llm_max_tokens_config" => {
            typed_schema::<crate::config::LlmMaxTokensConfig>()
        }
        "detect_image_config_models"
        | "detect_llm_config_models"
        | "detect_video_config_models" => typed_schema::<crate::config::ModelDiscoveryResult>(),
        "preview_test_embedding_config_request"
        | "preview_test_llm_config_request"
        | "preview_test_image_config_request" => {
            typed_schema::<crate::request_preview::ModelRequestPreview>()
        }
        "list_video_configs" => typed_schema::<Vec<crate::config::VideoConfigSummary>>(),
        "get_usage_guide" | "update_usage_guide" => {
            typed_schema::<crate::config::UsageGuideDocument>()
        }
        "list_themes" => typed_schema::<Vec<crate::theme::ThemeSummary>>(),
        "create_theme" | "update_theme" | "activate_theme" | "import_theme" => {
            typed_schema::<crate::theme::ThemeSummary>()
        }
        "delete_theme" => typed_schema::<()>(),
        "export_theme" | "get_theme_css_variables" => typed_schema::<String>(),
        "list_model_provider_catalog" => {
            typed_schema::<crate::provider_catalog::ModelProviderCatalog>()
        }
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
        "get_galgame_script" | "start_galgame_pipeline" => {
            typed_schema::<crate::galgame::GalgameScriptDetail>()
        }
        "start_manga_pipeline" | "resume_manga_pipeline" => typed_schema::<String>(),
        "preview_start_manga_pipeline_request" | "preview_resume_manga_pipeline_request" => {
            typed_schema::<crate::request_preview::ModelRequestPreview>()
        }
        "get_manga_pipeline_status" => typed_schema::<crate::manga::MangaPipelineStatusSummary>(),
        "cancel_manga_pipeline" => typed_schema::<i64>(),
        "list_manga_panels" => typed_schema::<Vec<crate::manga::MangaPromptSummary>>(),
        "bind_manga_reference_image" | "update_manga_prompt" | "optimize_manga_prompt" => {
            typed_schema::<crate::manga::MangaPromptSummary>()
        }
        "delete_manga_prompt" | "delete_manga_by_chapter" => typed_schema::<()>(),
        "cancel_galgame_pipeline" | "get_galgame_pipeline_status" => {
            typed_schema::<crate::galgame::GalgamePipelineStatusSummary>()
        }
        "preview_galgame_pipeline_request" => {
            typed_schema::<crate::request_preview::ModelRequestPreview>()
        }
        "update_galgame_line" | "insert_galgame_line" => {
            typed_schema::<crate::galgame::GalgameLine>()
        }
        "update_galgame_scene" => typed_schema::<crate::galgame::GalgameScene>(),
        "delete_galgame_line" => typed_schema::<()>(),
        "reorder_galgame_lines" => typed_schema::<Vec<crate::galgame::GalgameLine>>(),
        "update_galgame_choices" => typed_schema::<Vec<crate::galgame::GalgameChoice>>(),
        "plan_galgame_assets" => typed_schema::<crate::galgame::GalgameAssetPlan>(),
        "run_galgame_asset_generation" => {
            typed_schema::<crate::galgame::GalgameAssetGenerationSummary>()
        }
        "bind_galgame_media" | "import_galgame_audio" => {
            typed_schema::<crate::galgame::GalgameMediaBindingSummary>()
        }
        "unbind_galgame_media" => typed_schema::<u64>(),
        "export_galgame_webgal" => typed_schema::<crate::export::ExportFileData>(),
        "get_canvas_graph" | "save_canvas_graph" => {
            typed_schema::<crate::canvas::CanvasGraphDocument>()
        }
        "get_canvas_node_contracts" => typed_schema::<Vec<crate::canvas::CanvasNodeContract>>(),
        "save_canvas_text_asset" => typed_schema::<crate::canvas::CanvasTextAssetSummary>(),
        "preview_canvas_node_request" => typed_schema::<crate::canvas::CanvasNodeRequestPreview>(),
        "import_canvas_media" => typed_schema::<crate::canvas::ImportedCanvasMedia>(),
        "run_canvas_graph" | "run_canvas_node" => typed_schema::<crate::canvas::CanvasRunSummary>(),
        "get_chapter_video_timeline"
        | "commit_inserted_segment_draft"
        | "move_inserted_segment"
        | "delete_inserted_segment" => typed_schema::<crate::video::VideoTimelineSummary>(),
        "get_chapter_shooting_script" => {
            typed_schema::<Option<crate::shooting_script::ChapterShootingScriptView>>()
        }
        "get_chapter_shooting_script_unit_progress" => {
            typed_schema::<Option<crate::shooting_script::ChapterShootingScriptUnitProgressView>>()
        }
        "generate_chapter_shooting_script" => {
            typed_schema::<crate::shooting_script::ChapterShootingScriptView>()
        }
        "generate_next_chapter_shooting_script_unit" => {
            typed_schema::<crate::shooting_script::ChapterShootingScriptUnitProgressView>()
        }
        "edit_chapter_shooting_script" => {
            typed_schema::<crate::shooting_script::EditedChapterShootingScriptView>()
        }
        "prompt_list" => typed_schema::<Vec<crate::prompt::PromptEntrySummary>>(),
        "prompt_load_defaults" => typed_schema::<usize>(),
        "prompt_update" => typed_schema::<crate::prompt::PromptEntrySummary>(),
        "prompt_reset" | "prompt_set_project_override" | "prompt_remove_project_override" => {
            typed_schema::<()>()
        }
        "prompt_version_history" => typed_schema::<Vec<crate::prompt::PromptVersionSummary>>(),
        "prompt_render" | "prompt_version_diff" => typed_schema::<String>(),
        "prompt_render_ab" => typed_schema::<crate::prompt::AbRenderResult>(),
        "prompt_list_project_overrides" => {
            typed_schema::<Vec<crate::prompt::ProjectPromptOverrideSummary>>()
        }
        "prompt_validate_variables"
        | "prompt_get_dependencies"
        | "prompt_get_reverse_dependencies" => typed_schema::<Vec<String>>(),
        "export_txt_data"
        | "export_outline_json_data"
        | "export_characters_json_data"
        | "export_markdown_data"
        | "export_epub_data"
        | "export_pdf_data" => typed_schema::<crate::export::ExportFileData>(),
        "export_batch_data" => typed_schema::<Vec<crate::export::ExportFileData>>(),
        "prompt_list_categories" => typed_schema::<Vec<crate::prompt::CategoryInfo>>(),
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

fn required_task_typed_input_schema<T: JsonSchema>() -> Value {
    json!({
        "type": "object",
        "properties": {
            "input": raw_schema::<T>(),
            "taskId": {
                "type": "string",
                "minLength": 1,
                "maxLength": 160,
                "pattern": "^[A-Za-z0-9_.:-]+$",
            },
        },
        "required": ["input", "taskId"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed-input",
    })
}

fn project_typed_input_schema<T: JsonSchema>() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<T>(),
        },
        "required": ["projectId", "input"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed-input",
    })
}

fn empty_args_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
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
    let mut pointers = descriptor_override(command)
        .secret_pointers
        .unwrap_or_default()
        .iter()
        .map(|value| (*value).into())
        .collect::<Vec<String>>();
    if command_runtime_config_policy(command) == RuntimeConfigPolicy::Full {
        pointers.extend(
            RUNTIME_SNAPSHOT_SECRET_POINTERS
                .iter()
                .map(|value| (*value).into()),
        );
    }
    pointers
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
    if result_references_are_empty(command) {
        return Vec::new();
    }
    let galgame_references: &[&str] = match command {
        "get_galgame_script" | "start_galgame_pipeline" => &["galgame_script_ids"],
        "cancel_galgame_pipeline" | "get_galgame_pipeline_status" => &["workflow_run_ids"],
        "update_galgame_line" | "insert_galgame_line" | "reorder_galgame_lines" => {
            &["galgame_line_ids"]
        }
        "update_galgame_scene" => &["galgame_scene_ids"],
        "update_galgame_choices" => &["galgame_choice_ids"],
        "plan_galgame_assets" => &["generation_spec_ids"],
        "run_galgame_asset_generation" => &["generation_spec_ids", "media_asset_ids"],
        "bind_galgame_media" | "import_galgame_audio" => &["media_binding_ids", "media_asset_ids"],
        "delete_galgame_line" | "unbind_galgame_media" | "preview_galgame_pipeline_request" => &[],
        _ => &[],
    };
    if !galgame_references.is_empty() {
        return galgame_references
            .iter()
            .map(|reference| (*reference).to_string())
            .collect();
    }
    if command == "edit_chapter_shooting_script" {
        return vec!["project_ids".into()];
    }
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

const ADMIN_COMMANDS: &[&str] = &[
    "activate_image_config",
    "activate_video_config",
    "add_image_config",
    "add_video_config",
    "cancel_admin_task",
    "cleanup_orphan_project_data_dirs",
    "create_embedding_config",
    "create_invite_code",
    "create_llm_config",
    "create_user",
    "delete_embedding_config",
    "delete_llm_config",
    "disable_user",
    "export_all_configs_json",
    "get_admin_overview",
    "get_generation_call_analytics",
    "get_log_dir",
    "get_storage_paths",
    "get_token_summary",
    "get_usage_trends",
    "get_user_usage",
    "health_check",
    "import_all_configs",
    "list_all_projects",
    "list_all_users",
    "list_invite_codes",
    "list_orphan_project_data_dirs",
    "list_pending_tasks",
    "read_recent_logs",
    "remove_image_config",
    "remove_video_config",
    "set_default_embedding_config",
    "set_default_llm_config",
    "set_log_level",
    "update_embedding_config",
    "update_image_config",
    "update_llm_config",
    "update_llm_max_tokens_config",
    "update_usage_guide",
    "update_user_role",
    "update_video_config",
];

fn auth(command: &str) -> &'static str {
    match command {
        "login_user"
        | "register_user"
        | "refresh_token"
        | "oauth2_authorize_url"
        | "oauth2_callback" => "public",
        command if ADMIN_COMMANDS.contains(&command) => "admin",
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
    if is_novel_export_data_command(command) {
        return Vec::new();
    }
    if matches!(
        command,
        "start_manga_pipeline"
            | "resume_manga_pipeline"
            | "preview_start_manga_pipeline_request"
            | "preview_resume_manga_pipeline_request"
            | "optimize_manga_prompt"
    ) {
        return vec!["prompt", "llm"];
    }
    if matches!(
        command,
        "bind_manga_reference_image"
            | "cancel_manga_pipeline"
            | "delete_manga_by_chapter"
            | "delete_manga_prompt"
            | "get_manga_pipeline_status"
            | "list_manga_panels"
            | "update_manga_prompt"
    ) {
        return Vec::new();
    }
    if matches!(
        command,
        "start_galgame_pipeline" | "preview_galgame_pipeline_request"
    ) {
        return vec!["prompt", "llm"];
    }
    if command == "run_galgame_asset_generation" {
        return vec!["prompt", "image"];
    }
    if command == "export_galgame_webgal" {
        return vec!["export"];
    }
    if matches!(
        command,
        "delete_image"
            | "list_images_by_chapter"
            | "list_images_by_project"
            | "list_video_reference_images"
            | "update_image"
            | "upload_image"
            | "delete_video"
            | "list_videos_by_chapter"
            | "select_segment_video"
            | "update_video"
            | "upload_assembly_insert_video"
            | "upload_chapter_video"
            | "upload_reference_media_video"
            | "delete_chapter_video_prompt"
            | "list_chapter_video_prompts"
            | "update_chapter_video_prompt"
    ) {
        return Vec::new();
    }
    if command == "optimize_chapter_video_prompt" {
        return vec!["prompt", "llm"];
    }
    if matches!(
        command,
        "generate_chapter_shooting_script" | "generate_next_chapter_shooting_script_unit"
    ) {
        return vec!["prompt", "llm"];
    }
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
    "confirm_reverse_engineering_candidate",
    "execute_attribute_deletion",
    "leave_project",
    "prompt_remove_project_override",
    "prompt_reset",
    "request_attribute_deletion",
    "restore_blueprint_version",
    "select_segment_video",
    "update_assembly_push_status",
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::{config::USAGE_GUIDE_MAX_BLOCKS, generation_tasks::GENERATION_START_ROUTES};

    use super::{
        ADMIN_COMMANDS, API_COMMANDS, DESTRUCTIVE_EXCEPTIONS, RUNTIME_CONFIG_EXACT_COMMANDS,
        RUNTIME_EQUIVALENTS, RuntimeConfigPolicy, command_descriptor,
        command_runtime_config_policy, is_destructive,
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
        assert!(ADMIN_COMMANDS.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            ADMIN_COMMANDS.len(),
            ADMIN_COMMANDS.iter().copied().collect::<HashSet<_>>().len()
        );
        assert!(
            ADMIN_COMMANDS
                .iter()
                .all(|command| API_COMMANDS.binary_search(command).is_ok())
        );
        assert!(
            RUNTIME_CONFIG_EXACT_COMMANDS
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        assert!(
            RUNTIME_CONFIG_EXACT_COMMANDS
                .iter()
                .all(|command| API_COMMANDS.binary_search(command).is_ok())
        );
        assert!(
            GENERATION_START_ROUTES
                .windows(2)
                .all(|pair| pair[0].0 < pair[1].0)
        );
        assert_eq!(
            GENERATION_START_ROUTES.len(),
            GENERATION_START_ROUTES
                .iter()
                .map(|(command, _)| *command)
                .collect::<HashSet<_>>()
                .len()
        );
        assert_eq!(
            GENERATION_START_ROUTES.len(),
            GENERATION_START_ROUTES
                .iter()
                .map(|(_, path)| *path)
                .collect::<HashSet<_>>()
                .len()
        );
        assert!(
            GENERATION_START_ROUTES
                .iter()
                .all(|(command, _)| API_COMMANDS.binary_search(command).is_ok())
        );
    }

    #[test]
    fn chapter_scene_descriptors_match_host_arguments() {
        let list = command_descriptor("list_chapter_scenes").unwrap();
        assert_eq!(
            list.args_schema["x-a4n-context-fields"],
            serde_json::json!(["chapterId"])
        );
        assert_eq!(
            list.args_schema["required"],
            serde_json::json!(["chapterId"])
        );
        assert_eq!(
            list.args_schema["properties"]["chapterId"]["minimum"],
            serde_json::json!(1)
        );
        assert!(list.args_schema["properties"].get("projectId").is_none());

        for command in [
            "update_chapter_scene",
            "delete_chapter_scene",
            "reorder_chapter_scenes",
        ] {
            assert_eq!(
                command_descriptor(command).unwrap().args_schema["x-a4n-context-fields"],
                serde_json::json!([]),
                "{command}"
            );
        }
    }

    #[test]
    fn project_status_descriptors_match_host_contracts() {
        let statuses = serde_json::json!(
            crate::project::ProjectStatus::ALL.map(crate::project::ProjectStatus::as_str)
        );
        for command in ["forward_project_status", "rollback_project_status"] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["required"],
                serde_json::json!(["projectId", "target"])
            );
            assert_eq!(
                descriptor.args_schema["properties"]["target"]["enum"],
                statuses
            );
            assert_eq!(descriptor.result_schema["x-a4n-schema-quality"], "typed");
            assert_eq!(
                descriptor.result_references,
                vec![String::from("project_ids")]
            );
        }

        let machine = command_descriptor("get_project_status_machine").unwrap();
        assert_eq!(
            machine.args_schema["required"],
            serde_json::json!(["projectId"])
        );
        assert_eq!(machine.result_schema["x-a4n-schema-quality"], "typed");
        assert!(machine.result_references.is_empty());

        let preview = command_descriptor("preview_rollback").unwrap();
        assert_eq!(
            preview.args_schema["required"],
            serde_json::json!(["projectId", "target"])
        );
        assert_eq!(
            preview.args_schema["properties"]["target"]["enum"],
            statuses
        );
        assert_eq!(preview.result_schema["x-a4n-schema-quality"], "typed");
        assert!(preview.result_references.is_empty());
    }

    #[test]
    fn embedding_model_descriptors_match_host_contracts() {
        for command in [
            "get_embedding_model_download_info",
            "download_embedding_model",
            "cancel_embedding_model_download",
            "delete_local_embedding_model",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-schema-quality"], "typed-input",
                "{command}"
            );
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{command}"
            );
        }

        let list = command_descriptor("list_local_embedding_models").unwrap();
        assert_eq!(list.args_schema["x-a4n-schema-quality"], "typed");
        assert_eq!(list.result_schema["x-a4n-schema-quality"], "typed");

        let info = command_descriptor("get_embedding_model_download_info").unwrap();
        assert!(
            info.args_schema["properties"]["input"]["properties"]
                .get("task_id")
                .is_none()
        );
        let download = command_descriptor("download_embedding_model").unwrap();
        assert!(
            download.args_schema["properties"]["input"]["properties"]
                .get("task_id")
                .is_some()
        );
        assert!(download.result_references.is_empty());
    }

    #[test]
    fn collaboration_descriptors_match_host_arguments_and_results() {
        for command in [
            "accept_project_invite",
            "decline_project_invite",
            "list_my_project_invites",
        ] {
            assert_eq!(
                command_descriptor(command).unwrap().args_schema["x-a4n-context-fields"],
                serde_json::json!(["userId"]),
                "{command}"
            );
        }

        for command in [
            "accept_project_invite",
            "decline_project_invite",
            "invite_project_member",
            "leave_project",
            "remove_project_member",
            "revoke_project_invite",
            "update_project_member_role",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(descriptor.result_schema["x-a4n-schema-quality"], "typed");
            assert!(descriptor.result_references.is_empty(), "{command}");
        }

        for command in [
            "list_my_project_invites",
            "list_project_invites",
            "list_project_members",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(descriptor.result_schema["x-a4n-schema-quality"], "typed");
            assert!(descriptor.result_references.is_empty(), "{command}");
        }
    }

    #[test]
    fn workflow_task_descriptors_use_canonical_contracts() {
        for command in [
            "cancel_workflow",
            "get_workflow_progress_events",
            "resume_workflow_run",
        ] {
            assert_eq!(
                command_descriptor(command).unwrap().args_schema["x-a4n-schema-quality"],
                "typed-input",
                "{command}"
            );
        }

        for command in [
            "cancel_workflow",
            "get_queue_stats",
            "get_task_history",
            "get_workflow_progress_events",
            "list_active_workflows",
        ] {
            assert_eq!(
                command_descriptor(command).unwrap().result_schema["x-a4n-schema-quality"],
                "typed",
                "{command}"
            );
        }

        let progress = command_descriptor("get_workflow_progress_events").unwrap();
        assert_eq!(
            progress.args_schema["properties"]["input"]["properties"]["limit"]["minimum"],
            serde_json::json!(1)
        );
        assert_eq!(
            progress.args_schema["properties"]["input"]["properties"]["limit"]["maximum"],
            serde_json::json!(200)
        );
    }

    #[test]
    fn canvas_descriptors_use_canonical_contracts() {
        for command in [
            "get_canvas_graph",
            "get_canvas_node_contracts",
            "import_canvas_media",
            "preview_canvas_node_request",
            "run_canvas_graph",
            "run_canvas_node",
            "save_canvas_graph",
            "save_canvas_text_asset",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-schema-quality"], "typed-input",
                "{command} args"
            );
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{command} result"
            );
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([]),
                "{command} context"
            );
        }
    }

    #[test]
    fn galgame_descriptors_use_one_typed_host_contract() {
        let commands = [
            "bind_galgame_media",
            "cancel_galgame_pipeline",
            "delete_galgame_line",
            "export_galgame_webgal",
            "get_galgame_pipeline_status",
            "get_galgame_script",
            "import_galgame_audio",
            "insert_galgame_line",
            "plan_galgame_assets",
            "preview_galgame_pipeline_request",
            "reorder_galgame_lines",
            "run_galgame_asset_generation",
            "start_galgame_pipeline",
            "unbind_galgame_media",
            "update_galgame_choices",
            "update_galgame_line",
            "update_galgame_scene",
        ];
        let project_context_commands = [
            "delete_galgame_line",
            "insert_galgame_line",
            "reorder_galgame_lines",
            "update_galgame_choices",
            "update_galgame_line",
            "update_galgame_scene",
        ];

        for command in commands {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(descriptor.domain, "galgame", "{command}");
            assert_eq!(
                descriptor.args_schema["x-a4n-schema-quality"], "typed-input",
                "{command} args"
            );
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{command} result"
            );
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                if project_context_commands.contains(&command) {
                    serde_json::json!(["projectId"])
                } else {
                    serde_json::json!([])
                },
                "{command} context"
            );
        }

        let start = command_descriptor("start_galgame_pipeline").unwrap();
        assert!(
            start.args_schema["properties"]["input"]["required"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("task_id"))
        );
        assert_eq!(start.runtime_config_policy, RuntimeConfigPolicy::Full);
        assert_eq!(
            command_descriptor("run_galgame_asset_generation")
                .unwrap()
                .runtime_config_policy,
            RuntimeConfigPolicy::Full
        );
    }

    #[test]
    fn manga_descriptors_use_one_typed_host_contract() {
        let commands = [
            "bind_manga_reference_image",
            "cancel_manga_pipeline",
            "delete_manga_by_chapter",
            "delete_manga_prompt",
            "get_manga_pipeline_status",
            "list_manga_panels",
            "optimize_manga_prompt",
            "preview_resume_manga_pipeline_request",
            "preview_start_manga_pipeline_request",
            "resume_manga_pipeline",
            "start_manga_pipeline",
            "update_manga_prompt",
        ];
        let llm_commands = [
            "optimize_manga_prompt",
            "preview_resume_manga_pipeline_request",
            "preview_start_manga_pipeline_request",
            "resume_manga_pipeline",
            "start_manga_pipeline",
        ];

        for command in commands {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(descriptor.domain, "manga", "{command}");
            assert_eq!(
                descriptor.args_schema["x-a4n-schema-quality"], "typed-input",
                "{command} args"
            );
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{command} result"
            );
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([]),
                "{command} context"
            );
            assert_eq!(
                descriptor.args_schema["properties"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .collect::<Vec<_>>(),
                vec!["input"],
                "{command} top-level args"
            );
            assert_eq!(
                descriptor.required_capabilities,
                if llm_commands.contains(&command) {
                    vec!["prompt".to_string(), "llm".to_string()]
                } else {
                    Vec::new()
                },
                "{command} capabilities"
            );
        }

        for command in [
            "bind_manga_reference_image",
            "delete_manga_prompt",
            "optimize_manga_prompt",
            "update_manga_prompt",
        ] {
            assert!(
                command_descriptor(command).unwrap().args_schema["properties"]["input"]["required"]
                    .as_array()
                    .unwrap()
                    .contains(&serde_json::json!("downstream_strategy")),
                "{command}"
            );
        }

        for command in [
            "start_manga_pipeline",
            "resume_manga_pipeline",
            "optimize_manga_prompt",
        ] {
            assert_eq!(
                command_descriptor(command).unwrap().runtime_config_policy,
                RuntimeConfigPolicy::Full,
                "{command}"
            );
        }
        for command in [
            "preview_start_manga_pipeline_request",
            "preview_resume_manga_pipeline_request",
        ] {
            assert_eq!(
                command_descriptor(command).unwrap().runtime_config_policy,
                RuntimeConfigPolicy::WithoutSecrets,
                "{command}"
            );
        }
    }

    #[test]
    fn runtime_config_policy_is_manifest_owned() {
        let apply = command_descriptor("apply_source_sync_decisions").unwrap();
        assert_eq!(apply.runtime_config_policy, RuntimeConfigPolicy::Full);
        assert!(
            apply
                .secret_pointers
                .contains(&"/runtime_config_snapshot/llm/api_key_encrypted".to_string())
        );

        assert_eq!(
            command_descriptor("preview_chapter_prompt")
                .unwrap()
                .runtime_config_policy,
            RuntimeConfigPolicy::WithoutSecrets
        );
        assert_eq!(
            command_descriptor("preview_source_sync")
                .unwrap()
                .runtime_config_policy,
            RuntimeConfigPolicy::None
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

        let blueprint_sync = command_descriptor("sync_blueprint_materials").unwrap();
        assert_eq!(blueprint_sync.domain, "blueprint");
        assert_eq!(blueprint_sync.execution, "immediate");
        assert_eq!(blueprint_sync.idempotency, "safe");
        assert_eq!(
            blueprint_sync.args_schema["x-a4n-context-fields"],
            serde_json::json!(["projectId"])
        );
        assert_eq!(blueprint_sync.args_schema["x-a4n-schema-quality"], "typed");
        assert_eq!(
            blueprint_sync.result_schema["x-a4n-schema-quality"],
            "typed"
        );
        assert_eq!(
            command_runtime_config_policy("sync_blueprint_materials"),
            RuntimeConfigPolicy::Full
        );

        let outline_sync = command_descriptor("sync_outline_materials").unwrap();
        assert_eq!(outline_sync.domain, "outline");
        assert_eq!(outline_sync.execution, "immediate");
        assert_eq!(outline_sync.idempotency, "safe");
        assert_eq!(
            outline_sync.args_schema["x-a4n-context-fields"],
            serde_json::json!(["projectId"])
        );
        assert_eq!(outline_sync.args_schema["x-a4n-schema-quality"], "typed");
        assert_eq!(outline_sync.result_schema["x-a4n-schema-quality"], "typed");
        assert_eq!(
            command_runtime_config_policy("sync_outline_materials"),
            RuntimeConfigPolicy::Full
        );

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

        let usage_guide_update = command_descriptor("update_usage_guide").unwrap();
        assert_eq!(usage_guide_update.auth, "admin");
        assert_eq!(
            usage_guide_update.args_schema["x-a4n-context-fields"],
            serde_json::json!(["callerUserId"])
        );
        assert_eq!(
            usage_guide_update.args_schema["properties"]["input"]["properties"]["blocks"]["maxItems"],
            serde_json::json!(USAGE_GUIDE_MAX_BLOCKS)
        );

        let full_config_export = command_descriptor("export_all_configs_json").unwrap();
        assert_eq!(full_config_export.auth, "admin");
        assert_eq!(
            full_config_export.args_schema["x-a4n-context-fields"],
            serde_json::json!(["callerUserId"])
        );

        let analytics = command_descriptor("get_generation_call_analytics").unwrap();
        assert_eq!(analytics.auth, "admin");
        assert_eq!(analytics.args_schema["x-a4n-schema-quality"], "typed-input");
        assert_eq!(analytics.result_schema["x-a4n-schema-quality"], "typed");
        assert_eq!(
            analytics.args_schema["properties"]["input"]["required"],
            serde_json::json!(["days", "recent_limit"])
        );
        assert_eq!(
            analytics.args_schema["properties"]["input"]["properties"]["days"]["maximum"],
            serde_json::json!(180)
        );
        assert_eq!(
            analytics.args_schema["properties"]["input"]["properties"]["recent_limit"]["maximum"],
            serde_json::json!(200)
        );

        let storage_paths = command_descriptor("get_storage_paths").unwrap();
        assert_eq!(storage_paths.auth, "admin");
        assert_eq!(
            storage_paths.args_schema["x-a4n-context-fields"],
            serde_json::json!(["callerUserId"])
        );
        assert_eq!(storage_paths.result_schema["x-a4n-schema-quality"], "typed");

        let log_read = command_descriptor("read_recent_logs").unwrap();
        assert_eq!(log_read.auth, "admin");
        assert_eq!(log_read.args_schema["x-a4n-schema-quality"], "typed-input");
        assert_eq!(log_read.result_schema["x-a4n-schema-quality"], "typed");
        assert_eq!(
            log_read.args_schema["properties"]["input"]["required"],
            serde_json::json!(["lines", "days_ago"])
        );
        assert_eq!(
            log_read.args_schema["properties"]["input"]["properties"]["lines"]["maximum"],
            serde_json::json!(10_000)
        );

        let orphan_cleanup = command_descriptor("cleanup_orphan_project_data_dirs").unwrap();
        assert_eq!(orphan_cleanup.auth, "admin");
        assert_eq!(
            orphan_cleanup.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert_eq!(
            orphan_cleanup.args_schema["x-a4n-schema-quality"],
            "typed-input"
        );
        assert_eq!(
            orphan_cleanup.result_schema["x-a4n-schema-quality"],
            "typed"
        );

        for command in ADMIN_COMMANDS {
            assert_eq!(
                command_descriptor(command).unwrap().auth,
                "admin",
                "{command}"
            );
        }
        assert_eq!(
            command_descriptor("get_usage_trends").unwrap().args_schema["properties"]["input"]["properties"]
                ["days"]["minimum"],
            serde_json::json!(1)
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
        for command in ["list_sessions", "logout_user", "oauth2_callback"] {
            assert_eq!(
                command_descriptor(command).unwrap().result_schema["x-a4n-schema-quality"],
                "typed",
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
            "confirm_reverse_engineering_candidate",
            "prompt_remove_project_override",
            "prompt_reset",
            "restore_blueprint_version",
        ] {
            assert_eq!(
                command_descriptor(command).unwrap().effect,
                "destructive",
                "{command}"
            );
        }
    }

    #[test]
    fn prompt_descriptors_are_fully_typed_and_exclude_retired_graph_command() {
        for command in [
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
        ] {
            let descriptor = command_descriptor(command).expect("active prompt command");
            assert_ne!(
                descriptor.args_schema["x-a4n-schema-quality"], "opaque",
                "{command} args"
            );
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{command} result"
            );
        }
        assert!(command_descriptor("prompt_dependency_graph").is_none());
    }

    #[test]
    fn theme_descriptors_are_fully_typed_and_user_scoped() {
        for command in [
            "activate_theme",
            "create_theme",
            "delete_theme",
            "export_theme",
            "get_theme_css_variables",
            "import_theme",
            "list_themes",
            "update_theme",
        ] {
            let descriptor = command_descriptor(command).expect("active theme command");
            assert_ne!(
                descriptor.args_schema["x-a4n-schema-quality"], "opaque",
                "{command} args"
            );
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{command} result"
            );
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!(["userId"]),
                "{command} context"
            );
        }

        assert_eq!(
            command_descriptor("update_theme").unwrap().args_schema["properties"]["themeId"]["minimum"],
            1
        );
        assert!(
            command_descriptor("export_theme")
                .unwrap()
                .result_references
                .is_empty()
        );
    }

    #[test]
    fn llm_descriptors_preserve_strict_id_and_discovery_contracts() {
        let discovery = command_descriptor("detect_llm_config_models").unwrap();
        let discovery_input = &discovery.args_schema["properties"]["input"];
        assert!(
            discovery_input["required"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("config_id"))
        );
        assert_eq!(
            discovery_input["properties"]["config_id"]["type"],
            serde_json::json!(["integer", "null"])
        );
        assert_eq!(discovery.args_schema["x-a4n-schema-quality"], "typed-input");
        assert_eq!(discovery.result_schema["x-a4n-schema-quality"], "typed");

        for command in ["set_default_llm_config", "delete_llm_config"] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["properties"]["configId"]["minimum"], 1,
                "{command}"
            );
            assert_eq!(descriptor.result_schema["x-a4n-schema-quality"], "typed");
        }

        for command in ["test_llm_config", "preview_test_llm_config_request"] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["properties"]["configId"]["anyOf"],
                serde_json::json!([{ "const": -1 }, { "minimum": 1 }]),
                "{command}"
            );
            assert_eq!(
                descriptor.args_schema["properties"]["runtime_config_snapshot"]["type"],
                serde_json::json!(["object", "null"]),
                "{command}"
            );
            assert_eq!(
                descriptor.args_schema["required"],
                serde_json::json!(["configId"]),
                "{command}"
            );
            assert_ne!(
                descriptor.args_schema["x-a4n-schema-quality"], "opaque",
                "{command} args"
            );
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{command} result"
            );
        }
    }

    #[test]
    fn provider_config_descriptors_use_one_host_shape() {
        for command in [
            "list_image_configs",
            "list_llm_config",
            "list_video_configs",
            "list_embedding_config",
            "get_llm_max_tokens_config",
        ] {
            let descriptor = command_descriptor(command).expect("provider list command");
            assert_eq!(
                descriptor.args_schema["x-a4n-schema-quality"], "typed",
                "{command} args"
            );
            assert_eq!(
                descriptor.args_schema["additionalProperties"], false,
                "{command} args"
            );
        }

        for command in [
            "activate_image_config",
            "activate_video_config",
            "add_image_config",
            "add_video_config",
            "create_embedding_config",
            "create_llm_config",
            "delete_embedding_config",
            "delete_llm_config",
            "remove_image_config",
            "remove_video_config",
            "set_default_embedding_config",
            "set_default_llm_config",
            "update_embedding_config",
            "update_image_config",
            "update_llm_config",
            "update_llm_max_tokens_config",
            "update_video_config",
        ] {
            let descriptor = command_descriptor(command).expect("provider config mutation");
            assert_eq!(descriptor.auth, "admin", "{command} auth");
            assert_eq!(descriptor.domain, "settings", "{command} domain");
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([]),
                "{command} context"
            );
            assert_ne!(
                descriptor.args_schema["x-a4n-schema-quality"], "opaque",
                "{command} args"
            );
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{command} result"
            );
        }

        for command in [
            "test_embedding_config",
            "test_image_config",
            "test_video_config",
        ] {
            let descriptor = command_descriptor(command).expect("provider config test");
            assert_eq!(
                descriptor.args_schema["x-a4n-schema-quality"], "typed",
                "{command} args"
            );
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([]),
                "{command} context"
            );
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{command} result"
            );
        }

        let embedding_test = command_descriptor("test_embedding_config").unwrap();
        assert_eq!(
            embedding_test.args_schema["properties"]["configId"]["minimum"],
            1
        );
        assert_eq!(
            embedding_test.args_schema["properties"]["runtime_config_snapshot"]["type"],
            serde_json::json!(["object", "null"])
        );

        for command in [
            "preview_test_embedding_config_request",
            "preview_test_image_config_request",
        ] {
            let descriptor = command_descriptor(command).expect("provider preview command");
            assert_ne!(
                descriptor.args_schema["x-a4n-schema-quality"], "opaque",
                "{command} args"
            );
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{command} result"
            );
        }
    }

    #[test]
    fn image_record_descriptors_are_typed_without_provider_capabilities() {
        assert!(command_descriptor("score_image").is_none());
        assert!(command_descriptor("toggle_image_favorite").is_none());

        for command in [
            "delete_image",
            "list_images_by_chapter",
            "list_images_by_project",
            "list_video_reference_images",
            "update_image",
            "upload_image",
        ] {
            let descriptor = command_descriptor(command).expect("image record command");
            assert_ne!(
                descriptor.args_schema["x-a4n-schema-quality"], "opaque",
                "{command} args"
            );
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{command} result"
            );
            assert!(
                descriptor.required_capabilities.is_empty(),
                "{command} must not require an image provider"
            );
        }

        let update = command_descriptor("update_image").unwrap();
        assert_eq!(update.args_schema["properties"]["imageId"]["minimum"], 1);
        let upload = command_descriptor("upload_image").unwrap();
        assert!(
            upload.args_schema["properties"]["input"]["properties"]
                .get("model")
                .is_none()
        );
    }

    #[test]
    fn video_record_descriptors_are_typed_without_provider_capabilities() {
        assert!(command_descriptor("toggle_video_favorite").is_none());

        for command in [
            "delete_video",
            "list_videos_by_chapter",
            "select_segment_video",
            "update_video",
            "upload_assembly_insert_video",
            "upload_chapter_video",
            "upload_reference_media_video",
        ] {
            let descriptor = command_descriptor(command).expect("video record command");
            assert_ne!(
                descriptor.args_schema["x-a4n-schema-quality"], "opaque",
                "{command} args"
            );
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{command} result"
            );
            assert!(
                descriptor.required_capabilities.is_empty(),
                "{command} must not require a video provider"
            );
        }

        let update = command_descriptor("update_video").unwrap();
        assert_eq!(update.args_schema["properties"]["videoId"]["minimum"], 1);
        let update_fields = &update.args_schema["properties"]["input"]["properties"];
        for removed in [
            "clear_negative_prompt",
            "metadata_json",
            "clear_metadata_json",
            "insert_after_video_id",
            "move_assembly_insert_to_start",
        ] {
            assert!(update_fields.get(removed).is_none(), "{removed}");
        }
        assert_eq!(
            update.result_schema["properties"]["created_at"]["type"],
            "string"
        );
        assert_eq!(
            update.result_schema["properties"]["updated_at"]["type"],
            "string"
        );
        assert!(
            update.result_schema["properties"]
                .get("timeline_item_id")
                .is_some()
        );
    }

    #[test]
    fn chapter_video_prompt_descriptors_use_one_strict_contract() {
        let list = command_descriptor("list_chapter_video_prompts").unwrap();
        assert_eq!(
            list.args_schema["required"],
            serde_json::json!(["projectId", "chapterId"])
        );
        assert_eq!(
            list.args_schema["x-a4n-context-fields"],
            serde_json::json!(["projectId", "chapterId"])
        );
        assert!(list.required_capabilities.is_empty());
        let prompt_properties =
            &list.result_schema["$defs"]["ChapterVideoPromptSummary"]["properties"];
        assert!(prompt_properties.get("created_at").is_none());
        assert_eq!(prompt_properties["updated_at"]["type"], "string");

        for command in [
            "delete_chapter_video_prompt",
            "optimize_chapter_video_prompt",
            "update_chapter_video_prompt",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!(["userId"]),
                "{command}"
            );
            assert_eq!(
                descriptor.args_schema["x-a4n-schema-quality"], "typed-input",
                "{command}"
            );
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{command}"
            );
        }

        let delete = command_descriptor("delete_chapter_video_prompt").unwrap();
        assert_eq!(delete.args_schema["required"], serde_json::json!(["input"]));
        assert!(delete.args_schema["properties"].get("promptId").is_none());
        assert!(
            delete.args_schema["properties"]["input"]["required"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("downstream_strategy"))
        );
        assert!(delete.required_capabilities.is_empty());

        let update = command_descriptor("update_chapter_video_prompt").unwrap();
        let update_fields = &update.args_schema["properties"]["input"]["properties"];
        for field in [
            "duration_secs",
            "motion_prompt",
            "negative_prompt",
            "visual_prompt",
        ] {
            assert!(update_fields[field].get("default").is_none(), "{field}");
        }
        assert!(
            update.args_schema["properties"]["input"]["required"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("downstream_strategy"))
        );
        assert!(update.required_capabilities.is_empty());

        let optimize = command_descriptor("optimize_chapter_video_prompt").unwrap();
        assert!(
            optimize.args_schema["properties"]["input"]["required"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("downstream_strategy"))
        );
        assert_eq!(
            optimize.required_capabilities,
            vec![String::from("prompt"), String::from("llm")]
        );
    }

    #[test]
    fn chapter_shooting_script_descriptors_use_typed_canonical_contracts() {
        for command in [
            "edit_chapter_shooting_script",
            "generate_chapter_shooting_script",
            "generate_next_chapter_shooting_script_unit",
            "get_chapter_shooting_script",
            "get_chapter_shooting_script_unit_progress",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!(["userId"]),
                "{command}"
            );
            assert_eq!(
                descriptor.args_schema["x-a4n-schema-quality"], "typed-input",
                "{command}"
            );
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{command}"
            );
        }

        let get = command_descriptor("get_chapter_shooting_script").unwrap();
        let source = &get.args_schema["properties"]["input"];
        assert_eq!(source["properties"]["project_id"]["minimum"], 1);
        assert_eq!(source["properties"]["source_chapter_id"]["minimum"], 1);
        let record =
            &get.result_schema["$defs"]["ChapterShootingScriptRecordSummary"]["properties"];
        assert_eq!(record.as_object().unwrap().len(), 2);
        assert!(record.get("id").is_some());
        assert!(record.get("revision").is_some());

        let progress = command_descriptor("generate_next_chapter_shooting_script_unit").unwrap();
        assert_eq!(progress.execution, "workflow");
        assert_eq!(progress.idempotency, "server_workflow");
        assert_eq!(
            progress.required_capabilities,
            vec![String::from("prompt"), String::from("llm")]
        );
        assert!(progress.args_schema["properties"].get("taskId").is_some());
        let progress_fields = &progress.result_schema["properties"];
        assert!(progress_fields.get("completed_record").is_some());
        assert!(progress_fields.get("shooting_script").is_none());
        assert!(progress.result_references.is_empty());

        let edit = command_descriptor("edit_chapter_shooting_script").unwrap();
        assert_eq!(edit.result_references, vec![String::from("project_ids")]);
    }

    #[test]
    fn novel_export_descriptors_match_inline_host_contracts() {
        let commands = [
            "export_batch_data",
            "export_characters_json_data",
            "export_epub_data",
            "export_markdown_data",
            "export_outline_json_data",
            "export_pdf_data",
            "export_txt_data",
        ];

        for command in commands {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(descriptor.domain, "export", "{command}");
            assert_eq!(descriptor.execution, "download", "{command}");
            assert_eq!(
                descriptor.args_schema["x-a4n-schema-quality"],
                "typed-input"
            );
            assert_eq!(
                descriptor.args_schema["required"],
                serde_json::json!(["input", "taskId"]),
                "{command}"
            );
            assert_eq!(descriptor.result_schema["x-a4n-schema-quality"], "typed");
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([])
            );
            assert!(descriptor.required_capabilities.is_empty(), "{command}");
            assert!(descriptor.result_references.is_empty(), "{command}");
            assert!(
                descriptor.args_schema["properties"]
                    .get("projectId")
                    .is_none()
            );
            assert!(descriptor.args_schema["properties"].get("title").is_none());
            assert!(descriptor.args_schema["properties"].get("author").is_none());
        }

        let batch = command_descriptor("export_batch_data").unwrap();
        assert_eq!(batch.result_schema["type"], "array");
        assert_eq!(
            batch.args_schema["properties"]["input"]["$defs"]["BatchExportFormat"]["enum"],
            serde_json::json!(["txt", "markdown", "epub", "json", "pdf"])
        );
        assert!(
            batch.args_schema["properties"]["input"]["properties"]
                .get("title")
                .is_none()
        );
        assert!(
            batch.args_schema["properties"]["input"]["properties"]
                .get("author")
                .is_none()
        );
    }
}
