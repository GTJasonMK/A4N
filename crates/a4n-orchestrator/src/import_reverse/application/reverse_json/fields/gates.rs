use serde_json::Value;

pub(crate) fn reject_unknown_keys(
    value: &Value,
    allowed: &[&str],
    context: &str,
) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{context} 必须是 JSON 对象"))?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!(
                "{context} 包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            ));
        }
    }
    Ok(())
}
