use super::super::json_utils::{required_string, required_value};
use crate::actions::json_contract::reject_unknown_fields;
use serde_json::{Map, Value, json};

pub(in crate::actions::character_ai) fn parse_implicit_classification_output(
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
    allowed_keys: &Map<String, Value>,
) -> Result<Value, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "行为隐性属性分类字段 classifications 必须是 JSON 对象".to_string())?;
    let mut normalized = Map::new();
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
