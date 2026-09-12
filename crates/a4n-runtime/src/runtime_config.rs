mod access;
mod models;
mod redaction;
mod scope;

pub use a4n_api_contract::runtime_config::{
    RuntimeConfigSnapshot, RuntimeEmbeddingConfig, RuntimeImageConfig, RuntimeLlmConfig,
    RuntimeVideoConfig,
};
pub use access::{
    current_embedding_provider_config, current_image_provider_config_for_id,
    current_image_snapshot_scope_present, current_llm_provider_config,
    current_llm_provider_config_for_id, current_llm_snapshot_scope_present, current_snapshot,
    current_system_image_api_key, current_system_llm_api_key, current_system_video_api_key,
    current_user_id, current_video_provider_config, current_video_provider_config_for_id,
    current_video_snapshot_scope_present, uses_client_desktop_embedding,
    uses_system_default_image_template, uses_system_default_llm_template,
    uses_system_default_video_template,
};
pub(crate) use redaction::redacted_current_snapshot_value;
pub use scope::{request_scope, scope, snapshot_from_args};

#[cfg(test)]
mod tests;
