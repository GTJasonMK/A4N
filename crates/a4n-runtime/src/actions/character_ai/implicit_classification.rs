use crate::actions::json_contract::reject_unknown_fields;
use crate::actions::llm_request::PACK_KIND_IMPLICIT_CLASSIFICATION_CONTEXT;
use a4n_engine::prompt::request::build_json_task_user_message;
use serde_json::{Value, json};
use std::collections::BTreeSet;

use super::json_utils::{
    category_storage_key, required_array, required_i32, required_string, required_value,
};
use super::state_projection::apply_attribute_to_state;

pub(super) fn set_behavior_classification_results(
    behavior: &mut Value,
    classification_results: Value,
) -> Result<(), String> {
    let object = behavior
        .as_object_mut()
        .ok_or_else(|| "behavior 必须是 JSON 对象".to_string())?;
    object.insert("classification_results".to_string(), classification_results);
    Ok(())
}

#[cfg(test)]
pub(super) fn build_implicit_classification_prompt(
    profile_name: &str,
    behavior: &Value,
    implicit_attributes: &Value,
) -> Result<String, String> {
    let chapter = required_i32(behavior, "chapter_number")?;
    let behavior_type = required_string(behavior, "behavior_type")?;
    let description = required_string(behavior, "behavior_description")?;
    let original_text = required_string(behavior, "original_text")?;
    let behavior_tags = required_value(behavior, "behavior_tags")?;
    let payload = implicit_classification_payload(
        profile_name,
        json!({
            "behavior": {
                "chapter_number": chapter,
                "behavior_type": behavior_type,
                "behavior_description": description,
                "behavior_tags": behavior_tags,
                "original_text": original_text
            },
            "current_implicit_attributes": implicit_attributes,
            "current_implicit_attribute_keys": implicit_attribute_keys(implicit_attributes)?,
        }),
    );
    build_implicit_classification_prompt_from_payload(payload)
}

pub(super) fn build_implicit_classification_manifest_user_prompt(
    profile_name: &str,
    behavior: &Value,
) -> Result<String, String> {
    let chapter = required_i32(behavior, "chapter_number")?;
    let behavior_type = required_string(behavior, "behavior_type")?;
    let description = required_string(behavior, "behavior_description")?;
    let original_text = required_string(behavior, "original_text")?;
    let behavior_tags = required_value(behavior, "behavior_tags")?;
    let payload = implicit_classification_payload(
        profile_name,
        json!({
            "context_refs": {
                "current_implicit_attributes": format!("稳定上下文包 {PACK_KIND_IMPLICIT_CLASSIFICATION_CONTEXT}.current_implicit_attributes"),
                "current_implicit_attribute_keys": format!("稳定上下文包 {PACK_KIND_IMPLICIT_CLASSIFICATION_CONTEXT}.current_implicit_attribute_keys")
            },
            "behavior": {
                "chapter_number": chapter,
                "behavior_type": behavior_type,
                "behavior_description": description,
                "behavior_tags": behavior_tags,
                "original_text": original_text
            },
        }),
    );
    build_implicit_classification_prompt_from_payload(payload)
}

pub(super) fn implicit_classification_stable_context(
    implicit_attributes: &Value,
) -> Result<Value, String> {
    Ok(json!({
        "current_implicit_attributes": implicit_attributes,
        "current_implicit_attribute_keys": implicit_attribute_keys(implicit_attributes)?,
    }))
}

fn implicit_classification_payload(profile_name: &str, input: Value) -> Value {
    json!({
        "task": {
            "name": "implicit_attribute_classification",
            "instruction": "判断行为是否符合角色当前已有的隐性属性；如发现新隐性属性，只能放入 suggested_new_attributes"
        },
        "character": {
            "profile_name": profile_name
        },
        "input": input,
        "hard_rules": [
            "classifications 的每个 key 必须逐字来自 current_implicit_attribute_keys，不能自行创造 key",
            "行为体现的新隐性属性必须写入 suggested_new_attributes，绝不能作为 classifications key",
            "suggested_new_attributes 每项只能输出 key 和 value；不要输出 evidence，证据由系统使用当前 behavior.original_text 回填",
            "没有任何当前隐性属性适用时，classifications 必须是 {}"
        ],
        "output_contract": {
            "format": "json_object",
            "required_fields": {
                "classifications": "对象，key 只能来自 current_implicit_attributes；value 只能是 conform 或 non-conform",
                "reasoning": "字符串",
                "suggested_new_attributes": "数组，可为空；每项必须且只能包含 key / value"
            },
            "forbidden": [
                "Markdown 代码块",
                "解释性前后缀",
                "把新属性写入 classifications",
                "suggested_new_attributes[].evidence"
            ]
        }
    })
}

