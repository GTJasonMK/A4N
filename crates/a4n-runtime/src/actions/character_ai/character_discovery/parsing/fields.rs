use std::collections::BTreeSet;

use serde_json::Value;

use super::super::super::json_utils::required_array;
use super::super::identity::character_identity_key;

pub(super) fn required_positive_i64(json: &Value, key: &str, index: usize) -> Result<i64, String> {
    let number = json
        .get(key)
        .ok_or_else(|| format!("characters[{index}] 缺少字段: {key}"))?
        .as_i64()
        .ok_or_else(|| format!("characters[{index}].{key} 必须是 JSON 整数数字"))?;
    if number <= 0 {
        return Err(format!("characters[{index}].{key} 必须大于 0"));
    }
    Ok(number)
}

pub(super) fn normalize_aliases(json: &Value, key: &str, name: &str) -> Result<Value, String> {
    let values = required_array(json, key)?;
    let mut normalized = Vec::with_capacity(values.len());
    let mut seen = BTreeSet::new();
    let name_key = character_identity_key(name);
    for value in values {
        let text = value
            .as_str()
            .ok_or_else(|| format!("角色发现字段 {key} 必须是字符串数组"))?
            .trim();
        if text.is_empty() {
            continue;
        }
        let alias_key = character_identity_key(text);
        if alias_key == name_key {
            continue;
        }
        if seen.insert(alias_key) {
            normalized.push(Value::String(text.to_string()));
        }
    }
    Ok(Value::Array(normalized))
}

pub(super) fn required_discovery_object(json: &Value, key: &str) -> Result<Value, String> {
    json.get(key)
        .and_then(Value::as_object)
        .map(|object| Value::Object(object.clone()))
        .ok_or_else(|| format!("角色发现字段 {key} 必须是 JSON 对象"))
}

pub(super) fn value_or_null(value: Option<String>) -> Value {
    value.map(Value::String).unwrap_or(Value::Null)
}

pub(super) fn derive_discovery_summary(
    attributes: &Value,
    candidate_keys: &[&str],
) -> Option<String> {
    let object = attributes.as_object()?;
    candidate_keys.iter().find_map(|key| {
        object
            .get(*key)
            .and_then(summary_text_from_value)
            .filter(|text| !text.is_empty())
    })
}

fn summary_text_from_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.trim().to_string()),
        Value::Array(items) => {
            let parts = items
                .iter()
                .filter_map(summary_text_from_value)
                .collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join("、"))
        }
        Value::Object(object) => {
            let parts = object
                .iter()
                .filter_map(|(key, value)| {
                    summary_text_from_value(value).map(|text| format!("{key}:{text}"))
                })
                .collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join("；"))
        }
        _ => None,
    }
}
