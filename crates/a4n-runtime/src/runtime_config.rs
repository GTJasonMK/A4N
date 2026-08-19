mod access;
mod contracts;
mod models;
mod redaction;
mod scope;

pub use access::{
    current_embedding_model, current_image_model, current_image_model_for_id,
    current_image_snapshot_scope_present, current_llm_max_tokens, current_llm_model,
    current_llm_model_for_id, current_llm_snapshot_scope_present, current_snapshot,
    current_system_image_api_key, current_system_llm_api_key, current_system_video_api_key,
    current_user_id, current_video_model, current_video_model_for_id,
    current_video_snapshot_scope_present, uses_client_desktop_embedding,
    uses_system_default_image_template, uses_system_default_llm_template,
    uses_system_default_video_template,
};
pub use contracts::{
    RuntimeConfigSnapshot, RuntimeEmbeddingConfig, RuntimeImageConfig, RuntimeLlmConfig,
    RuntimeVideoConfig,
};
pub use redaction::{redacted_current_snapshot_value, redacted_snapshot_value};
pub use scope::{request_scope, scope, snapshot_from_args};

#[cfg(test)]
mod tests;
