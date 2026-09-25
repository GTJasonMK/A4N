use super::super::contracts::{LlmContextManifest, PlannedContextPack};
use super::super::manifest::render_context_manifest_user_content_with_cache_boundary;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatMessage, ChatRequest, MessageRole};

pub fn replace_request_user_content_from_manifest(
    request: &mut ChatRequest,
    manifest: &LlmContextManifest,
    selected_packs: &[PlannedContextPack],
) -> Result<(), AppError> {
    let user_index = request
        .messages
        .iter()
        .position(|message| !matches!(message.role, MessageRole::System))
        .unwrap_or(request.messages.len());
    if request
        .messages
        .get(user_index)
        .is_some_and(|message| !matches!(message.role, MessageRole::User))
    {
        return Err(AppError::InvalidArgument(
            "LLM 上下文 manifest 只能重写首条 user 消息，不能覆盖 assistant 或 tool 历史"
                .to_string(),
        ));
    }
    let (user_content, stable_prefix_lens) =
        render_context_manifest_user_content_with_cache_boundary(manifest, selected_packs)?;
    // The manifest owns the first user message only. Immutable observations
    // and later conversation messages remain byte-for-byte intact.
    if user_index == request.messages.len() {
        request.messages.push(ChatMessage::user(user_content));
    } else {
        request.messages[user_index].content = user_content;
    }
    request
        .cache_breakpoints
        .retain(|point| point.message_index != user_index);
    for boundary in stable_prefix_lens {
        request.add_cache_breakpoint(user_index, boundary)?;
    }
    Ok(())
}

pub fn context_manifest_dynamic_messages(
    request: &ChatRequest,
    manifest: &LlmContextManifest,
) -> Result<Vec<ChatMessage>, AppError> {
    let mut preview = request.clone();
    replace_request_user_content_from_manifest(&mut preview, manifest, &[])?;
    Ok(preview
        .messages
        .into_iter()
        .filter(|message| !matches!(message.role, MessageRole::System))
        .collect())
}
