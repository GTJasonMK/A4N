use serde_json::{Value, json};

pub(super) fn required_i64(json: &Value, key: &str) -> Result<i64, String> {
    json.get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("角色 AI 字段 {key} 必须是整数"))
}

pub(super) fn required_i32(json: &Value, key: &str) -> Result<i32, String> {
    let value = required_i64(json, key)?;
    i32::try_from(value).map_err(|_| format!("角色 AI 字段 {key} 超出 i32 范围"))
}

pub(super) fn required_i32_at(json: &Value, key: &str, path: &str) -> Result<i32, String> {
    let value = json
        .get(key)
        .ok_or_else(|| format!("角色 AI 结果缺少字段: {path}"))?;
    let number = value.as_i64().ok_or_else(|| {
        format!(
            "角色 AI 字段 {path} 必须是 JSON 整数数字（例如 2），不能是字符串、对象或带“第…章”的文本"
        )
    })?;
    i32::try_from(number).map_err(|_| format!("角色 AI 字段 {path} 超出 i32 范围"))
}

pub(super) fn required_usize(json: &Value, key: &str) -> Result<usize, String> {
    let value = json
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("角色 AI 字段 {key} 必须是非负整数"))?;
    usize::try_from(value).map_err(|_| format!("角色 AI 字段 {key} 超出 usize 范围"))
}

pub(super) fn required_string(json: &Value, key: &str) -> Result<String, String> {
    let text = json
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("角色 AI 字段 {key} 必须是字符串"))?
        .trim();
    if text.is_empty() {
        return Err(format!("角色 AI 字段 {key} 不能为空"));
    }
    Ok(text.to_string())
}

pub(super) fn required_category(json: &Value, key: &str) -> Result<String, String> {
    let category = required_string(json, key)?;
    if matches!(category.as_str(), "explicit" | "implicit" | "social") {
        Ok(category)
    } else {
        Err(format!(
            "角色 AI 字段 {key} 必须是 explicit/implicit/social"
        ))
    }
}

fn required_operation(json: &Value, key: &str) -> Result<String, String> {
    let operation = required_string(json, key)?;
    if matches!(operation.as_str(), "add" | "modify" | "delete") {
        Ok(operation)
    } else {
        Err(format!("角色 AI 字段 {key} 必须是 add/modify/delete"))
    }
}

pub(super) fn required_attribute_change_operation(
    json: &Value,
    key: &str,
) -> Result<String, String> {
    let operation = required_operation(json, key)?;
    if operation == "delete" {
        return Err(
            "attribute_changes.operation 不允许 delete；删除必须进入 deletion_candidates 并由删除保护阈值执行"
                .to_string(),
        );
    }
    Ok(operation)
}

pub(super) fn required_value<'a>(json: &'a Value, key: &str) -> Result<&'a Value, String> {
    json.get(key)
        .ok_or_else(|| format!("角色 AI 结果缺少字段: {key}"))
}

pub(super) fn required_array<'a>(json: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    json.get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("角色 AI 字段 {key} 必须是数组"))
}

pub(super) fn required_string_array(json: &Value, key: &str) -> Result<(), String> {
    let values = required_array(json, key)?;
    for (index, value) in values.iter().enumerate() {
        let Some(text) = value.as_str().map(str::trim) else {
            return Err(format!("角色 AI 字段 {key}[{index}] 必须是字符串"));
        };
        if text.is_empty() {
            return Err(format!("角色 AI 字段 {key}[{index}] 不能为空"));
        }
    }
    Ok(())
}

pub(super) fn optional_string_or_null(json: &Value, key: &str) -> Result<(), String> {
    let Some(value) = json.get(key) else {
        return Err(format!("角色 AI 结果缺少字段: {key}"));
    };
    if value.is_null() || value.is_string() {
        Ok(())
    } else {
        Err(format!("角色 AI 字段 {key} 必须是字符串或 null"))
    }
}

pub(super) fn optional_string(json: &Value, key: &str) -> Option<String> {
    json.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn category_storage_key(category: &str) -> &'static str {
    match category {
        "explicit" => "explicit_attributes",
        "implicit" => "implicit_attributes",
        "social" => "social_attributes",
        _ => "attributes",
    }
}

fn category_count(value: &Value, category: &str) -> Result<usize, String> {
    let storage_key = category_storage_key(category);
    match value.get(storage_key) {
        Some(Value::Object(object)) => Ok(object.len()),
        Some(_) => Err(format!("角色当前状态字段 {storage_key} 必须是对象")),
        None => Err(format!("角色当前状态缺少字段 {storage_key}")),
    }
}

pub(super) fn category_attributes(value: &Value, category: &str) -> Result<Value, String> {
    let storage_key = category_storage_key(category);
    value
        .get(storage_key)
        .and_then(Value::as_object)
        .map(|object| Value::Object(object.clone()))
        .ok_or_else(|| format!("角色当前状态缺少或损坏 {storage_key}"))
}

pub(super) fn attribute_counts(value: &Value) -> Result<Value, String> {
    Ok(json!({
        "explicit": category_count(value, "explicit")?,
        "implicit": category_count(value, "implicit")?,
        "social": category_count(value, "social")?,
    }))
}
