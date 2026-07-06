use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(crate) fn ensure_object(value: &Value, path: &str) -> Result<(), String> {
    if value.is_object() {
        Ok(())
    } else {
        Err(format!("{path} 必须是 JSON 对象"))
    }
}

pub(crate) fn reject_unknown_fields(
    value: &Value,
    path: &str,
    allowed: &[&str],
) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{path} 必须是 JSON 对象"))?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!(
                "{path} 包含未支持字段 `{key}`；只允许输出：{}",
                allowed.join(", ")
            ));
        }
    }
    Ok(())
}

pub(crate) fn parse_llm_json_object(raw: &str, context: &str) -> Result<Value, String> {
    let value: Value = serde_json::from_str(raw.trim())
        .map_err(|e| format!("{context} 不是合法 JSON 对象: {e}"))?;
    if !value.is_object() {
        return Err(format!("{context} 必须是 JSON 对象"));
    }
    Ok(value)
}

pub(crate) fn required_array<'a>(
    value: &'a Value,
    key: &str,
    path: &str,
) -> Result<&'a Vec<Value>, String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{path}.{key} 必须是数组"))
}

pub(crate) fn required_array_value(value: &Value, key: &str, path: &str) -> Result<Value, String> {
    Ok(Value::Array(required_array(value, key, path)?.clone()))
}

pub(crate) fn required_object_value(value: &Value, key: &str, path: &str) -> Result<Value, String> {
    let item = value
        .get(key)
        .ok_or_else(|| format!("{path}.{key} 缺少字段"))?;
    if !item.is_object() {
        return Err(format!("{path}.{key} 必须是 JSON 对象"));
    }
    Ok(item.clone())
}

pub(crate) fn required_string(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let text = value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{path}.{key} 必须是字符串"))?
        .trim();
    if text.is_empty() {
        return Err(format!("{path}.{key} 不能为空"));
    }
    Ok(text.to_string())
}

pub(crate) fn optional_string(
    value: &Value,
    key: &str,
    path: &str,
) -> Result<Option<String>, String> {
    let Some(item) = value.get(key) else {
        return Ok(None);
    };
    if item.is_null() {
        return Ok(None);
    }
    let text = item
        .as_str()
        .ok_or_else(|| format!("{path}.{key} 必须是字符串或 null"))?
        .trim();
    Ok((!text.is_empty()).then(|| text.to_string()))
}

pub(crate) fn optional_positive_i64(
    value: &Value,
    key: &str,
    path: &str,
) -> Result<Option<i64>, String> {
    let Some(item) = value.get(key) else {
        return Ok(None);
    };
    if item.is_null() {
        return Ok(None);
    }
    let number = item
        .as_i64()
        .ok_or_else(|| format!("{path}.{key} 必须是 JSON 整数数字或 null"))?;
    if number <= 0 {
        return Err(format!("{path}.{key} 必须大于 0"));
    }
    Ok(Some(number))
}

pub(crate) fn string_array(
    value: &Value,
    key: &str,
    path: &str,
    require_non_empty: bool,
) -> Result<Vec<String>, String> {
    let values = required_array(value, key, path)?;
    let mut normalized = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, item) in values.iter().enumerate() {
        let text = item
            .as_str()
            .ok_or_else(|| format!("{path}.{key}[{index}] 必须是字符串"))?
            .trim();
        if text.is_empty() {
            continue;
        }
        if seen.insert(text.to_string()) {
            normalized.push(text.to_string());
        }
    }
    if require_non_empty && normalized.is_empty() {
        return Err(format!("{path}.{key} 不能为空"));
    }
    Ok(normalized)
}

pub(crate) fn string_array_value(
    value: &Value,
    key: &str,
    path: &str,
    require_non_empty: bool,
) -> Result<Value, String> {
    Ok(json!(string_array(value, key, path, require_non_empty)?))
}

pub(crate) fn positive_i64_array(
    value: &Value,
    key: &str,
    path: &str,
    require_non_empty: bool,
) -> Result<Vec<i64>, String> {
    let values = required_array(value, key, path)?;
    let mut normalized = Vec::new();
    let mut seen = BTreeSet::new();
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
