use serde_json::Value;

pub fn required_array<'a>(
    value: &'a Value,
    key: &str,
    path: &str,
) -> Result<&'a Vec<Value>, String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{path}.{key} 必须是数组"))
}

pub fn required_array_value(value: &Value, key: &str, path: &str) -> Result<Value, String> {
    Ok(Value::Array(required_array(value, key, path)?.clone()))
}

pub fn required_object_value(value: &Value, key: &str, path: &str) -> Result<Value, String> {
    let item = value
        .get(key)
        .ok_or_else(|| format!("{path}.{key} 缺少字段"))?;
    if !item.is_object() {
        return Err(format!("{path}.{key} 必须是 JSON 对象"));
    }
    Ok(item.clone())
}

pub fn required_string(value: &Value, key: &str, path: &str) -> Result<String, String> {
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

pub fn optional_string(value: &Value, key: &str, path: &str) -> Result<Option<String>, String> {
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

pub fn optional_positive_i64(value: &Value, key: &str, path: &str) -> Result<Option<i64>, String> {
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
