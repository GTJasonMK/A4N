use serde_json::Value;

pub fn ensure_object(value: &Value, path: &str) -> Result<(), String> {
    if value.is_object() {
        Ok(())
    } else {
        Err(format!("{path} 必须是 JSON 对象"))
    }
}

pub fn reject_unknown_fields(value: &Value, path: &str, allowed: &[&str]) -> Result<(), String> {
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

pub fn parse_llm_json_object(raw: &str, context: &str) -> Result<Value, String> {
    let value = super::parse_strict_json_value(raw.trim())
        .map_err(|error| format!("{context} 不是合法 JSON 对象: {error}"))?;
    if !value.is_object() {
        return Err(format!("{context} 必须是 JSON 对象"));
    }
    Ok(value)
}
