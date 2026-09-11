use super::super::contracts::LlmContextManifest;
use super::super::manifest::{render_context_manifest_user_content, validate_context_manifest};
use crate::provider::ChatRequest;
use a4n_error::AppError;

pub fn context_aware_system_user_request(
    system_prompt: impl Into<String>,
    manifest: LlmContextManifest,
    temperature: Option<f32>,
    max_tokens: Option<usize>,
    max_tokens_profile: Option<String>,
) -> Result<ChatRequest, AppError> {
    validate_context_manifest(&manifest)?;
    let preview_user_content = render_context_manifest_user_content(&manifest, &[])?;
    Ok(ChatRequest::system_user(
        system_prompt,
        preview_user_content,
        temperature,
        max_tokens,
        max_tokens_profile,
    )
    .with_context_manifest_json(serde_json::to_value(&manifest).map_err(AppError::Serialization)?))
}
