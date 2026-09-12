use super::super::contracts::{
    ChatRequestStepInput, ChatRequestStepWithBudgetInput, ModelRequestPreviewStep,
};
use super::super::sanitize::{redact_data_uris_in_text, sanitize_preview_value};
use a4n_llm::context::preview_context_aware_chat_request;
use a4n_llm::provider::MessageRole;
use serde_json::json;

pub(crate) fn chat_request_step_with_budget(
    input: ChatRequestStepWithBudgetInput,
) -> Result<ModelRequestPreviewStep, String> {
    let ChatRequestStepWithBudgetInput {
        step:
            ChatRequestStepInput {
                key,
                title,
                prompt_name,
                state,
                note,
                mut request,
            },
        model_context_tokens,
        output_token_reserve,
        budget_note,
    } = input;
    request.max_tokens = Some(output_token_reserve);
    let context_preview = preview_context_aware_chat_request(
        &mut request,
        model_context_tokens,
        output_token_reserve,
    )
    .map_err(|err| err.to_string())?;
    let output_token_reserve = request
        .max_tokens
        .ok_or_else(|| "上下文预览未保留最终输出预算".to_string())?;
    let system_prompt = request
        .messages
        .iter()
        .find(|message| matches!(message.role, MessageRole::System))
        .map(|message| message.content.clone());
    let user_prompt = request
        .messages
        .iter()
        .rev()
        .find(|message| matches!(message.role, MessageRole::User))
        .map(|message| message.content.clone());
    let messages =
        sanitize_preview_value(serde_json::to_value(&request.messages).map_err(|e| e.to_string())?);
    let request =
        sanitize_preview_value(serde_json::to_value(&request).map_err(|e| e.to_string())?);
    let system_chars = system_prompt
        .as_deref()
        .map(str::chars)
        .map(Iterator::count);
    let user_chars = user_prompt.as_deref().map(str::chars).map(Iterator::count);
    let messages_text = messages.to_string();
    let request_text = request.to_string();
    let total_chars = messages_text.chars().count();
    let request_chars = request_text.chars().count();
    Ok(ModelRequestPreviewStep {
        key,
        title,
        kind: "llm_chat".to_string(),
        prompt_name,
        state,
        note: merge_preview_notes(note, budget_note),
        system_prompt: system_prompt.map(|text| redact_data_uris_in_text(&text)),
        user_prompt: user_prompt.map(|text| redact_data_uris_in_text(&text)),
        messages: Some(messages),
        request: Some(request),
        context_manifest: context_preview
            .as_ref()
            .map(|preview| sanitize_preview_value(preview.manifest_json.clone())),
        context_plan: context_preview
            .as_ref()
            .map(|preview| sanitize_preview_value(preview.plan_json.clone())),
        context_pack_refs: context_preview
            .as_ref()
            .map(|preview| sanitize_preview_value(preview.pack_refs_json.clone())),
        total_chars,
        char_counts: Some(json!({
            "system_chars": system_chars,
            "user_chars": user_chars,
            "messages_chars": total_chars,
            "messages_bytes": messages_text.len(),
            "request_chars": request_chars,
            "request_bytes": request_text.len(),
            "context_stable_prefix_tokens": context_preview.as_ref().map(|preview| preview.stable_prefix_tokens),
            "context_dynamic_delta_tokens": context_preview.as_ref().map(|preview| preview.dynamic_delta_tokens),
            "context_available_input_tokens": context_preview.as_ref().map(|preview| preview.available_input_tokens),
            "context_model_context_tokens": model_context_tokens,
            "context_output_token_reserve": output_token_reserve
        })),
    })
}

fn merge_preview_notes(note: Option<String>, budget_note: Option<String>) -> Option<String> {
    match (note, budget_note) {
        (Some(note), Some(budget_note)) if !budget_note.trim().is_empty() => {
            Some(format!("{note}；{budget_note}"))
        }
        (Some(note), _) => Some(note),
        (None, Some(budget_note)) if !budget_note.trim().is_empty() => Some(budget_note),
        (None, _) => None,
    }
}
