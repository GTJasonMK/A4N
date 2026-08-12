use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const COMMAND_MANIFEST_VERSION: u32 = 3;
pub const HOST_IDENTITY: &str = "a4n-server";
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

type SchemaFactory = fn() -> Value;

#[derive(Clone, Copy)]
struct Identity {
    auth: &'static str,
    context_fields: &'static [&'static str],
}

#[derive(Clone, Copy)]
struct Effect {
    effect: &'static str,
    idempotency: &'static str,
}

#[derive(Clone, Copy)]
struct CommandSpec {
    name: &'static str,
    args_schema: SchemaFactory,
    result_schema: SchemaFactory,
    auth: &'static str,
    domain: &'static str,
    effect: &'static str,
    execution: &'static str,
    context_fields: &'static [&'static str],
    secret_pointers: &'static [&'static str],
    file_inputs: &'static [&'static str],
    file_outputs: &'static [&'static str],
    result_references: &'static [&'static str],
    required_capabilities: &'static [&'static str],
    runtime_config_policy: RuntimeConfigPolicy,
    idempotency: &'static str,
}

impl CommandSpec {
    const fn new(
        name: &'static str,
        args_schema: SchemaFactory,
        result_schema: SchemaFactory,
        domain: &'static str,
    ) -> Self {
        Self {
            name,
            args_schema,
            result_schema,
            auth: "user",
            domain,
            effect: "write",
            execution: "immediate",
            context_fields: USER_CONTEXT,
            secret_pointers: NO_CONTEXT,
            file_inputs: NO_CONTEXT,
            file_outputs: NO_CONTEXT,
            result_references: NO_CONTEXT,
            required_capabilities: NO_CONTEXT,
            runtime_config_policy: RuntimeConfigPolicy::None,
            idempotency: "none",
        }
    }

    const fn identity(mut self, identity: Identity) -> Self {
        self.auth = identity.auth;
        self.context_fields = identity.context_fields;
        self
    }

    const fn effect(mut self, effect: Effect) -> Self {
        self.effect = effect.effect;
        self.idempotency = effect.idempotency;
        self
    }

    const fn execution(mut self, execution: &'static str) -> Self {
        self.execution = execution;
        self
    }

    const fn context(mut self, fields: &'static [&'static str]) -> Self {
        self.context_fields = fields;
        self
    }

    const fn secrets(mut self, values: &'static [&'static str]) -> Self {
        self.secret_pointers = values;
        self
    }

    const fn file_inputs(mut self, values: &'static [&'static str]) -> Self {
        self.file_inputs = values;
        self
    }

    const fn file_outputs(mut self, values: &'static [&'static str]) -> Self {
        self.file_outputs = values;
        self
    }

    const fn references(mut self, values: &'static [&'static str]) -> Self {
        self.result_references = values;
        self
    }

    const fn capabilities(mut self, values: &'static [&'static str]) -> Self {
        self.required_capabilities = values;
        self
    }

    const fn runtime(mut self, policy: RuntimeConfigPolicy) -> Self {
        self.runtime_config_policy = policy;
        self
    }

    const fn idempotency(mut self, value: &'static str) -> Self {
        self.idempotency = value;
        self
    }
}

const NO_CONTEXT: &[&str] = &[];
const USER_CONTEXT: &[&str] = &["userId"];
const CALLER_CONTEXT: &[&str] = &["callerUserId"];
const PROJECT_CONTEXT: &[&str] = &["projectId"];
const CHAPTER_CONTEXT: &[&str] = &["chapterId"];
const PROJECT_CHAPTER_CONTEXT: &[&str] = &["projectId", "chapterId"];
const USER_PROJECT_CONTEXT: &[&str] = &["userId", "projectId"];
const USER_CHAPTER_CONTEXT: &[&str] = &["userId", "chapterId"];

const PUBLIC_IDENTITY: Identity = Identity {
    auth: "public",
    context_fields: NO_CONTEXT,
};
const ADMIN_IDENTITY: Identity = Identity {
    auth: "admin",
    context_fields: CALLER_CONTEXT,
};
const READ_EFFECT: Effect = Effect {
    effect: "read",
    idempotency: "safe",
};
const DESTRUCTIVE_EFFECT: Effect = Effect {
    effect: "destructive",
    idempotency: "none",
};
const CAPABILITIES_EMBEDDING: &[&str] = &["embedding"];
const CAPABILITIES_EMBEDDING_RAG: &[&str] = &["embedding", "rag"];
const CAPABILITIES_EXPORT: &[&str] = &["export"];
const CAPABILITIES_IMAGE: &[&str] = &["image"];
const CAPABILITIES_IMPORT: &[&str] = &["import"];
const CAPABILITIES_LLM: &[&str] = &["llm"];
const CAPABILITIES_PROMPT: &[&str] = &["prompt"];
const CAPABILITIES_PROMPT_IMAGE: &[&str] = &["prompt", "image"];
const CAPABILITIES_PROMPT_LLM: &[&str] = &["prompt", "llm"];
const CAPABILITIES_RAG: &[&str] = &["rag"];
const CAPABILITIES_VIDEO: &[&str] = &["video"];
const CAPABILITIES_VIDEO_EXPORT: &[&str] = &["video", "export"];
const CAPABILITIES_VIDEO_IMPORT: &[&str] = &["video", "import"];
const FILE_INPUTS_SET_BASE64_SET_TEXT: &[&str] =
    &["使用 --set-base64/--set-text 将本地文件写入实际输入字段"];
const FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT: &[&str] =
    &["使用 --save-base64/--save-text 原子保存响应字段"];
const REFERENCES_ARTIFACTS: &[&str] = &["artifacts"];
const REFERENCES_CHAPTER_IDS: &[&str] = &["chapter_ids"];
const REFERENCES_CHAPTER_IDS_TASK_IDS: &[&str] = &["chapter_ids", "task_ids"];
const REFERENCES_GALGAME_CHOICE_IDS: &[&str] = &["galgame_choice_ids"];
const REFERENCES_GALGAME_LINE_IDS: &[&str] = &["galgame_line_ids"];
const REFERENCES_GALGAME_SCENE_IDS: &[&str] = &["galgame_scene_ids"];
const REFERENCES_GALGAME_SCRIPT_IDS: &[&str] = &["galgame_script_ids"];
const REFERENCES_GENERATION_SPEC_IDS: &[&str] = &["generation_spec_ids"];
const REFERENCES_GENERATION_SPEC_IDS_MEDIA_ASSET_IDS: &[&str] =
    &["generation_spec_ids", "media_asset_ids"];
const REFERENCES_MEDIA_BINDING_IDS_MEDIA_ASSET_IDS: &[&str] =
    &["media_binding_ids", "media_asset_ids"];
const REFERENCES_PROJECT_IDS: &[&str] = &["project_ids"];
const REFERENCES_TASK_IDS: &[&str] = &["task_ids"];
const REFERENCES_WORKFLOW_RUN_IDS: &[&str] = &["workflow_run_ids"];
const PROVIDER_SECRET_POINTERS: &[&str] = &["/input/api_key_encrypted", "/input/api_key"];
const OAUTH_CODE_SECRET_POINTERS: &[&str] = &["/input/code"];
const CHANGE_PASSWORD_SECRET_POINTERS: &[&str] = &["/input/old_password", "/input/new_password"];
const PASSWORD_SECRET_POINTERS: &[&str] = &["/input/password"];
const REFRESH_TOKEN_SECRET_POINTERS: &[&str] = &["/input/refresh_token"];
const RUNTIME_SECRET_POINTERS: &[&str] = &[
    "/runtime_config_snapshot/llm/api_key_encrypted",
    "/runtime_config_snapshot/image/api_key_encrypted",
    "/runtime_config_snapshot/video/api_key_encrypted",
    "/runtime_config_snapshot/embedding/api_key_encrypted",
    "/runtime_config_snapshot/system_llm_api_key_encrypted",
    "/runtime_config_snapshot/system_image_api_key_encrypted",
    "/runtime_config_snapshot/system_video_api_key_encrypted",
];

macro_rules! command_specs {
    ($($name:literal => $args:ident, $result:ident, $domain:literal $(, $method:ident($value:expr))*;)+) => {
        pub const API_COMMANDS: &[&str] = &[$($name,)+];
        const COMMAND_SPECS: &[CommandSpec] = &[
            $(CommandSpec::new($name, $args, $result, $domain)$(.$method($value))*,)+
        ];
    };
}