fn build_implicit_classification_prompt_from_payload(payload: Value) -> Result<String, String> {
    build_json_task_user_message(
        "执行行为隐性属性分类",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
    .map_err(|e| format!("行为隐性属性分类上下文序列化失败: {e}"))
}

pub(super) fn parse_implicit_classification_output(
    content: &str,
    implicit_attributes: &Value,
) -> Result<Value, String> {
    let value: Value = serde_json::from_str(content.trim())
        .map_err(|e| format!("行为隐性属性分类结果必须是 JSON 对象: {e}"))?;
    if !value.is_object() {
        return Err("行为隐性属性分类结果必须是 JSON 对象".to_string());
    }
    reject_unknown_fields(
        &value,
        "implicit_classification",
        &["classifications", "reasoning", "suggested_new_attributes"],
    )?;

    let allowed_keys = implicit_attributes
        .as_object()
        .ok_or_else(|| "当前隐性属性必须是 JSON 对象".to_string())?;
    let classifications = normalize_implicit_classifications(
        required_value(&value, "classifications")?,
        allowed_keys,
    )?;
    let reasoning = required_string(&value, "reasoning")?;
    let suggested_new_attributes =
        normalize_suggested_new_attributes(required_value(&value, "suggested_new_attributes")?)?;

    Ok(json!({
        "classifications": classifications,
        "reasoning": reasoning,
        "suggested_new_attributes": suggested_new_attributes,
    }))
}

fn normalize_implicit_classifications(
    value: &Value,
    allowed_keys: &serde_json::Map<String, Value>,
) -> Result<Value, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "行为隐性属性分类字段 classifications 必须是 JSON 对象".to_string())?;
    let mut normalized = serde_json::Map::new();
    for (key, value) in object {
        let key = key.trim();
        if key.is_empty() {
            return Err("行为隐性属性分类 classifications 包含空属性 key".to_string());
        }
        if !allowed_keys.contains_key(key) {
            return Err(format!(
                "行为隐性属性分类 classifications.{key} 不属于当前隐性属性；新属性必须放入 suggested_new_attributes"
            ));
        }
        let Some(text) = value.as_str().map(str::trim) else {
            return Err(format!(
                "行为隐性属性分类 classifications.{key} 必须是 conform/non-conform"
            ));
        };
        if !matches!(text, "conform" | "non-conform") {
            return Err(format!(
                "行为隐性属性分类 classifications.{key} 必须是 conform/non-conform"
            ));
        }
        if normalized
            .insert(key.to_string(), Value::String(text.to_string()))
            .is_some()
        {
            return Err(format!("行为隐性属性分类 classifications.{key} 重复"));
        }
    }
    Ok(Value::Object(normalized))
}

fn normalize_suggested_new_attributes(value: &Value) -> Result<Value, String> {
    let items = value
        .as_array()
        .ok_or_else(|| "行为隐性属性分类字段 suggested_new_attributes 必须是数组".to_string())?;
    let mut normalized = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        if !item.is_object() {
            return Err(format!(
                "suggested_new_attributes[{index}] 必须是 JSON 对象"
            ));
        }
        if item.get("evidence").is_some() {
            return Err(format!(
                "suggested_new_attributes[{index}].evidence 已废弃；证据由系统使用当前 behavior.original_text 回填"
            ));
        }
        reject_unknown_fields(
            item,
            &format!("suggested_new_attributes[{index}]"),
            &["key", "value"],
        )?;
        let key = required_string(item, "key")?;
        let suggested_value = required_value(item, "value")?;
        if suggested_value.is_null() {
            return Err(format!("suggested_new_attributes[{index}].value 不能为空"));
        }
        normalized.push(json!({
            "key": key,
            "value": suggested_value.clone(),
        }));
    }
    Ok(Value::Array(normalized))
}

fn implicit_attribute_keys(implicit_attributes: &Value) -> Result<Vec<String>, String> {
    let object = implicit_attributes
        .as_object()
        .ok_or_else(|| "当前隐性属性必须是 JSON 对象".to_string())?;
    Ok(object.keys().cloned().collect())
}

pub(super) fn apply_implicit_classification_to_behavior(
    behavior: &mut Value,
    parsed: &Value,
    current_state: &mut Value,
    suggested_keys: &mut BTreeSet<String>,
    sequence: usize,
) -> Result<Vec<Value>, String> {
    let chapter = required_i32(behavior, "chapter_number")?;
    let behavior_description = required_string(behavior, "behavior_description")?;
    let evidence = required_string(behavior, "original_text")?;
    let reasoning = required_string(parsed, "reasoning")?;
    let classifications = required_value(parsed, "classifications")?.clone();
    set_behavior_classification_results(behavior, classifications)?;

    let suggestions = required_array(parsed, "suggested_new_attributes")?;
    let mut applied = Vec::new();
    for item in suggestions {
        let key = required_string(item, "key")?;
        if state_has_attribute(current_state, "implicit", &key) || suggested_keys.contains(&key) {
            continue;
        }
        let value = required_value(item, "value")?.clone();
        apply_attribute_to_state(current_state, "implicit", &key, "add", value.clone())?;
        suggested_keys.insert(key.clone());
        applied.push(json!({
            "chapter_number": chapter,
            "key": key,
            "value": value,
            "evidence": evidence,
            "reasoning": reasoning,
            "behavior_description": behavior_description,
            "sequence": sequence,
        }));
    }
    Ok(applied)
}

fn state_has_attribute(state: &Value, category: &str, key: &str) -> bool {
    state
        .get(category_storage_key(category))
        .and_then(Value::as_object)
        .is_some_and(|attributes| attributes.contains_key(key))
}
