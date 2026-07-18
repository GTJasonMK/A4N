use super::*;

pub(super) fn reject_legacy_anchor_field(
    value: &Value,
    old_key: &str,
    old_path: &str,
    new_key: &str,
) -> Result<(), String> {
    if value.get(old_key).is_some() {
        return Err(format!(
            "角色 AI 字段 {old_path} 已废弃；必须输出 {new_key}"
        ));
    }
    Ok(())
}

pub(super) fn required_positive_i64(value: &Value, key: &str, path: &str) -> Result<i64, String> {
    let number = value
        .get(key)
        .ok_or_else(|| format!("角色 AI 字段 {path} 缺少字段"))?
        .as_i64()
        .ok_or_else(|| format!("角色 AI 字段 {path} 必须是 JSON 整数数字"))?;
    if number <= 0 {
        return Err(format!("角色 AI 字段 {path} 必须大于 0"));
    }
    Ok(number)
}

pub(super) fn required_limited_string(
    value: &Value,
    key: &str,
    path: &str,
    max_chars: usize,
) -> Result<String, String> {
    let text = required_string(value, key)?;
    validate_char_limit(&text, path, max_chars)?;
    Ok(text)
}

pub(super) fn optional_limited_string_or_null(
    value: &Value,
    key: &str,
    path: &str,
    max_chars: usize,
) -> Result<(), String> {
    optional_string_or_null(value, key)?;
    if let Some(text) = value.get(key).and_then(Value::as_str) {
        validate_char_limit(text.trim(), path, max_chars)?;
    }
    Ok(())
}

pub(super) fn required_limited_string_array(
    value: &Value,
    key: &str,
    path: &str,
    max_items: usize,
    max_item_chars: usize,
) -> Result<(), String> {
    required_string_array(value, key)?;
    let values = required_array(value, key)?;
    if values.len() > max_items {
        return Err(format!(
            "角色 AI 字段 {path} 最多 {max_items} 项，实际 {} 项",
            values.len()
        ));
    }
    for (index, item) in values.iter().enumerate() {
        let text = item
            .as_str()
            .expect("required_string_array already validated strings")
            .trim();
        validate_char_limit(text, &format!("{path}[{index}]"), max_item_chars)?;
    }
    Ok(())
}

pub(super) fn validate_char_limit(text: &str, path: &str, max_chars: usize) -> Result<(), String> {
    let count = text.chars().count();
    if count > max_chars {
        return Err(format!(
            "角色 AI 字段 {path} 过长：{count} 字，最多 {max_chars} 字；只引用关键原文或简短摘要"
        ));
    }
    Ok(())
}
