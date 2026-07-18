use super::super::{
    ChatRequest, GeneratePortraitPromptInput, context_aware_chat_request_trace_json,
    generation_checkpoint_fingerprint_sha256,
};
use a4n_core::models::character::protagonist_profile;
use serde_json::{Value, json};

pub(super) fn portrait_prompt_workflow_fingerprint(
    input: &GeneratePortraitPromptInput,
    profile: &protagonist_profile::Model,
    profile_context: &Value,
    project_context: Option<&Value>,
    request: &ChatRequest,
) -> Result<Value, String> {
    Ok(json!({
        "workflow": "portrait_prompt_generation",
        "schema_version": "portrait_prompt_workflow_v1",
        "project_id": input.project_id,
        "profile": {
            "id": profile.id,
            "name": profile.name,
            "appearance": profile.appearance,
            "gender": profile.gender,
            "age": profile.age,
            "personality": profile.personality,
            "background": profile.background,
            "base_attributes": profile.base_attributes,
            "current_state": profile.current_state,
            "character_arc": profile.character_arc,
            "voice_fingerprint": profile.voice_fingerprint},
        "portrait_type": input.portrait_type,
        "variables": input.variables.clone(),
        "profile_context": profile_context,
        "project_context": project_context.cloned().unwrap_or_else(|| json!({})),
        "request": context_aware_chat_request_trace_json(request)
            .map_err(|err| format!("人物三视图提示词请求序列化失败: {err}"))?}))
}

pub(super) fn portrait_prompt_business_key(fingerprint: &Value) -> Result<String, String> {
    Ok(format!(
        "portrait_prompt_generation:{}",
        generation_checkpoint_fingerprint_sha256(fingerprint).map_err(|err| err.to_string())?
    ))
}
