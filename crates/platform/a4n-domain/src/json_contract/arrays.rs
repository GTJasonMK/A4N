use std::collections::HashSet;

use serde_json::{Value, json};

use super::fields::{positive_i64_schema, required_array};

pub fn string_array(
    value: &Value,
    key: &str,
    path: &str,
    require_non_empty: bool,
) -> Result<Vec<String>, String> {
    let values = required_array(value, key, path)?;
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();
    for (index, item) in values.iter().enumerate() {
        let text = item
            .as_str()
            .ok_or_else(|| format!("{path}.{key}[{index}] 必须是字符串"))?
            .trim();
        if text.is_empty() {
            continue;
        }
        if seen.insert(text) {
            normalized.push(text.to_string());
        }
    }
    if require_non_empty && normalized.is_empty() {
        return Err(format!("{path}.{key} 不能为空"));
    }
    Ok(normalized)
}

pub fn string_array_value(
    value: &Value,
    key: &str,
    path: &str,
    require_non_empty: bool,
) -> Result<Value, String> {
    Ok(json!(string_array(value, key, path, require_non_empty)?))
}

pub fn positive_i64_array(
    value: &Value,
    key: &str,
    path: &str,
    require_non_empty: bool,
) -> Result<Vec<i64>, String> {
    let values = required_array(value, key, path)?;
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();
    for (index, item) in values.iter().enumerate() {
        let id = item
            .as_i64()
            .ok_or_else(|| format!("{path}.{key}[{index}] 必须是 JSON 整数数字"))?;
        if id <= 0 {
            return Err(format!("{path}.{key}[{index}] 必须大于 0"));
        }
        if seen.insert(id) {
            normalized.push(id);
        }
    }
    if require_non_empty && normalized.is_empty() {
        return Err(format!("{path}.{key} 不能为空"));
    }
    Ok(normalized)
}

pub fn positive_i64_array_schema(require_non_empty: bool) -> Value {
    json!({
        "type": "array",
        "items": positive_i64_schema(),
        "minItems": usize::from(require_non_empty),
    })
}
