//! 立绘生成工作流指纹与 business key 纯规则（无 DB、无 Host）。

use a4n_db::generation::generation_checkpoint_fingerprint_sha256;
use a4n_db_models::models::character::protagonist_profile;
use a4n_domain::source_text_hash as source_content_sha256;
use a4n_llm::provider::ChatRequest;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use serde_json::{Value, json};

/// 完整立绘生成工作流输入指纹。
pub fn portrait_generate_workflow_fingerprint(
    project_id: i64,
    profile: &protagonist_profile::Model,
    style: Option<&str>,
    expression: &str,
    pose: Option<&str>,
    reference_image_ids: &[i64],
    width: i32,
    height: i32,
    request: &ChatRequest,
) -> Result<Value, String> {
    Ok(json!({
        "workflow": "portrait_generation",
        "mode": "full_body",
        "project_id": project_id,
        "profile": profile_fingerprint_fields(profile),
        "style": style,
        "expression": expression,
        "pose": pose,
        "reference_image_ids": reference_image_ids,
        "width": width,
        "height": height,
        "request": context_aware_chat_request_trace_json(request)
            .map_err(|err| format!("立绘提示词请求序列化失败: {err}"))?,
    }))
}

/// 完整立绘生成 exclusive workflow business key。
pub fn portrait_generate_business_key(fingerprint: &Value) -> Result<String, String> {
    let serialized = serde_json::to_string(fingerprint)
        .map_err(|err| format!("立绘工作流指纹序列化失败: {err}"))?;
    Ok(format!(
        "portrait_generation:{}",
        source_content_sha256(&serialized)
    ))
}

/// 立绘提示词阶段 checkpoint 指纹。
pub fn portrait_prompt_checkpoint_fingerprint(
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

/// 立绘图片类工作流（再生成 / 年龄变体等）输入指纹。
pub fn portrait_image_workflow_fingerprint(
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

/// 立绘图片类工作流 business key。
pub fn portrait_image_business_key(workflow: &str, fingerprint: &Value) -> Result<String, String> {
    let serialized = serde_json::to_string(fingerprint)
        .map_err(|err| format!("立绘图片工作流指纹序列化失败: {err}"))?;
    Ok(format!("{workflow}:{}", source_content_sha256(&serialized)))
}

/// 缺失立绘批量生成单项指纹。
pub fn missing_portrait_item_fingerprint(
    project_id: i64,
    profile: &protagonist_profile::Model,
    prompt: &str,
    width: i32,
    height: i32,
) -> Value {
    json!({
        "workflow": "missing_portrait_generation",
        "project_id": project_id,
        "profile": profile_fingerprint_fields(profile),
        "prompt": prompt,
        "portrait_type": "half_body",
        "width": width,
        "height": height,
    })
}

/// 人物三视图提示词生成工作流输入指纹。
pub fn portrait_prompt_workflow_fingerprint(
    project_id: i64,
    profile: &protagonist_profile::Model,
    portrait_type: &str,
    variables: Option<&Value>,
    profile_context: &Value,
    project_context: Option<&Value>,
    request: &ChatRequest,
) -> Result<Value, String> {
    Ok(json!({
        "workflow": "portrait_prompt_generation",
        "schema_version": "portrait_prompt_workflow_v1",
        "project_id": project_id,
        "profile": profile_fingerprint_fields(profile),
        "portrait_type": portrait_type,
        "variables": variables.cloned(),
        "profile_context": profile_context,
        "project_context": project_context.cloned().unwrap_or_else(|| json!({})),
        "request": context_aware_chat_request_trace_json(request)
            .map_err(|err| format!("人物三视图提示词请求序列化失败: {err}"))?,
    }))
}

/// 人物三视图提示词生成 exclusive workflow business key。
pub fn portrait_prompt_business_key(fingerprint: &Value) -> Result<String, String> {
    Ok(format!(
        "portrait_prompt_generation:{}",
        generation_checkpoint_fingerprint_sha256(fingerprint).map_err(|err| err.to_string())?
    ))
}

fn profile_fingerprint_fields(profile: &protagonist_profile::Model) -> Value {
    json!({
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_llm::provider::{ChatMessage, ChatRequest};

    fn empty_request() -> ChatRequest {
        ChatRequest::new(vec![ChatMessage::user("hi")], None, None, None)
    }

    fn sample_profile() -> protagonist_profile::Model {
        let ts = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        protagonist_profile::Model {
            id: 9,
            project_id: 1,
            name: "测".into(),
            gender: None,
            age: None,
            base_attributes: json!({}),
            current_state: json!({}),
            character_arc: None,
            voice_fingerprint: None,
            appearance: None,
            background: None,
            personality: None,
            last_synced_chapter: 0,
            created_at: ts,
            updated_at: ts,
        }
    }

    #[test]
    fn business_key_is_stable_for_same_fingerprint() {
        let fp = json!({"workflow": "portrait_generation", "project_id": 1});
        let key = portrait_generate_business_key(&fp).expect("key");
        assert!(key.starts_with("portrait_generation:"));
        assert_eq!(key, portrait_generate_business_key(&fp).unwrap());
    }

    #[test]
    fn image_business_key_prefixes_workflow() {
        let fp = portrait_image_workflow_fingerprint("portrait_regen", 1, 2, "p", &[], json!({}));
        let key = portrait_image_business_key("portrait_regen", &fp).expect("key");
        assert!(key.starts_with("portrait_regen:"));
    }

    #[test]
    fn generate_fingerprint_includes_request_trace() {
        let fp = portrait_generate_workflow_fingerprint(
            1,
            &sample_profile(),
            Some("anime"),
            "neutral",
            None,
            &[1, 2],
            1024,
            1536,
            &empty_request(),
        )
        .expect("fingerprint");
        assert_eq!(fp["workflow"], "portrait_generation");
        assert_eq!(fp["profile"]["id"], 9);
        assert!(fp.get("request").is_some());
    }
}
