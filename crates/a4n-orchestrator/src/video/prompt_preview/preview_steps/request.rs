use serde_json::json;

use a4n_core::config::app_config::{DEFAULT_LLM_MAX_CONTEXT_TOKENS, DEFAULT_LLM_MAX_OUTPUT_TOKENS};
use a4n_core::error::AppError;
use a4n_llm::provider::{ChatRequest, MessageRole};

use a4n_llm::context::preview_context_aware_chat_request;
use a4n_llm::trace::context_aware_chat_request_trace_json;

use super::super::VideoPromptRequestPreviewStep;

pub(in crate::video::prompt_preview) fn step_from_request(
    key: &str,
    title: &str,
    prompt_name: &str,
    mut request: ChatRequest,
) -> Result<VideoPromptRequestPreviewStep, AppError> {
    let output_token_reserve = request.max_tokens.unwrap_or(DEFAULT_LLM_MAX_OUTPUT_TOKENS);
    let context_preview = preview_context_aware_chat_request(
        &mut request,
        DEFAULT_LLM_MAX_CONTEXT_TOKENS,
        output_token_reserve,
    )?;
    let system_prompt = request
        .messages
        .iter()
        .find(|message| matches!(message.role, MessageRole::System))
        .map(|message| message.content.clone())
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "视频提示词预览步骤 {prompt_name} 缺少 system message"
            ))
        })?;
    let user_prompt = request
        .messages
        .iter()
        .rev()
        .find(|message| matches!(message.role, MessageRole::User))
        .map(|message| message.content.clone())
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "视频提示词预览步骤 {prompt_name} 缺少 user message"
            ))
        })?;
    let total_chars = system_prompt.chars().count() + user_prompt.chars().count();
    let char_counts = json!({
        "system_chars": system_prompt.chars().count(),
        "user_chars": user_prompt.chars().count(),
        "total_prompt_chars": total_chars,
        "context_stable_prefix_tokens": context_preview.as_ref().map(|preview| preview.stable_prefix_tokens),
        "context_dynamic_delta_tokens": context_preview.as_ref().map(|preview| preview.dynamic_delta_tokens),
        "context_available_input_tokens": context_preview.as_ref().map(|preview| preview.available_input_tokens)
    });
    let messages = serde_json::to_value(&request.messages)?;
    let request = context_aware_chat_request_trace_json(&request)?;
    Ok(VideoPromptRequestPreviewStep {
        key: key.to_string(),
        title: title.to_string(),
        prompt_name: prompt_name.to_string(),
        state: "ready".to_string(),
        note: None,
        system_prompt: Some(system_prompt),
        user_prompt: Some(user_prompt),
        messages: Some(messages),
        request: Some(request),
        context_manifest: context_preview
            .as_ref()
            .map(|preview| preview.manifest_json.clone()),
        context_plan: context_preview
            .as_ref()
            .map(|preview| preview.plan_json.clone()),
        context_pack_refs: context_preview
            .as_ref()
            .map(|preview| preview.pack_refs_json.clone()),
        total_chars,
        char_counts: Some(char_counts),
    })
}

pub(in crate::video::prompt_preview) fn placeholder_step_from_request(
    key: &str,
    title: &str,
    prompt_name: &str,
    request: ChatRequest,
    note: &str,
) -> Result<VideoPromptRequestPreviewStep, AppError> {
    let mut step = step_from_request(key, title, prompt_name, request)?;
    step.state = "preview_placeholder".to_string();
    step.note = Some(note.to_string());
    Ok(step)
}

pub(in crate::video::prompt_preview) fn reused_step(
    key: &str,
    title: &str,
    prompt_name: &str,
    note: &str,
) -> VideoPromptRequestPreviewStep {
    VideoPromptRequestPreviewStep {
        key: key.to_string(),
        title: title.to_string(),
        prompt_name: prompt_name.to_string(),
        state: "reused".to_string(),
        note: Some(note.to_string()),
        system_prompt: None,
        user_prompt: None,
        messages: None,
        request: None,
        context_manifest: None,
        context_plan: None,
        context_pack_refs: None,
        total_chars: 0,
        char_counts: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_llm::provider::ChatMessage;

    fn chat_request(messages: Vec<ChatMessage>) -> ChatRequest {
        ChatRequest::new(messages, Some(0.2), Some(512), None)
    }

    fn message(role: MessageRole, content: &str) -> ChatMessage {
        ChatMessage::new(role, content)
    }

    #[test]
    fn step_from_request_rejects_missing_user_message() {
        let err = step_from_request(
            "segment_plan",
            "视频片段规划",
            "chapter_video_segment_plan",
            chat_request(vec![message(MessageRole::System, "system prompt")]),
        )
        .expect_err("missing user message must be visible");

        assert!(err.to_string().contains("缺少 user message"));
    }

    #[test]
    fn step_from_request_preserves_system_and_user_prompts() {
        let step = step_from_request(
            "segment_plan",
            "视频片段规划",
            "chapter_video_segment_plan",
            chat_request(vec![
                message(MessageRole::System, "system prompt"),
                message(MessageRole::User, "user prompt"),
            ]),
        )
        .expect("valid request should be previewed");

        assert_eq!(step.system_prompt.as_deref(), Some("system prompt"));
        assert_eq!(step.user_prompt.as_deref(), Some("user prompt"));
        assert_eq!(
            step.total_chars,
            "system prompt".chars().count() + "user prompt".chars().count()
        );
    }

    #[test]
    fn step_from_request_uses_message_roles_instead_of_fixed_positions() {
        let step = step_from_request(
            "segment_plan",
            "视频片段规划",
            "chapter_video_segment_plan",
            chat_request(vec![
                message(MessageRole::User, "old user prompt"),
                message(MessageRole::Assistant, "assistant draft"),
                message(MessageRole::System, "system prompt"),
                message(MessageRole::User, "latest user prompt"),
            ]),
        )
        .expect("valid request should be previewed by role");

        assert_eq!(step.system_prompt.as_deref(), Some("system prompt"));
        assert_eq!(step.user_prompt.as_deref(), Some("latest user prompt"));
    }
}