command_specs! {
    "accept_import_review" => args_accept_import_review, result_import_review, "import", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT), runtime(RuntimeConfigPolicy::Full);
    "accept_project_invite" => args_invite_id, result_unit, "project", context(NO_CONTEXT);
    "activate_theme" => args_activate_theme, result_activate_theme, "settings";
    "add_image_config" => args_add_image_config, result_add_image_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_IMAGE);
    "add_profile_attribute" => args_add_profile_attribute, result_unit, "profile";
    "add_video_config" => args_add_video_config, result_add_video_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_VIDEO);
    "apply_assembly_postprocess_edit" => args_apply_assembly_postprocess_edit, result_opaque, "system", effect(DESTRUCTIVE_EFFECT);
    "apply_implicit_update_decision" => args_apply_implicit_update_decision, result_apply_implicit_update_decision, "profile";
    "apply_optimization_patches" => args_opaque, result_opaque, "system", effect(DESTRUCTIVE_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "apply_source_sync_decisions" => args_opaque, result_opaque, "source-sync", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "assemble_chapter_videos" => args_opaque, result_opaque, "video", context(USER_CHAPTER_CONTEXT), capabilities(CAPABILITIES_VIDEO);
    "attribute_history" => args_attribute_history, result_attribute_history, "profile", effect(READ_EFFECT);
    "batch_accept_reviews" => args_batch_accept_reviews, result_import_reviews, "import", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT), runtime(RuntimeConfigPolicy::Full);
    "bind_galgame_media" => args_bind_galgame_media, result_bind_galgame_media, "galgame", context(NO_CONTEXT), references(REFERENCES_MEDIA_BINDING_IDS_MEDIA_ASSET_IDS);
    "bind_manga_reference_image" => args_bind_manga_reference_image, result_bind_manga_reference_image, "manga", context(NO_CONTEXT);
    "branch_project" => args_opaque, result_opaque, "project", context(PROJECT_CONTEXT), references(REFERENCES_PROJECT_IDS);
    "cancel_admin_task" => args_opaque, result_opaque, "task", identity(ADMIN_IDENTITY), context(USER_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "cancel_embedding_model_download" => args_cancel_embedding_model_download, result_unit, "rag", effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_EMBEDDING);
    "cancel_galgame_pipeline" => args_cancel_galgame_pipeline, result_cancel_galgame_pipeline, "galgame", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), execution("workflow"), references(REFERENCES_WORKFLOW_RUN_IDS), idempotency("server_workflow");
    "cancel_generation_task" => args_cancel_generation_task, result_generation_task_handle, "task", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "cancel_import_analysis" => args_project_id, result_unit, "import", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "cancel_manga_pipeline" => args_cancel_manga_pipeline, result_cancel_manga_pipeline, "manga", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), execution("workflow"), idempotency("server_workflow");
    "cancel_task" => args_cancel_task, result_unit, "task", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "cancel_workflow" => args_cancel_workflow, result_unit, "workflow", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), execution("workflow"), references(REFERENCES_WORKFLOW_RUN_IDS), idempotency("server_workflow");
    "change_user_password" => args_change_user_password, result_unit, "auth", context(NO_CONTEXT), secrets(CHANGE_PASSWORD_SECRET_POINTERS);
    "check_character_conflict" => args_opaque, result_opaque, "profile", effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "check_implicit_update" => args_check_implicit_update, result_check_implicit_update, "profile", effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "check_import_dedup" => args_check_import_dedup, result_dedup, "import", context(NO_CONTEXT), effect(READ_EFFECT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "check_profile_sync_conflict" => args_check_profile_sync_conflict, result_check_profile_sync_conflict, "profile", effect(READ_EFFECT);
    "cleanup_orphan_project_data_dirs" => args_cleanup_orphan_project_data_dirs, result_cleanup_orphan_project_data_dirs, "project", identity(ADMIN_IDENTITY), context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), references(REFERENCES_PROJECT_IDS);
    "confirm_reverse_engineering_candidate" => args_opaque, result_opaque, "workflow", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_WORKFLOW_RUN_IDS), runtime(RuntimeConfigPolicy::Full), idempotency("safe");
    "create_derived_project" => args_create_derived_project, result_opaque, "project", context(NO_CONTEXT), references(REFERENCES_PROJECT_IDS);
    "create_embedding_config" => args_create_embedding_config, result_create_embedding_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_EMBEDDING);
    "create_foreshadowing" => args_create_foreshadowing, result_foreshadowing_write, "profile", context(USER_CHAPTER_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "create_freeform_video_project" => args_create_freeform_video_project, result_opaque, "project", context(NO_CONTEXT), references(REFERENCES_PROJECT_IDS), capabilities(CAPABILITIES_VIDEO);
    "create_invite_code" => args_opaque, result_opaque, "collaboration", identity(ADMIN_IDENTITY);
    "create_llm_config" => args_create_llm_config, result_create_llm_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS);
    "create_object_profile" => args_create_object_profile, result_object_profile_mutation, "profile";
    "create_optimization_analysis_session" => args_opaque, result_opaque, "auth", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "create_optimization_chat_session" => args_opaque, result_opaque, "auth", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "create_profile" => args_create_profile, result_create_profile, "profile", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "create_project" => args_create_project, result_create_project, "project", context(NO_CONTEXT), references(REFERENCES_PROJECT_IDS);
    "create_scene_profile" => args_create_scene_profile, result_scene_profile_mutation, "profile", context(USER_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "create_theme" => args_create_theme, result_activate_theme, "settings";
    "create_user" => args_create_user, result_opaque, "auth", identity(ADMIN_IDENTITY), secrets(PASSWORD_SECRET_POINTERS);
    "decline_project_invite" => args_invite_id, result_unit, "project", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_chapter_scene" => args_delete_chapter_scene, result_unit, "chapter", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_chapter_video_prompt" => args_delete_chapter_video_prompt, result_unit, "video", effect(DESTRUCTIVE_EFFECT);
    "delete_embedding_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_EMBEDDING);
    "delete_foreshadowing" => args_delete_foreshadowing, result_foreshadowing_write, "profile", context(USER_CHAPTER_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_galgame_line" => args_delete_galgame_line, result_unit, "galgame", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_image" => args_delete_image, result_unit, "asset", effect(DESTRUCTIVE_EFFECT);
    "delete_import_review" => args_import_review_id, result_unit, "import", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "delete_latest_chapter_outlines" => args_delete_latest_outline_records, result_opaque, "chapter", context(USER_CHAPTER_CONTEXT), effect(DESTRUCTIVE_EFFECT), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::Full);
    "delete_latest_parts" => args_delete_latest_outline_records, result_opaque, "outline", effect(DESTRUCTIVE_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "delete_llm_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_local_embedding_model" => args_delete_local_embedding_model, result_unit, "rag", effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_EMBEDDING);
    "delete_manga_by_chapter" => args_cancel_manga_pipeline, result_unit, "manga", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_manga_prompt" => args_delete_manga_prompt, result_unit, "manga", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "delete_object_profile" => args_delete_object_profile, result_object_profile_delete, "profile", effect(DESTRUCTIVE_EFFECT);
    "delete_optimize_session" => args_opaque, result_opaque, "auth", effect(DESTRUCTIVE_EFFECT);
    "delete_profile" => args_delete_profile, result_delete_profile, "profile", effect(DESTRUCTIVE_EFFECT);
    "delete_project" => args_project_id, result_opaque, "project", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT), references(REFERENCES_PROJECT_IDS);
    "delete_scene_profile" => args_delete_scene_profile, result_scene_profile_delete, "profile", context(USER_CONTEXT), effect(DESTRUCTIVE_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "delete_theme" => args_activate_theme, result_unit, "settings", effect(DESTRUCTIVE_EFFECT);
    "delete_video" => args_delete_video, result_unit, "video", effect(DESTRUCTIVE_EFFECT);
    "detach_derived_project" => args_opaque, result_opaque, "source-sync", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "detect_image_config_models" => args_detect_image_config_models, result_detect_image_config_models, "settings", context(USER_CONTEXT), effect(READ_EFFECT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_IMAGE);
    "detect_llm_config_models" => args_detect_image_config_models, result_detect_image_config_models, "settings", context(USER_CONTEXT), effect(READ_EFFECT), secrets(PROVIDER_SECRET_POINTERS);
    "detect_video_config_models" => args_detect_image_config_models, result_detect_image_config_models, "settings", context(USER_CONTEXT), effect(READ_EFFECT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_VIDEO);
    "diff_blueprint_versions" => args_diff_blueprint_versions, result_diff_blueprint_versions, "blueprint", effect(READ_EFFECT);
    "diff_profile_chapters" => args_diff_profile_chapters, result_diff_profile_chapters, "profile", effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "disable_user" => args_opaque, result_opaque, "auth", identity(ADMIN_IDENTITY), effect(DESTRUCTIVE_EFFECT);
    "discover_character_profiles" => args_opaque, result_opaque, "profile", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "download_embedding_model" => args_download_embedding_model, result_download_embedding_model, "rag", execution("task"), capabilities(CAPABILITIES_EMBEDDING);
    "duplicate_project" => args_duplicate_project, result_opaque, "project", context(NO_CONTEXT), references(REFERENCES_PROJECT_IDS);
    "edit_chapter_shooting_script" => args_edit_chapter_shooting_script, result_edit_chapter_shooting_script, "chapter", references(REFERENCES_PROJECT_IDS);
    "execute_attribute_deletion" => args_execute_attribute_deletion, result_unit, "profile", effect(DESTRUCTIVE_EFFECT);
    "export_all_configs_json" => args_export_all_configs_json, result_export_all_configs_json, "export", identity(ADMIN_IDENTITY), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT), references(REFERENCES_ARTIFACTS);
    "export_batch_data" => args_export_batch_data, result_export_batch_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT);
    "export_characters_json_data" => args_export_characters_json_data, result_export_characters_json_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT);
    "export_derived_project_data" => args_export_derived_project_data, result_export_project_package_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT), references(REFERENCES_ARTIFACTS), capabilities(CAPABILITIES_EXPORT);
    "export_epub_data" => args_export_epub_data, result_export_characters_json_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT);
    "export_galgame_webgal" => args_cancel_galgame_pipeline, result_export_characters_json_data, "galgame", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT), references(REFERENCES_ARTIFACTS), capabilities(CAPABILITIES_EXPORT);
    "export_markdown_data" => args_export_epub_data, result_export_characters_json_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT);
    "export_outline_json_data" => args_export_characters_json_data, result_export_characters_json_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT);
    "export_pdf_data" => args_export_characters_json_data, result_export_characters_json_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT);
    "export_project_family_data" => args_export_project_family_data, result_export_project_package_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT), references(REFERENCES_ARTIFACTS), capabilities(CAPABILITIES_EXPORT);
    "export_theme" => args_activate_theme, result_export_theme, "export", effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT), capabilities(CAPABILITIES_EXPORT);
    "export_txt_data" => args_export_characters_json_data, result_export_characters_json_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT);
    "export_video_studio_project_data" => args_export_video_studio_project_data, result_export_project_package_data, "export", context(NO_CONTEXT), effect(READ_EFFECT), execution("download"), file_outputs(FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT), references(REFERENCES_ARTIFACTS), capabilities(CAPABILITIES_VIDEO_EXPORT);
    "forward_project_status" => args_forward_project_status, result_create_project, "project", context(PROJECT_CONTEXT), references(REFERENCES_PROJECT_IDS);
    "generate_chapter_analysis" => args_chapter_analysis_task, result_chapter_analysis_mutation, "chapter", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "generate_chapter_scene" => args_generate_chapter_scene, result_chapter_scene, "chapter", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "generate_chapter_shooting_script" => args_generate_chapter_shooting_script, result_generate_chapter_shooting_script, "chapter", execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("server_workflow");
    "generate_next_chapter_shooting_script_unit" => args_generate_chapter_shooting_script, result_generate_next_chapter_shooting_script_unit, "chapter", execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("server_workflow");
    "generate_object_profile_image_prompt" => args_generate_object_profile_image_prompt, result_object_profile_image_prompt, "profile", effect(READ_EFFECT);
    "generate_portrait_prompt" => args_opaque, result_opaque, "profile", execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_PROMPT_IMAGE), runtime(RuntimeConfigPolicy::Full);
    "generate_scene_profile_image_prompt" => args_generate_scene_profile_image_prompt, result_scene_profile_image_prompt, "profile", effect(READ_EFFECT);
    "get_admin_overview" => args_opaque, result_get_admin_overview, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "get_blueprint" => args_opaque, result_get_blueprint, "blueprint", effect(READ_EFFECT);
    "get_canvas_graph" => args_get_canvas_graph, result_get_canvas_graph, "canvas", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_canvas_node_contracts" => args_get_canvas_node_contracts, result_get_canvas_node_contracts, "canvas", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_chapter_outlines_pacing" => args_opaque, result_opaque, "chapter", context(USER_CHAPTER_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "get_chapter_shooting_script" => args_get_chapter_shooting_script, result_get_chapter_shooting_script, "chapter", effect(READ_EFFECT);
    "get_chapter_shooting_script_unit_progress" => args_get_chapter_shooting_script, result_get_chapter_shooting_script_unit_progress, "chapter", effect(READ_EFFECT);
    "get_character_arc_status" => args_opaque, result_opaque, "profile", effect(READ_EFFECT);
    "get_character_voice_fingerprint" => args_opaque, result_opaque, "profile", effect(READ_EFFECT);
    "get_current_user" => empty_args_schema, result_get_current_user, "auth", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_embedding_model_download_info" => args_delete_local_embedding_model, result_get_embedding_model_download_info, "rag", effect(READ_EFFECT), capabilities(CAPABILITIES_EMBEDDING);
    "get_galgame_pipeline_status" => args_cancel_galgame_pipeline, result_cancel_galgame_pipeline, "galgame", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow"), references(REFERENCES_WORKFLOW_RUN_IDS);
    "get_galgame_script" => args_cancel_galgame_pipeline, result_get_galgame_script, "galgame", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_GALGAME_SCRIPT_IDS);
    "get_generation_call_analytics" => args_get_generation_call_analytics, result_get_generation_call_analytics, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "get_generation_progress" => args_opaque, result_opaque, "task", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_implicit_review" => args_get_implicit_review, result_get_implicit_review, "profile", effect(READ_EFFECT);
    "get_inspiration_history" => args_opaque, result_opaque, "inspiration", effect(READ_EFFECT);
    "get_llm_max_tokens_config" => args_get_llm_max_tokens_config, result_get_llm_max_tokens_config, "settings", effect(READ_EFFECT);
    "get_log_dir" => args_opaque, result_export_theme, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "get_manga_pipeline_status" => args_cancel_manga_pipeline, result_get_manga_pipeline_status, "manga", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow");
    "get_optimize_session" => args_opaque, result_opaque, "auth", effect(READ_EFFECT);
    "get_primary_portrait" => args_opaque, result_opaque, "profile", effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE);
    "get_profile_state_at_chapter" => args_get_profile_state_at_chapter, result_get_profile_state_at_chapter, "profile", effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "get_project" => args_project_id, result_create_project, "project", context(PROJECT_CONTEXT), effect(READ_EFFECT), references(REFERENCES_PROJECT_IDS);
    "get_project_asset_dependency_graph" => args_project_id, result_get_project_asset_dependency_graph, "project", context(PROJECT_CONTEXT), effect(READ_EFFECT), references(REFERENCES_PROJECT_IDS);
    "get_project_asset_dependency_graph_node_detail" => args_project_asset_dependency_graph_node_detail, result_get_project_asset_dependency_graph_node_detail, "project", context(PROJECT_CONTEXT), effect(READ_EFFECT), references(REFERENCES_PROJECT_IDS);
    "get_project_import_analysis_progress" => args_project_id, result_opaque, "import", context(PROJECT_CONTEXT), effect(READ_EFFECT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "get_project_status_machine" => args_project_id, result_get_project_status_machine, "project", context(PROJECT_CONTEXT), effect(READ_EFFECT);
    "get_queue_stats" => empty_args_schema, result_get_queue_stats, "system", context(NO_CONTEXT), effect(READ_EFFECT);
    "get_snapshot" => args_get_snapshot, result_get_snapshot, "profile", effect(READ_EFFECT);
    "get_storage_paths" => args_opaque, result_get_storage_paths, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "get_task_history" => args_get_task_history, result_get_task_history, "task", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_TASK_IDS);
    "get_token_summary" => args_opaque, result_opaque, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "get_usage_trends" => args_get_usage_trends, result_get_usage_trends, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "get_user_usage" => args_opaque, result_get_user_usage, "auth", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "get_workflow_progress_events" => args_get_workflow_progress_events, result_get_workflow_progress_events, "workflow", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow"), references(REFERENCES_WORKFLOW_RUN_IDS);
    "get_workflow_run_detail" => args_get_workflow_run_detail, result_get_workflow_run_detail, "workflow", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow"), references(REFERENCES_WORKFLOW_RUN_IDS);
    "health_check" => args_opaque, result_opaque, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "import_all_configs" => args_import_all_configs, result_import_all_configs, "import", identity(ADMIN_IDENTITY), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT);
    "import_canvas_media" => args_import_canvas_media, result_import_canvas_media, "canvas", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "import_chapter" => args_import_chapter, result_chapter_summary, "import", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS), capabilities(CAPABILITIES_IMPORT), runtime(RuntimeConfigPolicy::Full);
    "import_galgame_audio" => args_import_galgame_audio, result_bind_galgame_media, "galgame", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), references(REFERENCES_MEDIA_BINDING_IDS_MEDIA_ASSET_IDS), capabilities(CAPABILITIES_IMPORT);
    "import_project_family_upload" => args_import_project_family_upload, result_project_summaries, "import", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "import_project_text_upload" => args_import_project_text_upload, result_import_reviews, "import", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "import_theme" => args_create_theme, result_activate_theme, "import", file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "import_video_studio_project_upload" => args_import_video_studio_project_upload, result_create_project, "import", context(NO_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_VIDEO_IMPORT);
    "insert_galgame_line" => args_insert_galgame_line, result_insert_galgame_line, "galgame", context(PROJECT_CONTEXT), references(REFERENCES_GALGAME_LINE_IDS);
    "invite_project_member" => args_invite_project_member, result_unit, "project", context(PROJECT_CONTEXT);
    "leave_project" => empty_args_schema, result_unit, "project", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "list_active_workflows" => empty_args_schema, result_list_active_workflows, "workflow", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow"), references(REFERENCES_TASK_IDS);
    "list_all_projects" => args_opaque, result_list_all_projects, "project", identity(ADMIN_IDENTITY), effect(READ_EFFECT), references(REFERENCES_PROJECT_IDS);
    "list_all_users" => args_opaque, result_opaque, "auth", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "list_behaviors" => args_list_behaviors, result_list_behaviors, "profile", effect(READ_EFFECT);
    "list_blueprint_versions" => args_list_blueprint_versions, result_list_blueprint_versions, "blueprint", effect(READ_EFFECT);
    "list_chapter_outlines" => args_opaque, result_opaque, "chapter", context(USER_CHAPTER_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "list_chapter_scenes" => args_list_chapter_scenes, result_chapter_scenes, "chapter", context(CHAPTER_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "list_chapter_video_prompts" => args_list_chapter_video_prompts, result_list_chapter_video_prompts, "video", context(PROJECT_CHAPTER_CONTEXT), effect(READ_EFFECT);
    "list_chapters" => args_project_id, result_chapter_summaries, "chapter", context(PROJECT_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "list_deletion_marks" => args_list_deletion_marks, result_list_deletion_marks, "profile", effect(READ_EFFECT);
    "list_embedding_config" => args_get_llm_max_tokens_config, result_list_embedding_config, "settings", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_EMBEDDING);
    "list_foreshadowing" => args_list_foreshadowing, result_list_foreshadowing, "profile", context(USER_CHAPTER_CONTEXT), effect(READ_EFFECT);
    "list_image_configs" => args_get_llm_max_tokens_config, result_list_image_configs, "settings", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE);
    "list_images_by_chapter" => args_list_images_by_chapter, result_list_images_by_chapter, "chapter", context(CHAPTER_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "list_images_by_project" => args_project_id, result_list_images_by_chapter, "asset", context(PROJECT_CONTEXT), effect(READ_EFFECT);
    "list_import_queue" => args_import_project_reviews, result_import_reviews, "import", context(NO_CONTEXT), effect(READ_EFFECT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "list_invite_codes" => args_opaque, result_opaque, "collaboration", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "list_llm_config" => args_get_llm_max_tokens_config, result_list_llm_config, "settings", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_local_embedding_models" => args_get_llm_max_tokens_config, result_list_local_embedding_models, "rag", effect(READ_EFFECT), capabilities(CAPABILITIES_EMBEDDING);
    "list_manga_page_prompts" => args_cancel_manga_pipeline, result_list_manga_page_prompts, "manga", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_model_provider_catalog" => args_opaque, result_list_model_provider_catalog, "system", effect(READ_EFFECT);
    "list_my_project_invites" => empty_args_schema, result_list_my_project_invites, "project", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_object_profiles" => args_list_object_profiles, result_list_object_profiles, "profile", effect(READ_EFFECT);
    "list_optimize_patch_sets" => args_opaque, result_opaque, "system", effect(READ_EFFECT);
    "list_optimize_sessions" => args_opaque, result_opaque, "auth", effect(READ_EFFECT);
    "list_optimize_turns" => args_opaque, result_opaque, "system", effect(READ_EFFECT);
    "list_orphan_project_data_dirs" => args_opaque, result_cleanup_orphan_project_data_dirs, "project", identity(ADMIN_IDENTITY), effect(READ_EFFECT), references(REFERENCES_PROJECT_IDS);
    "list_parts" => args_opaque, result_opaque, "outline", effect(READ_EFFECT);
    "list_pending_tasks" => args_opaque, result_list_pending_tasks, "task", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "list_portraits_by_project" => args_opaque, result_opaque, "profile", context(PROJECT_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE);
    "list_profile_materials" => args_list_profile_materials, result_list_profile_materials, "profile", effect(READ_EFFECT);
    "list_profiles" => args_list_profiles, result_list_profiles, "profile", effect(READ_EFFECT);
    "list_project_invites" => empty_args_schema, result_list_my_project_invites, "project", context(PROJECT_CONTEXT), effect(READ_EFFECT);
    "list_project_members" => empty_args_schema, result_list_project_members, "project", context(PROJECT_CONTEXT), effect(READ_EFFECT);
    "list_projects" => args_opaque, result_list_projects, "project", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_PROJECT_IDS);
    "list_scene_profiles" => args_list_scene_profiles, result_list_scene_profiles, "profile", effect(READ_EFFECT);
    "list_sessions" => empty_args_schema, result_list_sessions, "auth", context(NO_CONTEXT), effect(READ_EFFECT);
    "list_snapshots" => args_list_snapshots, result_list_snapshots, "profile", effect(READ_EFFECT);
    "list_source_snapshot_chapters" => args_project_id, result_opaque, "chapter", context(PROJECT_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "list_themes" => args_get_llm_max_tokens_config, result_list_themes, "settings", effect(READ_EFFECT);
    "list_video_configs" => args_get_llm_max_tokens_config, result_list_video_configs, "settings", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_VIDEO);
    "list_video_reference_images" => args_list_video_reference_images, result_list_images_by_chapter, "asset", context(PROJECT_CHAPTER_CONTEXT), effect(READ_EFFECT);
    "list_videos_by_chapter" => args_list_chapter_video_prompts, result_list_videos_by_chapter, "video", context(CHAPTER_CONTEXT), effect(READ_EFFECT);
    "list_workflow_runs" => args_list_workflow_runs, result_list_workflow_runs, "workflow", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow"), references(REFERENCES_WORKFLOW_RUN_IDS);
    "load_cached_assembled_chapter_video_preview" => args_opaque, result_opaque, "video", context(USER_CHAPTER_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_VIDEO);
    "load_video_playback_data" => args_opaque, result_opaque, "video", effect(READ_EFFECT), capabilities(CAPABILITIES_VIDEO);
    "load_video_scrub_playback_data" => args_opaque, result_opaque, "video", effect(READ_EFFECT), capabilities(CAPABILITIES_VIDEO);
    "load_video_segment_plan_draft" => args_opaque, result_opaque, "video", effect(READ_EFFECT), capabilities(CAPABILITIES_VIDEO);
    "login_user" => args_login_user, result_login_user, "auth", identity(PUBLIC_IDENTITY), secrets(PASSWORD_SECRET_POINTERS);
    "modify_profile_attribute" => args_modify_profile_attribute, result_unit, "profile";
    "oauth2_authorize_url" => args_opaque, result_opaque, "auth", identity(PUBLIC_IDENTITY);
    "oauth2_callback" => args_opaque, result_login_user, "auth", identity(PUBLIC_IDENTITY), secrets(OAUTH_CODE_SECRET_POINTERS);
    "optimize_chapter_video_prompt" => args_optimize_chapter_video_prompt, result_optimize_chapter_video_prompt, "video", secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full);
    "optimize_manga_prompt" => args_optimize_manga_prompt, result_bind_manga_reference_image, "manga", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full);
    "plan_galgame_assets" => args_cancel_galgame_pipeline, result_plan_galgame_assets, "galgame", context(NO_CONTEXT), references(REFERENCES_GENERATION_SPEC_IDS), idempotency("safe");
    "poll_video_job" => args_opaque, result_opaque, "video", effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_VIDEO), runtime(RuntimeConfigPolicy::Full);
    "prepare_chapter_generation_rag_embedding" => args_opaque, result_opaque, "rag", context(USER_CHAPTER_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_EMBEDDING_RAG), runtime(RuntimeConfigPolicy::Full);
    "prepare_optimization_analysis_session" => args_opaque, result_opaque, "auth", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "preview_apply_beat_mapping_request" => args_opaque, result_opaque, "system", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_batch_generate_part_outlines_request" => args_opaque, result_opaque, "outline", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_batch_images_request" => args_opaque, result_opaque, "asset", effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_canvas_node_request" => args_preview_canvas_node_request, result_preview_canvas_node_request, "canvas", context(NO_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_chapter_analysis_request" => args_chapter_analysis, result_model_request_preview, "chapter", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_chapter_prompt" => args_opaque, result_opaque, "chapter", context(USER_CHAPTER_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS), capabilities(CAPABILITIES_PROMPT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_chapter_scene_request" => args_preview_chapter_scene_request, result_model_request_preview, "chapter", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_chapter_video_request" => args_preview_chapter_video_request, result_opaque, "video", context(USER_CHAPTER_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_VIDEO), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_check_character_conflict_request" => args_opaque, result_opaque, "profile", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_check_implicit_update_request" => args_check_implicit_update, result_model_request_preview, "profile", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_continue_part_outlines_request" => args_opaque, result_opaque, "outline", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_create_optimization_analysis_session_request" => args_opaque, result_opaque, "auth", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_discover_character_profiles_request" => args_opaque, result_opaque, "profile", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_galgame_pipeline_request" => args_preview_galgame_pipeline_request, result_model_request_preview, "galgame", context(NO_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_age_variant_request" => args_opaque, result_opaque, "system", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_blueprint_request" => args_preview_generate_blueprint_request, result_opaque, "blueprint", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_chapter_outlines_by_count_request" => args_opaque, result_opaque, "chapter", context(USER_CHAPTER_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_missing_portraits_request" => args_opaque, result_opaque, "profile", effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_part_chapter_outlines_request" => args_opaque, result_opaque, "chapter", context(USER_CHAPTER_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_parts_request" => args_opaque, result_opaque, "outline", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_portrait_prompt_request" => args_opaque, result_opaque, "prompt", effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_generate_portrait_request" => args_opaque, result_opaque, "profile", effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_import_project_analysis_request" => args_project_id, result_opaque, "import", context(PROJECT_CONTEXT), effect(READ_EFFECT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_inspiration_request" => args_opaque, result_opaque, "inspiration", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_object_profile_image_request" => args_preview_object_profile_image_request, result_model_request_preview, "profile", effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_page_image_request" => args_opaque, result_opaque, "asset", effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_prepare_optimization_analysis_session_request" => args_opaque, result_opaque, "auth", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_propose_optimization_patches_request" => args_opaque, result_opaque, "system", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_reference_media_video_request" => args_opaque, result_opaque, "video", effect(READ_EFFECT), capabilities(CAPABILITIES_VIDEO), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_refine_blueprint_request" => args_preview_refine_blueprint_request, result_opaque, "blueprint", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_regenerate_chapter_outline_request" => args_opaque, result_opaque, "chapter", context(USER_CHAPTER_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_regenerate_last_part_request" => args_opaque, result_opaque, "outline", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_regenerate_part_outline_request" => args_opaque, result_opaque, "outline", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_regenerate_parts_request" => args_opaque, result_opaque, "outline", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_regenerate_portrait_request" => args_opaque, result_opaque, "profile", effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_resume_chapter_analysis_sync_request" => args_chapter_analysis, result_model_request_preview, "chapter", context(NO_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_resume_manga_pipeline_request" => args_preview_resume_manga_pipeline_request, result_model_request_preview, "manga", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow"), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_reverse_engineer_blueprint_request" => args_opaque, result_opaque, "blueprint", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_reverse_engineer_outline_request" => args_opaque, result_opaque, "outline", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_reverse_engineer_part_outline_request" => args_opaque, result_opaque, "outline", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_rollback" => args_forward_project_status, result_preview_rollback, "project", context(PROJECT_CONTEXT), effect(READ_EFFECT);
    "preview_scene_image_request" => args_opaque, result_opaque, "profile", context(USER_CHAPTER_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_send_optimization_message_request" => args_opaque, result_opaque, "system", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_source_sync" => args_opaque, result_opaque, "source-sync", context(NO_CONTEXT), effect(READ_EFFECT);
    "preview_start_manga_pipeline_request" => args_preview_start_manga_pipeline_request, result_model_request_preview, "manga", context(NO_CONTEXT), effect(READ_EFFECT), execution("workflow"), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_sync_all_character_profiles_request" => args_preview_sync_all_character_profiles_request, result_model_request_preview, "profile", effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_sync_character_from_chapters_request" => args_preview_sync_character_from_chapters_request, result_model_request_preview, "chapter", context(USER_CHAPTER_CONTEXT), effect(READ_EFFECT), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_test_embedding_config_request" => args_preview_test_embedding_config_request, result_model_request_preview, "settings", context(USER_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_EMBEDDING), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_test_image_config_request" => args_preview_test_image_config_request, result_model_request_preview, "settings", context(USER_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_test_llm_config_request" => args_preview_test_image_config_request, result_model_request_preview, "settings", context(USER_CONTEXT), effect(READ_EFFECT), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "preview_video_provider_request_by_video_id" => args_opaque, result_opaque, "video", effect(READ_EFFECT), capabilities(CAPABILITIES_VIDEO), runtime(RuntimeConfigPolicy::WithoutSecrets);
    "prompt_list" => args_prompt_list, result_prompt_list, "prompt", effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_list_categories" => args_prompt_list, result_prompt_list_categories, "prompt", effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_list_project_overrides" => args_prompt_list_project_overrides, result_prompt_list_project_overrides, "prompt", context(USER_PROJECT_CONTEXT), effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_load_defaults" => args_prompt_list, result_prompt_load_defaults, "prompt", capabilities(CAPABILITIES_PROMPT);
    "prompt_remove_project_override" => args_prompt_remove_project_override, result_prompt_remove_project_override, "prompt", context(USER_PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_render" => args_prompt_render, result_export_theme, "prompt", effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_render_ab" => args_prompt_render, result_prompt_render_ab, "prompt", effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_reset" => args_prompt_name, result_prompt_remove_project_override, "prompt", effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_set_project_override" => args_prompt_set_project_override, result_prompt_remove_project_override, "prompt", context(USER_PROJECT_CONTEXT), capabilities(CAPABILITIES_PROMPT);
    "prompt_update" => args_prompt_update, result_prompt_update, "prompt", capabilities(CAPABILITIES_PROMPT);
    "prompt_validate_variables" => args_prompt_validate_variables, result_prompt_validate_variables, "prompt", effect(READ_EFFECT), capabilities(CAPABILITIES_PROMPT);
    "prompt_version_diff" => args_prompt_version_diff, result_export_theme, "prompt", capabilities(CAPABILITIES_PROMPT);
    "prompt_version_history" => args_prompt_name, result_prompt_version_history, "prompt", capabilities(CAPABILITIES_PROMPT);
    "push_assembled_chapter_video" => args_push_assembled_chapter_video, result_opaque, "video", context(USER_CHAPTER_CONTEXT), capabilities(CAPABILITIES_VIDEO);
    "rag_enhanced_search" => args_opaque, result_opaque, "rag", secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_RAG), runtime(RuntimeConfigPolicy::Full);
    "rag_enhanced_search_with_vectors" => args_opaque, result_opaque, "rag", secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_RAG), runtime(RuntimeConfigPolicy::Full);
    "rag_ingest_all" => args_opaque, result_opaque, "rag", secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_RAG), runtime(RuntimeConfigPolicy::Full);
    "rag_prepare_enhanced_search" => args_opaque, result_opaque, "rag", secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_RAG), runtime(RuntimeConfigPolicy::Full);
    "rag_prepare_ingest_all" => args_opaque, result_opaque, "rag", secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_RAG), runtime(RuntimeConfigPolicy::Full);
    "rag_upload_ingest_embeddings" => args_opaque, result_opaque, "rag", secrets(RUNTIME_SECRET_POINTERS), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_EMBEDDING_RAG), runtime(RuntimeConfigPolicy::Full);
    "read_recent_logs" => args_read_recent_logs, result_read_recent_logs, "system", identity(ADMIN_IDENTITY), effect(READ_EFFECT);
    "record_optimize_agent_event" => args_opaque, result_opaque, "system";
    "refresh_token" => args_refresh_token, result_refresh_token, "auth", identity(PUBLIC_IDENTITY), secrets(REFRESH_TOKEN_SECRET_POINTERS);
    "register_user" => args_register_user, result_login_user, "auth", identity(PUBLIC_IDENTITY), secrets(PASSWORD_SECRET_POINTERS);
    "reject_import_review" => args_import_review_id, result_import_review, "import", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMPORT);
    "reject_pending_optimize_patch_sets" => args_opaque, result_opaque, "system", effect(DESTRUCTIVE_EFFECT);
    "remove_image_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_IMAGE);
    "remove_project_member" => args_member_user_id, result_unit, "project", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "remove_video_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_VIDEO);
    "reorder_chapter_scenes" => args_reorder_chapter_scenes, result_chapter_scenes, "chapter", context(NO_CONTEXT), references(REFERENCES_CHAPTER_IDS);
    "reorder_galgame_lines" => args_reorder_galgame_lines, result_reorder_galgame_lines, "galgame", context(PROJECT_CONTEXT), references(REFERENCES_GALGAME_LINE_IDS);
    "request_attribute_deletion" => args_request_attribute_deletion, result_request_attribute_deletion, "profile", effect(DESTRUCTIVE_EFFECT);
    "reset_deletion_marks" => args_reset_deletion_marks, result_reset_deletion_marks, "profile", effect(DESTRUCTIVE_EFFECT);
    "reset_project_generated_data" => args_project_id, result_opaque, "project", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT), references(REFERENCES_PROJECT_IDS);
    "restore_blueprint_version" => args_restore_blueprint_version, result_restore_blueprint_version, "blueprint", effect(DESTRUCTIVE_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "resume_chapter_analysis_sync" => args_chapter_analysis_task, result_chapter_summary, "chapter", context(NO_CONTEXT), execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::Full), idempotency("server_workflow");
    "resume_manga_pipeline" => args_preview_resume_manga_pipeline_request, result_export_theme, "manga", context(NO_CONTEXT), execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("server_workflow");
    "resume_workflow_run" => args_resume_workflow_run, result_workflow_run, "workflow", context(NO_CONTEXT), execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_WORKFLOW_RUN_IDS), runtime(RuntimeConfigPolicy::Full), idempotency("server_workflow");
    "reverse_engineer_blueprint" => args_opaque, result_opaque, "blueprint", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "reverse_engineer_outline" => args_opaque, result_opaque, "outline", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "reverse_engineer_part_outline" => args_opaque, result_opaque, "outline", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "revoke_project_invite" => args_invite_id, result_unit, "project", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "revoke_session" => args_revoke_session, result_unit, "auth", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "rollback_project_status" => args_forward_project_status, result_create_project, "project", context(PROJECT_CONTEXT), effect(DESTRUCTIVE_EFFECT), references(REFERENCES_PROJECT_IDS);
    "rollback_to_chapter" => args_rollback_to_chapter, result_rollback_to_chapter, "profile", effect(DESTRUCTIVE_EFFECT), references(REFERENCES_CHAPTER_IDS);
    "run_canvas_graph" => args_run_canvas_graph, result_run_canvas_graph, "canvas", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "run_canvas_node" => args_run_canvas_node, result_run_canvas_graph, "canvas", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "run_galgame_asset_generation" => args_run_galgame_asset_generation, result_run_galgame_asset_generation, "galgame", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_GENERATION_SPEC_IDS_MEDIA_ASSET_IDS), capabilities(CAPABILITIES_PROMPT_IMAGE), runtime(RuntimeConfigPolicy::Full);
    "run_import_project_analysis" => args_project_id, result_opaque, "import", context(PROJECT_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_IMPORT), runtime(RuntimeConfigPolicy::Full);
    "save_blueprint" => args_save_blueprint, result_restore_blueprint_version, "blueprint", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "save_canvas_graph" => args_save_canvas_graph, result_get_canvas_graph, "canvas", context(NO_CONTEXT);
    "save_canvas_text_asset" => args_save_canvas_text_asset, result_save_canvas_text_asset, "canvas", context(NO_CONTEXT);
    "save_chapter_outline" => args_save_chapter_outline, result_save_chapter_outline, "outline", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "save_part_outline" => args_save_part_outline, result_save_part_outline, "outline", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "select_segment_video" => args_select_segment_video, result_select_segment_video, "video", effect(DESTRUCTIVE_EFFECT);
    "set_default_embedding_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), capabilities(CAPABILITIES_EMBEDDING);
    "set_default_image_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), capabilities(CAPABILITIES_IMAGE);
    "set_default_llm_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), capabilities(CAPABILITIES_LLM);
    "set_default_video_config" => args_config_id, result_unit, "settings", identity(ADMIN_IDENTITY), capabilities(CAPABILITIES_VIDEO);
    "set_log_level" => args_set_log_level, result_unit, "system", identity(ADMIN_IDENTITY);
    "set_primary_portrait" => args_opaque, result_opaque, "profile", capabilities(CAPABILITIES_IMAGE);
    "set_profile_primary_material" => args_set_profile_primary_material, result_profile_material_group, "profile";
    "start_blueprint_generation" => args_start_blueprint_generation, result_generation_task_handle, "blueprint", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_blueprint_refinement" => args_start_blueprint_refinement, result_generation_task_handle, "blueprint", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_chapter_generation" => args_start_chapter_generation, result_generation_task_handle, "chapter", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_galgame_pipeline" => args_start_galgame_pipeline, result_get_galgame_script, "galgame", context(NO_CONTEXT), execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_GALGAME_SCRIPT_IDS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("server_workflow");
    "start_image_generation" => args_start_image_generation, result_generation_task_handle, "asset", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_inspiration_generation" => args_start_inspiration_generation, result_generation_task_handle, "inspiration", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_manga_pipeline" => args_preview_start_manga_pipeline_request, result_export_theme, "manga", context(NO_CONTEXT), execution("workflow"), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_PROMPT_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("server_workflow");
    "start_optimize_analysis_generation" => args_opaque, result_generation_task_handle, "system", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_optimize_chat_generation" => args_opaque, result_generation_task_handle, "system", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_optimize_patch_generation" => args_opaque, result_generation_task_handle, "system", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_outline_generation" => args_start_outline_generation, result_generation_task_handle, "outline", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_portrait_generation" => args_start_portrait_generation, result_generation_task_handle, "profile", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "start_video_generation" => args_start_video_generation, result_generation_task_handle, "video", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_VIDEO), runtime(RuntimeConfigPolicy::Full), idempotency("client_request_id");
    "sync_all_character_profiles" => args_sync_all_character_profiles, result_sync_all_character_profiles, "profile", execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "sync_blueprint_materials" => args_project_id, result_restore_blueprint_version, "blueprint", context(PROJECT_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full), idempotency("safe");
    "sync_character_from_chapters" => args_sync_character_from_chapters, result_sync_character_from_chapters, "chapter", context(USER_CHAPTER_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS_TASK_IDS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "sync_outline_materials" => args_project_id, result_sync_outline_materials, "outline", context(PROJECT_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full), idempotency("safe");
    "sync_scene_profiles" => args_sync_scene_profiles, result_unit, "profile", context(NO_CONTEXT), execution("task"), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_LLM), runtime(RuntimeConfigPolicy::Full);
    "test_embedding_config" => args_preview_test_embedding_config_request, result_unit, "settings", context(USER_CONTEXT), effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_EMBEDDING), runtime(RuntimeConfigPolicy::Full);
    "test_image_config" => args_preview_test_image_config_request, result_unit, "settings", context(USER_CONTEXT), effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), capabilities(CAPABILITIES_IMAGE), runtime(RuntimeConfigPolicy::Full);
    "test_llm_config" => args_preview_test_image_config_request, result_unit, "settings", context(USER_CONTEXT), effect(READ_EFFECT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "trim_video" => args_opaque, result_opaque, "video", effect(DESTRUCTIVE_EFFECT), capabilities(CAPABILITIES_VIDEO);
    "unbind_galgame_media" => args_unbind_galgame_media, result_unbind_galgame_media, "galgame", context(NO_CONTEXT), effect(DESTRUCTIVE_EFFECT);
    "update_assembly_push_status" => args_opaque, result_opaque, "system", effect(DESTRUCTIVE_EFFECT);
    "update_chapter_content" => args_update_chapter_content, result_chapter_summary, "chapter", context(NO_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::Full);
    "update_chapter_outline" => args_opaque, result_opaque, "chapter", context(CHAPTER_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), references(REFERENCES_CHAPTER_IDS), runtime(RuntimeConfigPolicy::Full);
    "update_chapter_scene" => args_update_chapter_scene, result_chapter_scene, "chapter", context(NO_CONTEXT), references(REFERENCES_CHAPTER_IDS);
    "update_chapter_video_prompt" => args_update_chapter_video_prompt, result_optimize_chapter_video_prompt, "video";
    "update_chapter_video_segment_recipe" => args_opaque, result_opaque, "video", context(USER_CHAPTER_CONTEXT), capabilities(CAPABILITIES_VIDEO);
    "update_character_profile" => args_update_character_profile, result_update_character_profile, "profile", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "update_embedding_config" => args_update_embedding_config, result_create_embedding_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_EMBEDDING);
    "update_foreshadowing" => args_update_foreshadowing, result_foreshadowing_write, "profile", context(USER_CHAPTER_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "update_galgame_choices" => args_update_galgame_choices, result_update_galgame_choices, "galgame", context(PROJECT_CONTEXT), references(REFERENCES_GALGAME_CHOICE_IDS);
    "update_galgame_line" => args_update_galgame_line, result_insert_galgame_line, "galgame", context(PROJECT_CONTEXT), references(REFERENCES_GALGAME_LINE_IDS);
    "update_galgame_scene" => args_update_galgame_scene, result_update_galgame_scene, "galgame", context(PROJECT_CONTEXT), references(REFERENCES_GALGAME_SCENE_IDS);
    "update_image" => args_update_image, result_update_image, "asset";
    "update_image_config" => args_update_image_config, result_add_image_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_IMAGE);
    "update_llm_config" => args_update_llm_config, result_create_llm_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS);
    "update_llm_max_tokens_config" => args_update_llm_max_tokens_config, result_get_llm_max_tokens_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT);
    "update_manga_prompt" => args_update_manga_prompt, result_bind_manga_reference_image, "manga", context(NO_CONTEXT);
    "update_object_profile" => args_update_object_profile, result_object_profile_mutation, "profile";
    "update_project" => args_update_project, result_create_project, "project", context(PROJECT_CONTEXT), references(REFERENCES_PROJECT_IDS);
    "update_project_member_role" => args_update_project_member_role, result_unit, "project", context(PROJECT_CONTEXT);
    "update_scene_profile" => args_update_scene_profile, result_scene_profile_mutation, "profile", context(USER_CONTEXT), secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "update_theme" => args_update_theme, result_activate_theme, "settings";
    "update_user_profile" => args_update_user_profile, result_get_current_user, "auth", context(NO_CONTEXT);
    "update_user_role" => args_opaque, result_opaque, "auth", identity(ADMIN_IDENTITY);
    "update_video" => args_update_video, result_select_segment_video, "video";
    "update_video_config" => args_update_video_config, result_add_video_config, "settings", identity(ADMIN_IDENTITY), context(NO_CONTEXT), secrets(PROVIDER_SECRET_POINTERS), capabilities(CAPABILITIES_VIDEO);
    "update_video_segment_plan_draft_recipe" => args_opaque, result_opaque, "video", capabilities(CAPABILITIES_VIDEO);
    "update_world_setting" => args_update_world_setting, result_restore_blueprint_version, "system", secrets(RUNTIME_SECRET_POINTERS), runtime(RuntimeConfigPolicy::Full);
    "upload_assembly_insert_video" => args_upload_assembly_insert_video, result_select_segment_video, "video", file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT);
    "upload_chapter_video" => args_upload_chapter_video, result_select_segment_video, "video", context(USER_CHAPTER_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT);
    "upload_image" => args_upload_image, result_update_image, "asset", context(PROJECT_CONTEXT), file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT);
    "upload_portrait" => args_opaque, result_opaque, "profile", file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMAGE);
    "upload_profile_material_image" => args_upload_profile_material_image, result_profile_material_group, "profile", file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT), capabilities(CAPABILITIES_IMAGE);
    "upload_reference_media_video" => args_upload_reference_media_video, result_select_segment_video, "video", file_inputs(FILE_INPUTS_SET_BASE64_SET_TEXT);
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

