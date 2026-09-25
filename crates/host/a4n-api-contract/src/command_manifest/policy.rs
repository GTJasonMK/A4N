use crate::agent::{
    AgentToolCostClass, AgentToolExposure, AgentToolReplayPolicy, AgentToolReversibility,
    AgentToolRisk,
};

use super::model::{AgentPolicySpec, Effect, Identity};

pub(crate) const NO_CONTEXT: &[&str] = &[];
pub(crate) const PROJECT_CONTEXT: &[&str] = &["projectId"];
pub(crate) const CHAPTER_CONTEXT: &[&str] = &["chapterId"];
pub(crate) const PROJECT_CHAPTER_CONTEXT: &[&str] = &["projectId", "chapterId"];

pub(crate) const PUBLIC_IDENTITY: Identity = Identity {
    auth: "public",
    context_fields: NO_CONTEXT,
};
pub(crate) const ADMIN_IDENTITY: Identity = Identity {
    auth: "admin",
    context_fields: NO_CONTEXT,
};
pub(crate) const READ_EFFECT: Effect = Effect {
    effect: "read",
    idempotency: "safe",
};
pub(crate) const DESTRUCTIVE_EFFECT: Effect = Effect {
    effect: "destructive",
    idempotency: "none",
};
pub(crate) const AGENT_READ: AgentPolicySpec = AgentPolicySpec {
    exposure: AgentToolExposure::Discoverable,
    risk: AgentToolRisk::Low,
    reversibility: AgentToolReversibility::Reversible,
    replay_policy: AgentToolReplayPolicy::Safe,
    cost_class: AgentToolCostClass::None,
    tags: NO_CONTEXT,
};
pub(crate) const AGENT_GUARDED_WRITE: AgentPolicySpec = AgentPolicySpec {
    exposure: AgentToolExposure::Discoverable,
    risk: AgentToolRisk::High,
    reversibility: AgentToolReversibility::Unknown,
    replay_policy: AgentToolReplayPolicy::Never,
    cost_class: AgentToolCostClass::None,
    tags: NO_CONTEXT,
};
pub(crate) const CAPABILITIES_EMBEDDING: &[&str] = &["embedding"];
pub(crate) const CAPABILITIES_EMBEDDING_RAG: &[&str] = &["embedding", "rag"];
pub(crate) const CAPABILITIES_EXPORT: &[&str] = &["export"];
pub(crate) const CAPABILITIES_IMAGE: &[&str] = &["image"];
pub(crate) const CAPABILITIES_IMPORT: &[&str] = &["import"];
pub(crate) const CAPABILITIES_LLM: &[&str] = &["llm"];
pub(crate) const CAPABILITIES_PROMPT: &[&str] = &["prompt"];
pub(crate) const CAPABILITIES_PROMPT_IMAGE: &[&str] = &["prompt", "image"];
pub(crate) const CAPABILITIES_PROMPT_LLM: &[&str] = &["prompt", "llm"];
pub(crate) const CAPABILITIES_PROMPT_LLM_IMAGE: &[&str] = &["prompt", "llm", "image"];
pub(crate) const CAPABILITIES_RAG: &[&str] = &["rag"];
pub(crate) const CAPABILITIES_VIDEO: &[&str] = &["video"];
pub(crate) const CAPABILITIES_VIDEO_EXPORT: &[&str] = &["video", "export"];
pub(crate) const CAPABILITIES_VIDEO_IMPORT: &[&str] = &["video", "import"];
pub(crate) const FILE_INPUTS_SET_BASE64_SET_TEXT: &[&str] =
    &["使用 --set-base64/--set-text 将本地文件写入实际输入字段"];
pub(crate) const FILE_OUTPUTS_SAVE_BASE64_SAVE_TEXT: &[&str] =
    &["使用 --save-base64/--save-text 原子保存响应字段"];
pub(crate) const REFERENCES_ARTIFACTS: &[&str] = &["artifacts"];
pub(crate) const REFERENCES_CHAPTER_IDS: &[&str] = &["chapter_ids"];
pub(crate) const REFERENCES_CHAPTER_IDS_TASK_IDS: &[&str] = &["chapter_ids", "task_ids"];
pub(crate) const REFERENCES_GALGAME_CHOICE_IDS: &[&str] = &["galgame_choice_ids"];
pub(crate) const REFERENCES_GALGAME_LINE_IDS: &[&str] = &["galgame_line_ids"];
pub(crate) const REFERENCES_GALGAME_SCENE_IDS: &[&str] = &["galgame_scene_ids"];
pub(crate) const REFERENCES_GALGAME_SCRIPT_IDS: &[&str] = &["galgame_script_ids"];
pub(crate) const REFERENCES_GENERATION_SPEC_IDS: &[&str] = &["generation_spec_ids"];
pub(crate) const REFERENCES_GENERATION_SPEC_IDS_MEDIA_ASSET_IDS: &[&str] =
    &["generation_spec_ids", "media_asset_ids"];
pub(crate) const REFERENCES_MEDIA_BINDING_IDS_MEDIA_ASSET_IDS: &[&str] =
    &["media_binding_ids", "media_asset_ids"];
pub(crate) const REFERENCES_PROJECT_IDS: &[&str] = &["project_ids"];
pub(crate) const REFERENCES_TASK_IDS: &[&str] = &["task_ids"];
pub(crate) const REFERENCES_WORKFLOW_RUN_IDS: &[&str] = &["workflow_run_ids"];
pub(crate) const PROVIDER_SECRET_POINTERS: &[&str] =
    &["/input/api_key_encrypted", "/input/api_key"];
pub(crate) const OAUTH_CODE_SECRET_POINTERS: &[&str] = &["/input/code", "/input/code_verifier"];
pub(crate) const CHANGE_PASSWORD_SECRET_POINTERS: &[&str] =
    &["/input/old_password", "/input/new_password"];
pub(crate) const PASSWORD_SECRET_POINTERS: &[&str] = &["/input/password"];
pub(crate) const REFRESH_TOKEN_SECRET_POINTERS: &[&str] = &["/input/refresh_token"];
pub(crate) const RUNTIME_SECRET_POINTERS: &[&str] = &[
    "/runtime_config_snapshot/llm/api_key_encrypted",
    "/runtime_config_snapshot/image/api_key_encrypted",
    "/runtime_config_snapshot/video/api_key_encrypted",
    "/runtime_config_snapshot/embedding/api_key_encrypted",
    "/runtime_config_snapshot/web_search/api_key_encrypted",
    "/runtime_config_snapshot/system_llm_api_key_encrypted",
    "/runtime_config_snapshot/system_image_api_key_encrypted",
    "/runtime_config_snapshot/system_video_api_key_encrypted",
];
