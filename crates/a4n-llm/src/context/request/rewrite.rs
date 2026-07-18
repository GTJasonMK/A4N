use super::super::contracts::{LlmContextManifest, PlannedContextPack};
use super::super::manifest::render_context_manifest_user_content;
use crate::provider::{ChatMessage, ChatRequest, MessageRole};
use a4n_core::error::AppError;

pub fn replace_request_user_content_from_manifest(
    request: &mut ChatRequest,
    manifest: &LlmContextManifest,
    selected_packs: &[PlannedContextPack],
) -> Result<(), AppError> {
    let non_system_count = request
        .messages
        .iter()
        .filter(|message| !matches!(message.role, MessageRole::System))
        .count();
    let has_non_user_message = request
        .messages
        .iter()
        .any(|message| !matches!(message.role, MessageRole::System | MessageRole::User));
    if non_system_count > 1 || has_non_user_message {
        return Err(AppError::InvalidArgument(
            "LLM 上下文 manifest 只能重写 system + 单 user 结构的请求".to_string(),
        ));
    }
    let user_content = render_context_manifest_user_content(manifest, selected_packs)?;
    let mut messages: Vec<ChatMessage> = request
        .messages
        .iter()
        .filter(|message| matches!(message.role, MessageRole::System))
        .cloned()
        .collect();
    messages.push(ChatMessage::user(user_content));
    request.messages = messages;
    Ok(())
}
