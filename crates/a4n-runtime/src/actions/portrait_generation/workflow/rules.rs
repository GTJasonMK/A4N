use super::super::GeneratePortraitInput;
use a4n_core::models::character::protagonist_profile;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::provider::ChatRequest;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use serde_json::{Value, json};

pub(crate) fn portrait_generate_workflow_fingerprint(
    input: &GeneratePortraitInput,
    profile: &protagonist_profile::Model,
    expression: &str,
    request: &ChatRequest,
) -> Result<Value, String> {
    Ok(json!({
        "workflow": "portrait_generation",
        "mode": "full_body",
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
            "voice_fingerprint": profile.voice_fingerprint,
        },
        "style": input.style,
        "expression": expression,
        "pose": input.pose,
        "reference_image_ids": input.reference_image_ids,
        "request": context_aware_chat_request_trace_json(request)
            .map_err(|err| format!("立绘提示词请求序列化失败: {err}"))?,
    }))
}

pub(crate) fn portrait_generate_business_key(fingerprint: &Value) -> Result<String, String> {
    let serialized = serde_json::to_string(fingerprint)
        .map_err(|err| format!("立绘工作流指纹序列化失败: {err}"))?;
    Ok(format!(
        "portrait_generation:{}",
        source_content_sha256(&serialized)
    ))
}

pub(crate) fn portrait_prompt_checkpoint_fingerprint(
    workflow_fingerprint: &Value,
    request: &ChatRequest,
) -> Result<Value, String> {
    Ok(json!({
        "workflow": "portrait_generation",
        "stage": "prompt",
        "workflow_fingerprint": workflow_fingerprint,
        "request": context_aware_chat_request_trace_json(request)
            .map_err(|err| format!("立绘提示词请求序列化失败: {err}"))?,
    }))
}

pub(crate) fn portrait_image_workflow_fingerprint(
    workflow: &str,
    project_id: i64,
    source_portrait_id: i64,
    prompt: &str,
    reference_image_ids: &[i64],
    extra: Value,
) -> Value {
    json!({
        "workflow": workflow,
        "project_id": project_id,
        "source_portrait_id": source_portrait_id,
        "prompt": prompt,
        "reference_image_ids": reference_image_ids,
        "extra": extra,
    })
}

pub(crate) fn portrait_image_business_key(
    workflow: &str,
    fingerprint: &Value,
) -> Result<String, String> {
    let serialized = serde_json::to_string(fingerprint)
        .map_err(|err| format!("立绘图片工作流指纹序列化失败: {err}"))?;
    Ok(format!("{workflow}:{}", source_content_sha256(&serialized)))
}

pub(crate) fn missing_portrait_item_fingerprint(
    project_id: i64,
    profile: &protagonist_profile::Model,
    prompt: &str,
) -> Value {
    json!({
        "workflow": "missing_portrait_generation",
        "project_id": project_id,
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
            "voice_fingerprint": profile.voice_fingerprint,
        },
        "prompt": prompt,
        "portrait_type": "half_body",
    })
}