pub fn command_idempotency(command: &str) -> Option<&'static str> {
    command_spec(command).map(|spec| spec.idempotency)
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
    CommandDescriptor {
        name: spec.name.to_string(),
        kind: "host_api".into(),
        domain: spec.domain.into(),
        summary: spec.name.replace('_', " "),
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
    }
}

fn owned_strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn args_opaque() -> Value {
    json!({
        "type": "object",
        "additionalProperties": true,
        "x-a4n-schema-quality": "opaque",
    })
}

fn args_config_id() -> Value {
    json!({
        "type": "object",
        "properties": { "configId": { "type": "integer", "minimum": 1 } },
        "required": ["configId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

fn args_activate_theme() -> Value {
    {
        json!({
            "type": "object",
            "properties": { "themeId": { "type": "integer", "minimum": 1 } },
            "required": ["themeId"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed",
        })
    }
}

fn args_accept_import_review() -> Value {
    strict_typed_input_schema::<crate::import_review::AcceptImportReviewInput>()
}

fn args_add_profile_attribute() -> Value {
    strict_typed_input_schema::<crate::character::AddProfileAttributeInput>()
}

fn args_add_image_config() -> Value {
    typed_input_schema::<crate::config::ImageConfigInput>()
}

fn args_add_video_config() -> Value {
    typed_input_schema::<crate::config::VideoConfigInput>()
}

fn args_apply_assembly_postprocess_edit() -> Value {
    typed_input_schema::<crate::video::ApplyAssemblyPostprocessEditInput>()
}

fn args_apply_implicit_update_decision() -> Value {
    strict_typed_input_schema::<crate::character::ApplyImplicitUpdateDecisionInput>()
}

fn args_attribute_history() -> Value {
    strict_typed_input_schema::<crate::character::AttributeHistoryInput>()
}

fn args_bind_galgame_media() -> Value {
    typed_input_schema::<crate::galgame::BindGalgameMediaInput>()
}

fn args_bind_manga_reference_image() -> Value {
    typed_input_schema::<crate::manga::BindMangaReferenceImageInput>()
}

fn args_batch_accept_reviews() -> Value {
    strict_typed_input_schema::<crate::import_review::BatchAcceptImportReviewsInput>()
}

fn args_cancel_embedding_model_download() -> Value {
    typed_input_schema::<crate::embedding::CancelEmbeddingModelDownloadInput>()
}

fn args_cancel_generation_task() -> Value {
    strict_typed_input_schema::<crate::generation_tasks::CancelGenerationTaskInput>()
}

fn args_cancel_galgame_pipeline() -> Value {
    typed_input_schema::<crate::galgame::GalgameProjectInput>()
}

fn args_cancel_manga_pipeline() -> Value {
    typed_input_schema::<crate::manga::MangaChapterInput>()
}

fn args_cancel_task() -> Value {
    typed_schema::<crate::task::CancelTaskInput>()
}

fn args_cancel_workflow() -> Value {
    strict_typed_input_schema::<crate::workflow::CancelWorkflowInput>()
}

fn args_change_user_password() -> Value {
    typed_input_schema::<crate::auth::ChangePasswordInput>()
}

fn args_check_import_dedup() -> Value {
    strict_typed_input_schema::<crate::import_review::CheckImportDedupInput>()
}

fn args_check_implicit_update() -> Value {
    strict_typed_input_schema::<crate::character::CheckImplicitUpdateInput>()
}

fn args_check_profile_sync_conflict() -> Value {
    strict_typed_input_schema::<crate::character::CheckProfileSyncConflictInput>()
}

fn args_chapter_analysis() -> Value {
    strict_runtime_typed_input_schema::<crate::chapter::ChapterAnalysisInput>()
}

fn args_chapter_analysis_task() -> Value {
    optional_task_runtime_typed_input_schema::<crate::chapter::ChapterAnalysisInput>()
}

fn args_cleanup_orphan_project_data_dirs() -> Value {
    typed_input_schema::<crate::storage::CleanupOrphanProjectDataDirsInput>()
}

fn args_create_derived_project() -> Value {
    typed_input_schema::<crate::project::CreateDerivedProjectInput>()
}

fn args_create_embedding_config() -> Value {
    typed_input_schema::<crate::config::CreateEmbeddingInput>()
}

fn args_create_freeform_video_project() -> Value {
    typed_input_schema::<crate::project::CreateFreeformVideoProjectInput>()
}

fn args_create_llm_config() -> Value {
    typed_input_schema::<crate::config::CreateLlmInput>()
}

fn args_create_profile() -> Value {
    typed_input_schema::<crate::character::CreateProfileInput>()
}

fn args_create_object_profile() -> Value {
    strict_typed_input_schema::<crate::profile::CreateObjectProfileInput>()
}

fn args_create_foreshadowing() -> Value {
    strict_runtime_typed_input_schema::<crate::profile::CreateForeshadowingInput>()
}

fn args_create_scene_profile() -> Value {
    strict_runtime_typed_input_schema::<crate::profile::CreateSceneProfileInput>()
}

fn args_create_project() -> Value {
    typed_input_schema::<crate::project::CreateProjectInput>()
}

fn args_create_theme() -> Value {
    typed_input_schema::<crate::theme::CreateThemeInput>()
}

fn args_create_user() -> Value {
    typed_input_schema::<crate::admin::CreateUserInput>()
}

fn args_delete_chapter_video_prompt() -> Value {
    typed_input_schema::<crate::video::DeleteChapterVideoPromptInput>()
}

fn args_delete_chapter_scene() -> Value {
    json!({
        "type": "object",
        "properties": { "sceneId": { "type": "integer", "minimum": 1 } },
        "required": ["sceneId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

fn args_delete_galgame_line() -> Value {
    project_typed_input_schema::<crate::galgame::DeleteGalgameLineInput>()
}

fn args_delete_image() -> Value {
    json!({
        "type": "object",
        "properties": { "imageId": { "type": "integer", "minimum": 1 } },
        "required": ["imageId"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed",
    })
}

fn args_delete_object_profile() -> Value {
    strict_typed_input_schema::<crate::profile::DeleteObjectProfileInput>()
}

fn args_delete_foreshadowing() -> Value {
    strict_typed_input_schema::<crate::profile::DeleteForeshadowingInput>()
}

fn args_delete_scene_profile() -> Value {
    strict_runtime_typed_input_schema::<crate::profile::DeleteSceneProfileInput>()
}

fn args_delete_latest_outline_records() -> Value {
    typed_input_schema::<crate::outline::DeleteLatestOutlineRecordsInput>()
}

fn args_delete_local_embedding_model() -> Value {
    typed_input_schema::<crate::embedding::EmbeddingModelInput>()
}

fn args_delete_manga_prompt() -> Value {
    typed_input_schema::<crate::manga::DeleteMangaPromptInput>()
}

fn args_delete_profile() -> Value {
    json!({
        "type": "object",
        "properties": { "profileId": { "type": "integer", "minimum": 1 } },
        "required": ["profileId"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed",
    })
}

fn args_project_id() -> Value {
    json!({
        "type": "object",
        "properties": { "projectId": { "type": "integer", "minimum": 1 } },
        "required": ["projectId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

fn args_delete_video() -> Value {
    json!({
        "type": "object",
        "properties": { "videoId": { "type": "integer", "minimum": 1 } },
        "required": ["videoId"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed",
    })
}

fn args_duplicate_project() -> Value {
    typed_input_schema::<crate::project::DuplicateProjectInput>()
}

fn args_detect_image_config_models() -> Value {
    json!({
        "type": "object",
        "properties": {
            "userId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::config::ModelDiscoveryInput>(),
        },
        "required": ["userId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

fn args_diff_blueprint_versions() -> Value {
    typed_input_schema::<crate::blueprint::DiffBlueprintVersionsInput>()
}

fn args_diff_profile_chapters() -> Value {
    strict_typed_input_schema::<crate::character::DiffProfileChaptersInput>()
}

fn args_download_embedding_model() -> Value {
    typed_input_schema::<crate::embedding::DownloadEmbeddingModelInput>()
}

fn args_execute_attribute_deletion() -> Value {
    strict_typed_input_schema::<crate::character::ExecuteAttributeDeletionInput>()
}

fn args_edit_chapter_shooting_script() -> Value {
    typed_input_schema::<crate::shooting_script::EditChapterShootingScriptInput>()
}

fn args_export_batch_data() -> Value {
    required_task_typed_input_schema::<crate::export::BatchExportInput>()
}

fn args_export_all_configs_json() -> Value {
    empty_args_schema()
}

fn args_export_characters_json_data() -> Value {
    required_task_typed_input_schema::<crate::export::ExportProjectInput>()
}

fn args_export_derived_project_data() -> Value {
    strict_typed_input_schema::<crate::project_transfer::ExportDerivedProjectDataInput>()
}

fn args_export_epub_data() -> Value {
    required_task_typed_input_schema::<crate::export::ExportPublicationInput>()
}

fn args_export_project_family_data() -> Value {
    strict_typed_input_schema::<crate::project_transfer::ExportProjectFamilyDataInput>()
}

fn args_export_video_studio_project_data() -> Value {
    strict_typed_input_schema::<crate::project_transfer::ExportVideoStudioProjectDataInput>()
}

fn args_forward_project_status() -> Value {
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

fn args_project_asset_dependency_graph_node_detail() -> Value {
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

fn args_generate_chapter_shooting_script() -> Value {
    {
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
}

fn args_generate_chapter_scene() -> Value {
    optional_task_runtime_typed_input_schema::<crate::chapter::GenerateChapterSceneInput>()
}

fn args_generate_object_profile_image_prompt() -> Value {
    strict_typed_input_schema::<crate::image_generation::ObjectProfileImageInput>()
}

fn args_generate_scene_profile_image_prompt() -> Value {
    strict_typed_input_schema::<crate::image_generation::SceneProfileImageInput>()
}

fn args_get_canvas_graph() -> Value {
    typed_input_schema::<crate::canvas::GetCanvasGraphInput>()
}

fn args_get_canvas_node_contracts() -> Value {
    typed_input_schema::<crate::canvas::GetCanvasNodeContractsInput>()
}

fn args_get_chapter_shooting_script() -> Value {
    typed_input_schema::<crate::shooting_script::ChapterShootingScriptSourceInput>()
}

fn args_get_generation_call_analytics() -> Value {
    typed_input_schema::<crate::admin::GenerationCallAnalyticsInput>()
}

fn args_get_implicit_review() -> Value {
    strict_typed_input_schema::<crate::character::GetImplicitReviewInput>()
}

fn args_get_profile_state_at_chapter() -> Value {
    strict_typed_input_schema::<crate::character::GetProfileStateAtChapterInput>()
}

fn args_get_snapshot() -> Value {
    strict_typed_input_schema::<crate::character::GetSnapshotInput>()
}

fn args_get_llm_max_tokens_config() -> Value {
    empty_args_schema()
}

fn args_get_task_history() -> Value {
    typed_schema::<crate::task::GetTaskHistoryInput>()
}

fn args_get_usage_trends() -> Value {
    typed_input_schema::<crate::admin::UsageTrendsInput>()
}

fn args_get_workflow_progress_events() -> Value {
    strict_typed_input_schema::<crate::workflow::WorkflowProgressEventsInput>()
}

fn args_get_workflow_run_detail() -> Value {
    strict_typed_input_schema::<crate::workflow::GetWorkflowRunInput>()
}

fn args_import_canvas_media() -> Value {
    typed_input_schema::<crate::canvas::ImportCanvasMediaInput>()
}

fn args_import_chapter() -> Value {
    strict_typed_input_schema::<crate::chapter::ImportChapterInput>()
}

fn args_import_all_configs() -> Value {
    typed_input_schema::<crate::config::ImportAllConfigsInput>()
}

fn args_import_galgame_audio() -> Value {
    typed_input_schema::<crate::galgame::ImportGalgameAudioInput>()
}

fn args_import_project_family_upload() -> Value {
    strict_typed_input_schema::<crate::project_transfer::ImportProjectPackageUploadInput>()
}

fn args_import_project_reviews() -> Value {
    strict_typed_input_schema::<crate::import_review::ImportProjectReviewsInput>()
}

fn args_import_project_text_upload() -> Value {
    strict_typed_input_schema::<crate::import::ImportProjectTextUploadInput>()
}

fn args_import_review_id() -> Value {
    strict_typed_input_schema::<crate::import_review::ImportReviewIdInput>()
}

fn args_import_video_studio_project_upload() -> Value {
    strict_typed_input_schema::<crate::project_transfer::ImportProjectPackageUploadInput>()
}

fn args_insert_galgame_line() -> Value {
    project_typed_input_schema::<crate::galgame::InsertGalgameLineInput>()
}

fn args_invite_project_member() -> Value {
    typed_input_schema::<crate::collaboration::InviteProjectMemberInput>()
}

fn args_invite_id() -> Value {
    json!({
        "type": "object",
        "properties": { "inviteId": { "type": "integer", "minimum": 1 } },
        "required": ["inviteId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

fn args_list_blueprint_versions() -> Value {
    typed_input_schema::<crate::blueprint::ListBlueprintVersionsInput>()
}

fn args_list_behaviors() -> Value {
    strict_typed_input_schema::<crate::character::ListBehaviorsInput>()
}

fn args_list_chapter_scenes() -> Value {
    json!({
        "type": "object",
        "properties": { "chapterId": { "type": "integer", "minimum": 1 } },
        "required": ["chapterId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

fn args_list_chapter_video_prompts() -> Value {
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

fn args_list_deletion_marks() -> Value {
    strict_typed_input_schema::<crate::character::ListDeletionMarksInput>()
}

fn args_list_images_by_chapter() -> Value {
    typed_input_schema::<crate::image::ListImagesByChapterInput>()
}

fn args_list_object_profiles() -> Value {
    strict_typed_input_schema::<crate::profile::ListObjectProfilesInput>()
}

fn args_list_foreshadowing() -> Value {
    strict_typed_input_schema::<crate::profile::ListForeshadowingInput>()
}

fn args_list_profile_materials() -> Value {
    strict_typed_input_schema::<crate::profile::ListProfileMaterialsInput>()
}

fn args_list_scene_profiles() -> Value {
    strict_typed_input_schema::<crate::profile::ListSceneProfilesInput>()
}

fn args_list_profiles() -> Value {
    typed_input_schema::<crate::character::ListProfilesInput>()
}

fn args_list_snapshots() -> Value {
    strict_typed_input_schema::<crate::character::ListSnapshotsInput>()
}

fn args_list_video_reference_images() -> Value {
    typed_input_schema::<crate::image::ListVideoReferenceImagesInput>()
}

fn args_login_user() -> Value {
    typed_input_schema::<crate::auth::LoginInput>()
}

fn args_modify_profile_attribute() -> Value {
    strict_typed_input_schema::<crate::character::ModifyProfileAttributeInput>()
}

fn args_optimize_chapter_video_prompt() -> Value {
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

fn args_optimize_manga_prompt() -> Value {
    typed_input_schema::<crate::manga::OptimizeMangaPromptInput>()
}

fn args_preview_canvas_node_request() -> Value {
    typed_input_schema::<crate::canvas::PreviewCanvasNodeRequestInput>()
}

fn args_preview_chapter_video_request() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "promptId": { "type": "integer", "minimum": 1 },
            "configId": { "type": ["integer", "null"], "minimum": 1 },
            "promptOverride": raw_schema::<crate::video::PreviewChapterVideoPromptOverrideInput>(),
        },
        "required": ["projectId", "chapterId", "promptId"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed-input",
    })
}

fn args_preview_chapter_scene_request() -> Value {
    strict_runtime_typed_input_schema::<crate::chapter::GenerateChapterSceneInput>()
}

fn args_preview_galgame_pipeline_request() -> Value {
    typed_input_schema::<crate::galgame::PreviewGalgamePipelineInput>()
}

fn args_preview_object_profile_image_request() -> Value {
    strict_runtime_typed_input_schema::<crate::image_generation::ObjectProfileImageInput>()
}

fn args_preview_generate_blueprint_request() -> Value {
    typed_input_schema::<crate::blueprint::PreviewBlueprintGenerationInput>()
}

fn args_preview_refine_blueprint_request() -> Value {
    typed_input_schema::<crate::blueprint::PreviewBlueprintRefinementInput>()
}

fn args_preview_resume_manga_pipeline_request() -> Value {
    typed_input_schema::<crate::manga::MangaPipelineResumeInput>()
}

fn args_preview_start_manga_pipeline_request() -> Value {
    typed_input_schema::<crate::manga::MangaPipelineStartInput>()
}

fn args_preview_sync_all_character_profiles_request() -> Value {
    typed_input_schema::<crate::character::SyncAllCharacterProfilesInput>()
}

fn args_preview_sync_character_from_chapters_request() -> Value {
    typed_input_schema::<crate::character::SyncCharacterProfileInput>()
}

fn args_preview_test_embedding_config_request() -> Value {
    json!({
        "type": "object",
        "properties": {
            "userId": { "type": "integer", "minimum": 1 },
            "configId": { "type": "integer", "minimum": 1 },
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["userId", "configId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

fn args_preview_test_image_config_request() -> Value {
    json!({
        "type": "object",
        "properties": {
            "userId": { "type": "integer", "minimum": 1 },
            "configId": {
                "type": "integer",
                "anyOf": [{ "const": -1 }, { "minimum": 1 }],
            },
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["userId", "configId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

fn args_prompt_name() -> Value {
    typed_schema::<crate::prompt::PromptNameArgs>()
}

fn args_prompt_list() -> Value {
    typed_schema::<crate::prompt::PromptEmptyArgs>()
}

fn args_prompt_list_project_overrides() -> Value {
    typed_schema::<crate::prompt::PromptProjectArgs>()
}

fn args_prompt_remove_project_override() -> Value {
    typed_input_schema::<crate::prompt::RemoveOverrideInput>()
}

fn args_prompt_render() -> Value {
    typed_input_schema::<crate::prompt::RenderInput>()
}

fn args_prompt_set_project_override() -> Value {
    typed_input_schema::<crate::prompt::SetOverrideInput>()
}

fn args_prompt_update() -> Value {
    typed_input_schema::<crate::prompt::UpdatePromptInput>()
}

fn args_prompt_validate_variables() -> Value {
    typed_input_schema::<crate::prompt::ValidateVarsInput>()
}

fn args_prompt_version_diff() -> Value {
    typed_input_schema::<crate::prompt::VersionDiffInput>()
}

fn args_push_assembled_chapter_video() -> Value {
    typed_input_schema::<crate::video::PushAssembledChapterVideoInput>()
}

fn args_reorder_chapter_scenes() -> Value {
    strict_typed_input_schema::<crate::chapter::ReorderChapterScenesInput>()
}

fn args_read_recent_logs() -> Value {
    typed_input_schema::<crate::logging::ReadLogsInput>()
}

fn args_list_workflow_runs() -> Value {
    strict_typed_input_schema::<crate::workflow::ListWorkflowRunsInput>()
}

fn args_refresh_token() -> Value {
    typed_input_schema::<crate::auth::RefreshTokenInput>()
}

fn args_revoke_session() -> Value {
    typed_input_schema::<crate::auth::RevokeSessionInput>()
}

fn args_register_user() -> Value {
    typed_input_schema::<crate::auth::RegisterInput>()
}

fn args_reorder_galgame_lines() -> Value {
    project_typed_input_schema::<crate::galgame::ReorderGalgameLinesInput>()
}

fn args_request_attribute_deletion() -> Value {
    strict_typed_input_schema::<crate::character::RequestAttributeDeletionInput>()
}

fn args_reset_deletion_marks() -> Value {
    strict_typed_input_schema::<crate::character::ResetDeletionMarksInput>()
}

fn args_restore_blueprint_version() -> Value {
    typed_input_schema::<crate::blueprint::RestoreBlueprintVersionInput>()
}

fn args_rollback_to_chapter() -> Value {
    strict_typed_input_schema::<crate::character::RollbackToChapterInput>()
}

fn args_resume_workflow_run() -> Value {
    strict_runtime_typed_input_schema::<crate::workflow::ResumeWorkflowRunInput>()
}

fn args_run_canvas_graph() -> Value {
    typed_input_schema::<crate::canvas::RunCanvasGraphInput>()
}

fn args_run_canvas_node() -> Value {
    typed_input_schema::<crate::canvas::RunCanvasNodeInput>()
}

fn args_run_galgame_asset_generation() -> Value {
    typed_input_schema::<crate::galgame::RunGalgameAssetGenerationInput>()
}

fn args_save_blueprint() -> Value {
    typed_input_schema::<crate::blueprint::SaveBlueprintInput>()
}

fn args_save_canvas_graph() -> Value {
    typed_input_schema::<crate::canvas::SaveCanvasGraphInput>()
}

fn args_save_canvas_text_asset() -> Value {
    typed_input_schema::<crate::canvas::SaveCanvasTextAssetInput>()
}

fn args_save_chapter_outline() -> Value {
    typed_input_schema::<crate::outline::SaveChapterOutlineInput>()
}

fn args_save_part_outline() -> Value {
    typed_input_schema::<crate::outline::SavePartOutlineInput>()
}

fn args_select_segment_video() -> Value {
    json!({
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
    })
}

fn args_set_log_level() -> Value {
    typed_input_schema::<crate::logging::SetLogLevelInput>()
}

fn args_set_profile_primary_material() -> Value {
    strict_typed_input_schema::<crate::profile::SetProfilePrimaryMaterialInput>()
}

fn args_start_blueprint_generation() -> Value {
    typed_input_schema::<crate::blueprint::StartBlueprintGenerationInput>()
}

fn args_start_blueprint_refinement() -> Value {
    typed_input_schema::<crate::blueprint::StartBlueprintRefinementInput>()
}

fn args_start_chapter_generation() -> Value {
    typed_input_schema::<crate::chapter::StartChapterGenerationInput>()
}

fn args_start_galgame_pipeline() -> Value {
    typed_input_schema::<crate::galgame::StartGalgamePipelineInput>()
}

fn args_start_image_generation() -> Value {
    typed_input_schema::<crate::image_generation::StartImageGenerationInput>()
}

fn args_start_inspiration_generation() -> Value {
    typed_input_schema::<crate::inspiration::StartInspirationGenerationInput>()
}

fn args_start_outline_generation() -> Value {
    typed_input_schema::<crate::outline::StartOutlineGenerationInput>()
}

fn args_start_portrait_generation() -> Value {
    typed_input_schema::<crate::portrait::StartPortraitGenerationInput>()
}

fn args_start_video_generation() -> Value {
    typed_input_schema::<crate::video::StartVideoGenerationInput>()
}

fn args_sync_all_character_profiles() -> Value {
    required_task_typed_input_schema::<crate::character::SyncAllCharacterProfilesInput>()
}

fn args_sync_scene_profiles() -> Value {
    optional_task_runtime_typed_input_schema::<crate::profile::SyncSceneProfilesInput>()
}

fn args_sync_character_from_chapters() -> Value {
    required_task_typed_input_schema::<crate::character::SyncCharacterProfileInput>()
}

fn args_unbind_galgame_media() -> Value {
    typed_input_schema::<crate::galgame::UnbindGalgameMediaInput>()
}

fn args_update_chapter_video_prompt() -> Value {
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

fn args_update_character_profile() -> Value {
    typed_input_schema::<crate::character::UpdateCharacterProfileInput>()
}

fn args_update_chapter_content() -> Value {
    strict_typed_input_schema::<crate::chapter::UpdateChapterContentInput>()
}

fn args_update_chapter_scene() -> Value {
    strict_typed_input_schema::<crate::chapter::UpdateChapterSceneInput>()
}

fn args_update_embedding_config() -> Value {
    typed_input_schema::<crate::config::UpdateEmbeddingInput>()
}

fn args_update_galgame_choices() -> Value {
    project_typed_input_schema::<crate::galgame::UpdateGalgameChoicesInput>()
}

fn args_update_galgame_line() -> Value {
    project_typed_input_schema::<crate::galgame::UpdateGalgameLineInput>()
}

fn args_update_galgame_scene() -> Value {
    project_typed_input_schema::<crate::galgame::UpdateGalgameSceneInput>()
}

fn args_update_image() -> Value {
    json!({
        "type": "object",
        "properties": {
            "imageId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::image::UpdateGeneratedImageInput>(),
        },
        "required": ["imageId", "input"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed-input",
    })
}

fn args_update_image_config() -> Value {
    typed_input_schema::<crate::config::UpdateImageConfigInput>()
}

fn args_update_llm_config() -> Value {
    typed_input_schema::<crate::config::UpdateLlmInput>()
}

fn args_update_llm_max_tokens_config() -> Value {
    typed_input_schema::<crate::config::LlmMaxTokensConfig>()
}

fn args_update_manga_prompt() -> Value {
    typed_input_schema::<crate::manga::UpdateMangaPromptInput>()
}

fn args_update_project() -> Value {
    typed_input_schema::<crate::project::UpdateProjectInput>()
}

fn args_update_project_member_role() -> Value {
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

fn args_update_object_profile() -> Value {
    strict_typed_input_schema::<crate::profile::UpdateObjectProfileInput>()
}

fn args_update_foreshadowing() -> Value {
    strict_runtime_typed_input_schema::<crate::profile::UpdateForeshadowingInput>()
}

fn args_update_scene_profile() -> Value {
    strict_runtime_typed_input_schema::<crate::profile::UpdateSceneProfileInput>()
}

fn args_member_user_id() -> Value {
    json!({
        "type": "object",
        "properties": { "memberUserId": { "type": "integer", "minimum": 1 } },
        "required": ["memberUserId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

fn args_update_theme() -> Value {
    json!({
        "type": "object",
        "properties": {
            "themeId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::theme::UpdateThemeInput>(),
        },
        "required": ["themeId", "input"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed-input",
    })
}

fn args_update_user_profile() -> Value {
    typed_input_schema::<crate::auth::UpdateProfileInput>()
}

fn args_update_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "videoId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::video::UpdateGeneratedVideoInput>(),
        },
        "required": ["videoId", "input"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed-input",
    })
}

fn args_update_video_config() -> Value {
    typed_input_schema::<crate::config::UpdateVideoConfigInput>()
}

fn args_update_world_setting() -> Value {
    typed_input_schema::<crate::blueprint::UpdateWorldSettingInput>()
}

fn args_upload_assembly_insert_video() -> Value {
    json!({
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
    })
}

fn args_upload_chapter_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::video::UploadChapterVideoInput>(),
        },
        "required": ["projectId", "chapterId", "input"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed-input",
    })
}

fn args_upload_image() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::image::UploadImageInput>(),
        },
        "required": ["projectId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

fn args_upload_profile_material_image() -> Value {
    strict_typed_input_schema::<crate::profile::UploadProfileMaterialImageInput>()
}

fn args_upload_reference_media_video() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "chapterId": { "type": "integer", "minimum": 1 },
            "input": raw_schema::<crate::video::UploadReferenceMediaVideoInput>(),
        },
        "required": ["projectId", "chapterId", "input"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed-input",
    })
}

fn result_opaque() -> Value {
    json!({ "x-a4n-schema-quality": "opaque" })
}

fn result_unit() -> Value {
    typed_schema::<()>()
}

fn result_activate_theme() -> Value {
    typed_schema::<crate::theme::ThemeSummary>()
}

fn result_add_image_config() -> Value {
    typed_schema::<crate::config::ImageConfigSummary>()
}

fn result_add_video_config() -> Value {
    typed_schema::<crate::config::VideoConfigSummary>()
}

fn result_apply_implicit_update_decision() -> Value {
    typed_schema::<crate::character::ApplyImplicitUpdateDecisionResultSummary>()
}

fn result_attribute_history() -> Value {
    typed_schema::<Vec<crate::character::AttributeEventSummary>>()
}

fn result_bind_galgame_media() -> Value {
    typed_schema::<crate::galgame::GalgameMediaBindingSummary>()
}

fn result_bind_manga_reference_image() -> Value {
    typed_schema::<crate::manga::MangaPagePromptSummary>()
}

fn result_cancel_galgame_pipeline() -> Value {
    typed_schema::<crate::galgame::GalgamePipelineStatusSummary>()
}

fn result_cancel_manga_pipeline() -> Value {
    typed_schema::<i64>()
}

fn result_chapter_summary() -> Value {
    typed_schema::<crate::chapter::ChapterSummary>()
}

fn result_chapter_summaries() -> Value {
    typed_schema::<Vec<crate::chapter::ChapterSummary>>()
}

fn result_chapter_analysis_mutation() -> Value {
    typed_schema::<crate::chapter::ChapterAnalysisMutationResult>()
}

fn result_chapter_scene() -> Value {
    typed_schema::<crate::chapter::ChapterSceneSummary>()
}

fn result_chapter_scenes() -> Value {
    typed_schema::<Vec<crate::chapter::ChapterSceneSummary>>()
}

fn result_check_implicit_update() -> Value {
    typed_schema::<crate::character::ImplicitUpdateDecisionSummary>()
}

fn result_check_profile_sync_conflict() -> Value {
    typed_schema::<crate::character::ProfileSyncConflictSummary>()
}

fn result_cleanup_orphan_project_data_dirs() -> Value {
    typed_schema::<crate::storage::OrphanProjectDataDirsSummary>()
}

fn result_create_embedding_config() -> Value {
    typed_schema::<crate::config::EmbeddingConfigSummary>()
}

fn result_create_llm_config() -> Value {
    typed_schema::<crate::config::LlmConfigSummary>()
}

fn result_create_profile() -> Value {
    typed_schema::<crate::character::CharacterProfileCreateResult>()
}

fn result_create_project() -> Value {
    typed_schema::<crate::project::ProjectSummary>()
}

fn result_object_profile_mutation() -> Value {
    typed_schema::<crate::profile::ObjectProfileMutationResult>()
}

fn result_foreshadowing_write() -> Value {
    typed_schema::<crate::profile::ForeshadowingWriteResult>()
}

fn result_scene_profile_mutation() -> Value {
    typed_schema::<crate::profile::SceneProfileMutationResult>()
}

fn result_export_project_package_data() -> Value {
    typed_schema::<crate::project_transfer::ExportProjectPackageData>()
}

fn result_delete_profile() -> Value {
    typed_schema::<crate::character::CharacterProfileDeleteResult>()
}

fn result_object_profile_delete() -> Value {
    typed_schema::<crate::profile::ObjectProfileDeleteResult>()
}

fn result_scene_profile_delete() -> Value {
    typed_schema::<crate::profile::SceneProfileDeleteResult>()
}

fn result_detect_image_config_models() -> Value {
    typed_schema::<crate::config::ModelDiscoveryResult>()
}

fn result_diff_blueprint_versions() -> Value {
    typed_schema::<crate::blueprint::BlueprintDiffSummary>()
}

fn result_diff_profile_chapters() -> Value {
    typed_schema::<crate::character::SnapshotDeltaSummary>()
}

fn result_download_embedding_model() -> Value {
    typed_schema::<crate::embedding::LocalEmbeddingModelSummary>()
}

fn result_edit_chapter_shooting_script() -> Value {
    typed_schema::<crate::shooting_script::EditedChapterShootingScriptView>()
}

fn result_export_batch_data() -> Value {
    typed_schema::<Vec<crate::export::ExportFileData>>()
}

fn result_export_characters_json_data() -> Value {
    typed_schema::<crate::export::ExportFileData>()
}

fn result_export_theme() -> Value {
    typed_schema::<String>()
}

fn result_export_all_configs_json() -> Value {
    typed_schema::<String>()
}

fn result_import_all_configs() -> Value {
    typed_schema::<Vec<String>>()
}

fn result_generate_chapter_shooting_script() -> Value {
    typed_schema::<crate::shooting_script::ChapterShootingScriptView>()
}

fn result_generate_next_chapter_shooting_script_unit() -> Value {
    typed_schema::<crate::shooting_script::ChapterShootingScriptUnitProgressView>()
}

fn result_get_admin_overview() -> Value {
    typed_schema::<crate::admin::AdminOverviewSummary>()
}

fn result_get_blueprint() -> Value {
    typed_schema::<Option<crate::blueprint::BlueprintSummary>>()
}

fn result_get_canvas_graph() -> Value {
    typed_schema::<crate::canvas::CanvasGraphDocument>()
}

fn result_get_canvas_node_contracts() -> Value {
    typed_schema::<Vec<crate::canvas::CanvasNodeContract>>()
}

fn result_get_chapter_shooting_script() -> Value {
    typed_schema::<Option<crate::shooting_script::ChapterShootingScriptView>>()
}

fn result_get_chapter_shooting_script_unit_progress() -> Value {
    typed_schema::<Option<crate::shooting_script::ChapterShootingScriptUnitProgressView>>()
}

fn result_get_current_user() -> Value {
    typed_schema::<crate::auth::UserSummary>()
}

fn result_get_embedding_model_download_info() -> Value {
    typed_schema::<crate::embedding::EmbeddingModelDownloadInfoSummary>()
}

fn result_get_galgame_script() -> Value {
    typed_schema::<crate::galgame::GalgameScriptDetail>()
}

fn result_get_generation_call_analytics() -> Value {
    typed_schema::<crate::admin::GenerationCallAnalytics>()
}

fn result_get_implicit_review() -> Value {
    typed_schema::<crate::character::ImplicitReviewSummary>()
}

fn result_get_profile_state_at_chapter() -> Value {
    typed_schema::<crate::character::ProfileChapterStateSummary>()
}

fn result_get_snapshot() -> Value {
    typed_schema::<crate::character::SnapshotSummary>()
}

fn result_get_llm_max_tokens_config() -> Value {
    typed_schema::<crate::config::LlmMaxTokensConfig>()
}

fn result_get_manga_pipeline_status() -> Value {
    typed_schema::<crate::manga::MangaPipelineStatusSummary>()
}

fn result_get_project_asset_dependency_graph() -> Value {
    typed_schema::<crate::project::AssetDependencyGraph>()
}

fn result_get_project_asset_dependency_graph_node_detail() -> Value {
    typed_schema::<crate::project::AssetGraphNodeDetail>()
}

fn result_get_project_status_machine() -> Value {
    typed_schema::<crate::project::ProjectStatusMachineSummary>()
}

fn result_get_queue_stats() -> Value {
    typed_schema::<crate::task::QueueStatsSummary>()
}

fn result_get_storage_paths() -> Value {
    typed_schema::<crate::storage::StoragePathsSummary>()
}

fn result_get_task_history() -> Value {
    typed_schema::<Vec<crate::task::TaskInfoSummary>>()
}

fn result_get_usage_trends() -> Value {
    typed_schema::<Vec<crate::admin::DailyUsagePoint>>()
}

fn result_get_user_usage() -> Value {
    typed_schema::<Vec<crate::admin::UsageLogSummary>>()
}

fn result_get_workflow_progress_events() -> Value {
    typed_schema::<crate::workflow::WorkflowProgressEventsResponse>()
}

fn result_get_workflow_run_detail() -> Value {
    typed_schema::<crate::workflow::WorkflowRunDetailSummary>()
}

fn result_import_canvas_media() -> Value {
    typed_schema::<crate::canvas::ImportedCanvasMedia>()
}

fn result_dedup() -> Value {
    typed_schema::<crate::import_review::DedupResult>()
}

fn result_import_review() -> Value {
    typed_schema::<crate::import_review::ImportReviewSummary>()
}

fn result_import_reviews() -> Value {
    typed_schema::<Vec<crate::import_review::ImportReviewSummary>>()
}

fn result_insert_galgame_line() -> Value {
    typed_schema::<crate::galgame::GalgameLine>()
}

fn result_list_active_workflows() -> Value {
    typed_schema::<Vec<crate::workflow::WorkflowStateSummary>>()
}

fn result_list_workflow_runs() -> Value {
    typed_schema::<Vec<crate::workflow::WorkflowRunSummary>>()
}

fn result_list_all_projects() -> Value {
    typed_schema::<Vec<crate::admin::AdminProjectSummary>>()
}

fn result_list_blueprint_versions() -> Value {
    typed_schema::<Vec<crate::blueprint::BlueprintVersionSummary>>()
}

fn result_list_behaviors() -> Value {
    typed_schema::<Vec<crate::character::BehaviorRecordSummary>>()
}

fn result_list_chapter_video_prompts() -> Value {
    typed_schema::<Vec<crate::video::ChapterVideoPromptSummary>>()
}

fn result_list_embedding_config() -> Value {
    typed_schema::<Vec<crate::config::EmbeddingConfigSummary>>()
}

fn result_list_deletion_marks() -> Value {
    typed_schema::<Vec<crate::character::DeletionMarkSummary>>()
}

fn result_list_image_configs() -> Value {
    typed_schema::<Vec<crate::config::ImageConfigSummary>>()
}

fn result_list_images_by_chapter() -> Value {
    typed_schema::<Vec<crate::image::GeneratedImageSummary>>()
}

fn result_list_object_profiles() -> Value {
    typed_schema::<Vec<crate::profile::ObjectProfileSummary>>()
}

fn result_list_foreshadowing() -> Value {
    typed_schema::<Vec<crate::profile::ForeshadowingSummary>>()
}

fn result_list_profile_materials() -> Value {
    typed_schema::<Vec<crate::profile::ProfileMaterialGroup>>()
}

fn result_list_scene_profiles() -> Value {
    typed_schema::<Vec<crate::profile::SceneProfileSummary>>()
}

fn result_list_llm_config() -> Value {
    typed_schema::<Vec<crate::config::LlmConfigSummary>>()
}

fn result_list_local_embedding_models() -> Value {
    typed_schema::<Vec<crate::embedding::LocalEmbeddingModelSummary>>()
}

fn result_list_manga_page_prompts() -> Value {
    typed_schema::<Vec<crate::manga::MangaPagePromptSummary>>()
}

fn result_list_model_provider_catalog() -> Value {
    typed_schema::<crate::provider_catalog::ModelProviderCatalog>()
}

fn result_list_my_project_invites() -> Value {
    typed_schema::<Vec<crate::collaboration::ProjectInviteSummary>>()
}

fn result_list_pending_tasks() -> Value {
    typed_schema::<Vec<crate::admin::TaskQueueSummary>>()
}

fn result_list_profiles() -> Value {
    typed_schema::<Vec<crate::character::CharacterProfileSummary>>()
}

fn result_list_snapshots() -> Value {
    typed_schema::<Vec<crate::character::SnapshotReferenceSummary>>()
}

fn result_list_project_members() -> Value {
    typed_schema::<Vec<crate::collaboration::ProjectMemberSummary>>()
}

fn result_project_summaries() -> Value {
    typed_schema::<Vec<crate::project::ProjectSummary>>()
}

fn result_list_projects() -> Value {
    typed_schema::<crate::project::PaginatedProjects>()
}

fn result_list_sessions() -> Value {
    typed_schema::<Vec<crate::auth::SessionSummary>>()
}

fn result_list_themes() -> Value {
    typed_schema::<Vec<crate::theme::ThemeSummary>>()
}

fn result_list_video_configs() -> Value {
    typed_schema::<Vec<crate::config::VideoConfigSummary>>()
}

fn result_list_videos_by_chapter() -> Value {
    typed_schema::<Vec<crate::video::GeneratedVideoSummary>>()
}

fn result_login_user() -> Value {
    typed_schema::<crate::auth::AuthResult>()
}

fn result_optimize_chapter_video_prompt() -> Value {
    typed_schema::<crate::video::ChapterVideoPromptSummary>()
}

fn result_plan_galgame_assets() -> Value {
    typed_schema::<crate::galgame::GalgameAssetPlan>()
}

fn result_preview_canvas_node_request() -> Value {
    typed_schema::<crate::canvas::CanvasNodeRequestPreview>()
}

fn result_model_request_preview() -> Value {
    typed_schema::<crate::request_preview::ModelRequestPreview>()
}

fn result_object_profile_image_prompt() -> Value {
    typed_schema::<crate::image_generation::ObjectProfileImagePromptOutput>()
}

fn result_scene_profile_image_prompt() -> Value {
    typed_schema::<crate::image_generation::SceneProfileImagePromptOutput>()
}

fn result_preview_rollback() -> Value {
    typed_schema::<crate::project::ProjectRollbackPreview>()
}

fn result_prompt_validate_variables() -> Value {
    typed_schema::<Vec<String>>()
}

fn result_prompt_list() -> Value {
    typed_schema::<Vec<crate::prompt::PromptEntrySummary>>()
}

fn result_prompt_list_categories() -> Value {
    typed_schema::<Vec<crate::prompt::CategoryInfo>>()
}

fn result_prompt_list_project_overrides() -> Value {
    typed_schema::<Vec<crate::prompt::ProjectPromptOverrideSummary>>()
}

fn result_prompt_load_defaults() -> Value {
    typed_schema::<usize>()
}

fn result_prompt_remove_project_override() -> Value {
    typed_schema::<()>()
}

fn result_prompt_render_ab() -> Value {
    typed_schema::<crate::prompt::AbRenderResult>()
}

fn result_prompt_update() -> Value {
    typed_schema::<crate::prompt::PromptEntrySummary>()
}

fn result_prompt_version_history() -> Value {
    typed_schema::<Vec<crate::prompt::PromptVersionSummary>>()
}

fn result_read_recent_logs() -> Value {
    typed_schema::<crate::logging::LogReadResult>()
}

fn result_profile_material_group() -> Value {
    typed_schema::<crate::profile::ProfileMaterialGroup>()
}

fn result_request_attribute_deletion() -> Value {
    typed_schema::<crate::character::DeletionMarkSummary>()
}

fn result_reset_deletion_marks() -> Value {
    typed_schema::<u64>()
}

fn result_refresh_token() -> Value {
    typed_schema::<crate::auth::TokenPairSummary>()
}

fn result_reorder_galgame_lines() -> Value {
    typed_schema::<Vec<crate::galgame::GalgameLine>>()
}

fn result_restore_blueprint_version() -> Value {
    typed_schema::<crate::blueprint::BlueprintMutationResult>()
}

fn result_rollback_to_chapter() -> Value {
    typed_schema::<crate::character::RollbackResultSummary>()
}

fn result_run_canvas_graph() -> Value {
    typed_schema::<crate::canvas::CanvasRunSummary>()
}

fn result_run_galgame_asset_generation() -> Value {
    typed_schema::<crate::galgame::GalgameAssetGenerationSummary>()
}

fn result_save_canvas_text_asset() -> Value {
    typed_schema::<crate::canvas::CanvasTextAssetSummary>()
}

fn result_save_chapter_outline() -> Value {
    typed_schema::<crate::outline::SaveChapterOutlineResult>()
}

fn result_save_part_outline() -> Value {
    typed_schema::<crate::outline::SavePartOutlineResult>()
}

fn result_select_segment_video() -> Value {
    typed_schema::<crate::video::GeneratedVideoSummary>()
}

fn result_generation_task_handle() -> Value {
    typed_schema::<crate::generation_tasks::GenerationTaskHandle>()
}

fn result_sync_all_character_profiles() -> Value {
    typed_schema::<crate::character::SyncAllCharacterProfilesResult>()
}

fn result_sync_character_from_chapters() -> Value {
    typed_schema::<crate::character::SyncCharacterProfileResult>()
}

fn result_sync_outline_materials() -> Value {
    typed_schema::<crate::outline::OutlineMutationResult>()
}

fn result_unbind_galgame_media() -> Value {
    typed_schema::<u64>()
}

fn result_update_character_profile() -> Value {
    typed_schema::<crate::character::CharacterProfileMutationResult>()
}

fn result_update_galgame_choices() -> Value {
    typed_schema::<Vec<crate::galgame::GalgameChoice>>()
}

fn result_update_galgame_scene() -> Value {
    typed_schema::<crate::galgame::GalgameScene>()
}

fn result_update_image() -> Value {
    typed_schema::<crate::image::GeneratedImageSummary>()
}

fn result_workflow_run() -> Value {
    typed_schema::<crate::workflow::WorkflowRunSummary>()
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

fn strict_typed_input_schema<T: JsonSchema>() -> Value {
    let mut schema = typed_input_schema::<T>();
    schema["additionalProperties"] = Value::Bool(false);
    schema
}

fn strict_runtime_typed_input_schema<T: JsonSchema>() -> Value {
    json!({
        "type": "object",
        "properties": {
            "input": raw_schema::<T>(),
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

fn optional_task_runtime_typed_input_schema<T: JsonSchema>() -> Value {
    let mut schema = strict_runtime_typed_input_schema::<T>();
    schema["properties"]
        .as_object_mut()
        .expect("runtime input properties must be an object")
        .insert(
            "taskId".into(),
            json!({
                "type": ["string", "null"],
                "minLength": 1,
                "maxLength": 160,
                "pattern": "^[A-Za-z0-9_.:-]+$",
            }),
        );
    schema
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

fn with_context_fields(mut schema: Value, fields: &[&str]) -> Value {
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::generation_tasks::GENERATION_START_ROUTES;

    use super::{
        API_COMMANDS, COMMAND_SPECS, RUNTIME_EQUIVALENTS, RuntimeConfigPolicy, command_descriptor,
        command_runtime_config_policy, is_destructive,
    };

    #[test]
    fn command_specs_are_the_complete_sorted_source() {
        assert_eq!(API_COMMANDS.len(), COMMAND_SPECS.len());
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
        assert!(
            GENERATION_START_ROUTES
                .iter()
                .all(|(command, _)| API_COMMANDS.binary_search(command).is_ok())
        );

        for (name, spec) in API_COMMANDS.iter().zip(COMMAND_SPECS) {
            assert_eq!(*name, spec.name);
            let descriptor = command_descriptor(name).expect("declared command");
            assert_eq!(descriptor.name, *name);
            assert!(descriptor.args_schema.get("x-a4n-context-fields").is_some());
        }
    }

    #[test]
    fn security_and_lifecycle_metadata_remain_explicit() {
        let provider = command_descriptor("create_llm_config").unwrap();
        assert_eq!(provider.auth, "admin");
        assert!(
            provider
                .secret_pointers
                .contains(&"/input/api_key_encrypted".to_string())
        );

        let generation = command_descriptor("start_chapter_generation").unwrap();
        assert_eq!(generation.execution, "task");
        assert_eq!(generation.idempotency, "client_request_id");
        assert_eq!(generation.runtime_config_policy, RuntimeConfigPolicy::Full);
        assert!(
            generation
                .secret_pointers
                .contains(&"/runtime_config_snapshot/llm/api_key_encrypted".to_string())
        );
        assert_eq!(
            command_runtime_config_policy("preview_chapter_prompt"),
            RuntimeConfigPolicy::WithoutSecrets
        );
        assert_eq!(
            command_runtime_config_policy("preview_source_sync"),
            RuntimeConfigPolicy::None
        );

        assert!(is_destructive("delete_project"));
        assert!(!is_destructive("get_project"));
        assert!(is_destructive("delete_future_resource"));
    }

    #[test]
    fn material_mutations_use_the_request_embedding_configuration() {
        for command in [
            "accept_import_review",
            "apply_optimization_patches",
            "batch_accept_reviews",
            "confirm_reverse_engineering_candidate",
            "create_foreshadowing",
            "create_profile",
            "create_scene_profile",
            "delete_latest_chapter_outlines",
            "delete_latest_parts",
            "delete_scene_profile",
            "import_chapter",
            "restore_blueprint_version",
            "save_blueprint",
            "save_chapter_outline",
            "save_part_outline",
            "update_chapter_content",
            "update_chapter_outline",
            "update_character_profile",
            "update_foreshadowing",
            "update_scene_profile",
            "update_world_setting",
        ] {
            assert_eq!(
                command_runtime_config_policy(command),
                RuntimeConfigPolicy::Full,
                "{command} 会更新资料检索索引，必须复用当前请求的 Embedding 配置",
            );
        }
    }

    #[test]
    fn profile_material_commands_expose_one_typed_contract() {
        for command in [
            "list_foreshadowing",
            "create_foreshadowing",
            "update_foreshadowing",
            "delete_foreshadowing",
            "list_object_profiles",
            "create_object_profile",
            "update_object_profile",
            "delete_object_profile",
            "list_scene_profiles",
            "create_scene_profile",
            "update_scene_profile",
            "delete_scene_profile",
            "list_profile_materials",
            "set_profile_primary_material",
            "upload_profile_material_image",
            "generate_object_profile_image_prompt",
            "generate_scene_profile_image_prompt",
            "preview_object_profile_image_request",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["additionalProperties"], false,
                "{command}"
            );
            assert_eq!(
                descriptor.args_schema["properties"]["input"]["additionalProperties"], false,
                "{command}",
            );
            assert_ne!(
                descriptor.result_schema["x-a4n-schema-quality"], "opaque",
                "{command}",
            );
        }

        for command in [
            "generate_object_profile_image_prompt",
            "generate_scene_profile_image_prompt",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(descriptor.effect, "read", "{command}");
            assert_eq!(descriptor.execution, "immediate", "{command}");
            assert!(descriptor.secret_pointers.is_empty(), "{command}");
            assert!(descriptor.result_references.is_empty(), "{command}");
            assert_eq!(descriptor.runtime_config_policy, RuntimeConfigPolicy::None);
        }
    }

    #[test]
    fn scene_profile_sync_uses_authenticated_typed_input_and_unit_result() {
        let descriptor = command_descriptor("sync_scene_profiles").unwrap();
        assert_eq!(descriptor.execution, "task");
        assert_eq!(descriptor.args_schema["additionalProperties"], false);
        assert_eq!(
            descriptor.args_schema["properties"]["input"]["additionalProperties"],
            false
        );
        assert!(descriptor.args_schema["properties"].get("userId").is_none());
        assert!(
            descriptor.args_schema["properties"]
                .get("projectId")
                .is_none()
        );
        assert!(
            descriptor.args_schema["properties"]
                .get("chapterNumber")
                .is_none()
        );
        assert_eq!(descriptor.result_schema["x-a4n-schema-quality"], "typed");
        assert!(descriptor.result_references.is_empty());
    }

    #[test]
    fn character_history_commands_expose_one_strict_contract() {
        for command in [
            "add_profile_attribute",
            "modify_profile_attribute",
            "attribute_history",
            "list_behaviors",
            "get_implicit_review",
            "check_implicit_update",
            "preview_check_implicit_update_request",
            "apply_implicit_update_decision",
            "list_snapshots",
            "get_profile_state_at_chapter",
            "get_snapshot",
            "diff_profile_chapters",
            "check_profile_sync_conflict",
            "rollback_to_chapter",
            "list_deletion_marks",
            "request_attribute_deletion",
            "execute_attribute_deletion",
            "reset_deletion_marks",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["additionalProperties"], false,
                "{command}"
            );
            assert_eq!(
                descriptor.args_schema["properties"]["input"]["additionalProperties"], false,
                "{command}",
            );
            assert_ne!(
                descriptor.result_schema["x-a4n-schema-quality"], "opaque",
                "{command}"
            );
        }
    }

    #[test]
    fn chapter_content_commands_expose_the_immediate_strict_contract() {
        let import = command_descriptor("import_chapter").unwrap();
        assert_eq!(import.execution, "immediate");
        assert_eq!(import.idempotency, "none");
        assert!(import.file_inputs.is_empty());
        assert_eq!(import.result_references, vec!["chapter_ids".to_string()]);
        assert_eq!(import.args_schema["additionalProperties"], false);
        assert_eq!(
            import.args_schema["properties"]["input"]["additionalProperties"],
            false
        );
        assert!(
            import.args_schema["properties"]["input"]["properties"]
                .get("manual")
                .is_none()
        );
        assert_eq!(import.result_schema["x-a4n-schema-quality"], "typed");

        let update = command_descriptor("update_chapter_content").unwrap();
        assert_eq!(
            update.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert_eq!(update.args_schema["additionalProperties"], false);
        assert_eq!(update.result_schema["x-a4n-schema-quality"], "typed");

        let list = command_descriptor("list_chapters").unwrap();
        assert_eq!(list.result_schema["x-a4n-schema-quality"], "typed");
    }

    #[test]
    fn chapter_analysis_commands_expose_one_strict_contract() {
        let generate = command_descriptor("generate_chapter_analysis").unwrap();
        assert_eq!(generate.execution, "task");
        assert_eq!(generate.idempotency, "none");
        assert_eq!(
            generate.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert_eq!(generate.args_schema["additionalProperties"], false);
        assert_eq!(
            generate.args_schema["properties"]["input"]["additionalProperties"],
            false
        );
        assert_eq!(
            generate.args_schema["properties"]["taskId"]["type"],
            serde_json::json!(["string", "null"])
        );
        assert!(generate.args_schema["properties"].get("userId").is_none());
        assert!(
            generate.args_schema["properties"]
                .get("chapterId")
                .is_none()
        );
        assert_eq!(generate.result_schema["x-a4n-schema-quality"], "typed");

        let preview = command_descriptor("preview_chapter_analysis_request").unwrap();
        assert_eq!(preview.effect, "read");
        assert_eq!(preview.args_schema["additionalProperties"], false);
        assert!(preview.args_schema["properties"].get("taskId").is_none());
        assert_eq!(preview.result_schema["x-a4n-schema-quality"], "typed");

        let resume = command_descriptor("resume_chapter_analysis_sync").unwrap();
        assert_eq!(resume.execution, "workflow");
        assert_eq!(resume.idempotency, "server_workflow");
        assert_eq!(resume.args_schema["additionalProperties"], false);
        assert_eq!(resume.result_schema["x-a4n-schema-quality"], "typed");

        let resume_preview =
            command_descriptor("preview_resume_chapter_analysis_sync_request").unwrap();
        assert_eq!(resume_preview.effect, "read");
        assert_eq!(resume_preview.args_schema["additionalProperties"], false);
        assert_eq!(
            resume_preview.result_schema["x-a4n-schema-quality"],
            "typed"
        );
    }

    #[test]
    fn chapter_scene_commands_expose_one_strict_contract() {
        let generate = command_descriptor("generate_chapter_scene").unwrap();
        assert_eq!(generate.execution, "task");
        assert_eq!(generate.args_schema["additionalProperties"], false);
        assert_eq!(
            generate.args_schema["properties"]["input"]["additionalProperties"],
            false
        );
        assert!(generate.args_schema["properties"].get("userId").is_none());
        assert!(
            generate.args_schema["properties"]
                .get("chapterId")
                .is_none()
        );
        assert_eq!(generate.result_schema["x-a4n-schema-quality"], "typed");

        let preview = command_descriptor("preview_chapter_scene_request").unwrap();
        assert_eq!(preview.effect, "read");
        assert_eq!(preview.args_schema["additionalProperties"], false);
        assert!(preview.args_schema["properties"].get("userId").is_none());
        assert!(preview.args_schema["properties"].get("chapterId").is_none());
        assert_eq!(preview.result_schema["x-a4n-schema-quality"], "typed");

        for command in [
            "list_chapter_scenes",
            "update_chapter_scene",
            "delete_chapter_scene",
            "reorder_chapter_scenes",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_ne!(descriptor.args_schema["x-a4n-schema-quality"], "opaque");
            assert_eq!(descriptor.args_schema["additionalProperties"], false);
            assert_ne!(descriptor.result_schema["x-a4n-schema-quality"], "opaque");
        }
    }

    #[test]
    fn task_workflow_commands_expose_generated_contracts_without_identity_args() {
        for command in ["get_queue_stats", "list_active_workflows"] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(descriptor.args_schema["additionalProperties"], false);
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([])
            );
            assert!(descriptor.args_schema["properties"].get("userId").is_none());
            assert_eq!(descriptor.result_schema["x-a4n-schema-quality"], "typed");
        }

        let cancel_task = command_descriptor("cancel_task").unwrap();
        assert_eq!(cancel_task.args_schema["additionalProperties"], false);
        assert!(
            cancel_task.args_schema["properties"]
                .get("taskId")
                .is_some()
        );
        assert!(
            cancel_task.args_schema["properties"]
                .get("userId")
                .is_none()
        );
        assert_eq!(cancel_task.result_schema["x-a4n-schema-quality"], "typed");

        let history = command_descriptor("get_task_history").unwrap();
        assert_eq!(history.args_schema["additionalProperties"], false);
        assert!(history.args_schema["properties"].get("projectId").is_some());
        assert!(history.args_schema["properties"].get("userId").is_none());
        assert_eq!(history.result_schema["x-a4n-schema-quality"], "typed");

        for command in [
            "list_workflow_runs",
            "get_workflow_run_detail",
            "resume_workflow_run",
            "get_workflow_progress_events",
            "cancel_workflow",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_ne!(descriptor.args_schema["x-a4n-schema-quality"], "opaque");
            assert!(descriptor.args_schema["properties"].get("input").is_some());
            assert!(descriptor.args_schema["properties"].get("userId").is_none());
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([])
            );
            assert_ne!(descriptor.result_schema["x-a4n-schema-quality"], "opaque");
        }

        let cancel_generation = command_descriptor("cancel_generation_task").unwrap();
        assert_eq!(cancel_generation.args_schema["additionalProperties"], false);
        assert!(
            cancel_generation.args_schema["properties"]
                .get("input")
                .is_some()
        );
        assert!(
            cancel_generation.args_schema["properties"]
                .get("userId")
                .is_none()
        );
        assert_eq!(
            cancel_generation.result_schema["x-a4n-schema-quality"],
            "typed"
        );
    }

    #[test]
    fn manga_page_prompt_contract_stays_on_the_canonical_command() {
        assert!(command_descriptor("list_manga_panels").is_none());
        let descriptor = command_descriptor("list_manga_page_prompts").unwrap();
        assert_eq!(descriptor.domain, "manga");
        assert_eq!(descriptor.effect, "read");
        assert_eq!(descriptor.result_schema["x-a4n-schema-quality"], "typed");
    }

    #[test]
    fn chapter_video_preview_uses_the_shared_override_contract() {
        let descriptor = command_descriptor("preview_chapter_video_request").unwrap();
        assert_eq!(
            descriptor.args_schema["x-a4n-schema-quality"],
            "typed-input"
        );
        let prompt_override = &descriptor.args_schema["properties"]["promptOverride"];
        assert_eq!(prompt_override["type"], "object");
        assert_eq!(prompt_override["additionalProperties"], false);
    }

    #[test]
    fn update_profile_uses_authenticated_identity_and_non_null_email() {
        let descriptor = command_descriptor("update_user_profile").unwrap();
        assert_eq!(
            descriptor.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert!(descriptor.args_schema["properties"].get("userId").is_none());
        assert_eq!(
            descriptor.args_schema["properties"]["input"]["properties"]["email"]["type"],
            "string"
        );
    }

    #[test]
    fn change_password_uses_authenticated_identity_and_unit_result() {
        let descriptor = command_descriptor("change_user_password").unwrap();
        assert_eq!(
            descriptor.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert!(descriptor.args_schema["properties"].get("userId").is_none());
        assert_eq!(descriptor.result_schema["x-a4n-schema-quality"], "typed");
    }

    #[test]
    fn session_commands_use_one_authenticated_identity() {
        assert!(command_descriptor("logout_user").is_none());

        let current_user = command_descriptor("get_current_user").unwrap();
        assert_eq!(
            current_user.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert_eq!(current_user.args_schema["additionalProperties"], false);

        let list = command_descriptor("list_sessions").unwrap();
        assert_eq!(
            list.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert_eq!(list.args_schema["additionalProperties"], false);

        let revoke = command_descriptor("revoke_session").unwrap();
        assert_eq!(
            revoke.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert_eq!(
            revoke.args_schema["properties"]["input"]["properties"]["session_id"]["type"],
            "string"
        );
        assert_eq!(revoke.result_schema["x-a4n-schema-quality"], "typed");
    }

    #[test]
    fn project_commands_use_authenticated_identity_without_user_id_args() {
        for command in [
            "list_projects",
            "create_project",
            "create_derived_project",
            "create_freeform_video_project",
            "duplicate_project",
            "export_project_family_data",
            "export_derived_project_data",
            "import_project_family_upload",
            "export_video_studio_project_data",
            "import_video_studio_project_upload",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([]),
                "{command}"
            );
            assert!(
                descriptor.args_schema["properties"].get("userId").is_none(),
                "{command}"
            );
        }

        for command in [
            "export_project_family_data",
            "export_derived_project_data",
            "import_project_family_upload",
            "export_video_studio_project_data",
            "import_video_studio_project_upload",
        ] {
            assert_eq!(
                command_descriptor(command).unwrap().args_schema["additionalProperties"],
                false,
                "{command}"
            );
            assert_eq!(
                command_descriptor(command).unwrap().result_schema["x-a4n-schema-quality"],
                "typed",
                "{command}"
            );
        }
        assert_eq!(
            command_descriptor("import_project_family_upload")
                .unwrap()
                .args_schema,
            command_descriptor("import_video_studio_project_upload")
                .unwrap()
                .args_schema
        );
        assert!(command_descriptor("import_project_full_upload").is_none());

        for command in ["update_project", "branch_project"] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!(["projectId"]),
                "{command}"
            );
            assert!(
                descriptor.args_schema["properties"].get("userId").is_none(),
                "{command}"
            );
        }

        for command in [
            "get_project",
            "get_project_asset_dependency_graph",
            "get_project_asset_dependency_graph_node_detail",
            "get_project_status_machine",
            "list_chapters",
            "list_images_by_project",
            "list_source_snapshot_chapters",
            "delete_project",
            "reset_project_generated_data",
            "sync_blueprint_materials",
            "sync_outline_materials",
            "forward_project_status",
            "preview_rollback",
            "rollback_project_status",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!(["projectId"]),
                "{command}"
            );
            assert!(
                descriptor.args_schema["properties"].get("userId").is_none(),
                "{command}"
            );
            assert_eq!(
                descriptor.args_schema["additionalProperties"], false,
                "{command}"
            );
        }

        for command in [
            "get_project_asset_dependency_graph",
            "get_project_asset_dependency_graph_node_detail",
        ] {
            assert_eq!(
                command_descriptor(command).unwrap().result_schema["x-a4n-schema-quality"],
                "typed",
                "{command}"
            );
        }
    }

    #[test]
    fn collaboration_commands_separate_caller_from_target_identity() {
        for command in [
            "list_my_project_invites",
            "accept_project_invite",
            "decline_project_invite",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([]),
                "{command}"
            );
            assert!(descriptor.args_schema["properties"].get("userId").is_none());
        }

        for command in [
            "list_project_members",
            "invite_project_member",
            "list_project_invites",
            "revoke_project_invite",
            "update_project_member_role",
            "remove_project_member",
            "leave_project",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!(["projectId"]),
                "{command}"
            );
            assert!(descriptor.args_schema["properties"].get("userId").is_none());
        }
    }

    #[test]
    fn provider_defaults_use_one_strict_admin_contract() {
        assert!(command_descriptor("activate_image_config").is_none());
        assert!(command_descriptor("activate_video_config").is_none());

        for command in [
            "set_default_embedding_config",
            "set_default_image_config",
            "set_default_llm_config",
            "set_default_video_config",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(descriptor.auth, "admin");
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!(["callerUserId"])
            );
            assert_eq!(descriptor.args_schema["additionalProperties"], false);
            assert_eq!(descriptor.result_schema["x-a4n-schema-quality"], "typed");
        }
    }
}
