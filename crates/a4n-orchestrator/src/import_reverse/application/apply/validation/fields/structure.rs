use serde_json::Value;

pub(in crate::import_reverse::application::apply) fn required_array_clone(
    value: &Value,
    key: &str,
    context: &str,
) -> Result<Value, String> {
    let field = value
        .get(key)
        .ok_or_else(|| format!("{context} 缺少必填数组字段 {key}"))?;
    if !field.is_array() {
        return Err(format!("{context}.{key} 必须是数组"));
    }
    Ok(field.clone())
}

pub(in crate::import_reverse::application::apply) fn required_object_clone(
    value: &Value,
    key: &str,
    context: &str,
) -> Result<Value, String> {
    let field = value
        .get(key)
        .ok_or_else(|| format!("{context} 缺少必填对象字段 {key}"))?;
    if !field.is_object() {
        return Err(format!("{context}.{key} 必须是对象"));
    }
    Ok(field.clone())
}
