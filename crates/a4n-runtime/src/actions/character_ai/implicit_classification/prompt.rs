use super::super::json_utils::{required_i32, required_string, required_value};
use crate::actions::llm_request::PACK_KIND_IMPLICIT_CLASSIFICATION_CONTEXT;
use a4n_prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

#[cfg(test)]
pub(in crate::actions::character_ai) fn build_implicit_classification_prompt(
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

pub(in crate::actions::character_ai) fn build_implicit_classification_manifest_user_prompt(
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

pub(in crate::actions::character_ai) fn implicit_classification_stable_context(
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

fn implicit_attribute_keys(implicit_attributes: &Value) -> Result<Vec<String>, String> {
    let object = implicit_attributes
        .as_object()
        .ok_or_else(|| "当前隐性属性必须是 JSON 对象".to_string())?;
    Ok(object.keys().cloned().collect())
}
