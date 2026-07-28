//! 角色 AI JSON 字段纯规则（无 DB、无 Host IO）。

use serde_json::{Value, json};

pub fn required_i64(json: &Value, key: &str) -> Result<i64, String> {
    json.get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("角色 AI 字段 {key} 必须是整数"))
}

pub fn required_i32(json: &Value, key: &str) -> Result<i32, String> {
    let value = required_i64(json, key)?;
    i32::try_from(value).map_err(|_| format!("角色 AI 字段 {key} 超出 i32 范围"))
}

pub fn required_i32_at(json: &Value, key: &str, path: &str) -> Result<i32, String> {
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

pub fn required_usize(json: &Value, key: &str) -> Result<usize, String> {
    let value = json
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("角色 AI 字段 {key} 必须是非负整数"))?;
    usize::try_from(value).map_err(|_| format!("角色 AI 字段 {key} 超出 usize 范围"))
}

pub fn required_string(json: &Value, key: &str) -> Result<String, String> {
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

pub fn required_category(json: &Value, key: &str) -> Result<String, String> {
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

pub fn required_attribute_change_operation(json: &Value, key: &str) -> Result<String, String> {
    let operation = required_operation(json, key)?;
    if operation == "delete" {
        return Err(
            "attribute_changes.operation 不允许 delete；删除必须进入 deletion_candidates 并由删除保护阈值执行"
                .to_string(),
        );
    }
    Ok(operation)
}

pub fn required_value<'a>(json: &'a Value, key: &str) -> Result<&'a Value, String> {
    json.get(key)
        .ok_or_else(|| format!("角色 AI 结果缺少字段: {key}"))
}

pub fn required_array<'a>(json: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    json.get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("角色 AI 字段 {key} 必须是数组"))
}

pub fn required_string_array(json: &Value, key: &str) -> Result<(), String> {
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

pub fn optional_string_or_null(json: &Value, key: &str) -> Result<(), String> {
    let Some(value) = json.get(key) else {
        return Err(format!("角色 AI 结果缺少字段: {key}"));
    };
    if value.is_null() || value.is_string() {
        Ok(())
    } else {
        Err(format!("角色 AI 字段 {key} 必须是字符串或 null"))
    }
}

pub fn optional_string(json: &Value, key: &str) -> Option<String> {
    json.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn category_storage_key(category: &str) -> &'static str {
    match category {
        "explicit" => "explicit_attributes",
        "implicit" => "implicit_attributes",
        "social" => "social_attributes",
        _ => "attributes",
    }
}

/// 统计角色状态某一分类属性对象的键数量。
pub fn category_count(value: &Value, category: &str) -> Result<usize, String> {
    let storage_key = category_storage_key(category);
    match value.get(storage_key) {
        Some(Value::Object(object)) => Ok(object.len()),
        Some(_) => Err(format!("角色当前状态字段 {storage_key} 必须是对象")),
        None => Err(format!("角色当前状态缺少字段 {storage_key}")),
    }
}

pub fn category_attributes(value: &Value, category: &str) -> Result<Value, String> {
    let storage_key = category_storage_key(category);
    value
        .get(storage_key)
        .and_then(Value::as_object)
        .map(|object| Value::Object(object.clone()))
        .ok_or_else(|| format!("角色当前状态缺少或损坏 {storage_key}"))
}

pub fn attribute_counts(value: &Value) -> Result<Value, String> {
    Ok(json!({
        "explicit": category_count(value, "explicit")?,
        "implicit": category_count(value, "implicit")?,
        "social": category_count(value, "social")?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn attribute_counts_requires_all_category_objects() {
        let counts = attribute_counts(&json!({
            "explicit_attributes": {"身份": "侦探"},
            "implicit_attributes": {},
            "social_attributes": {"关系": "同伴"},
        }))
        .expect("complete attribute categories should count");

        assert_eq!(counts["explicit"], json!(1));
        assert_eq!(counts["implicit"], json!(0));
        assert_eq!(counts["social"], json!(1));

        let missing = attribute_counts(&json!({
            "explicit_attributes": {},
            "implicit_attributes": {},
        }))
        .expect_err("missing social category should fail");
        assert!(missing.contains("角色当前状态缺少字段 social_attributes"));

        let corrupt = attribute_counts(&json!({
            "explicit_attributes": [],
            "implicit_attributes": {},
            "social_attributes": {},
        }))
        .expect_err("corrupt explicit category should fail");
        assert!(corrupt.contains("角色当前状态字段 explicit_attributes 必须是对象"));
    }
}
