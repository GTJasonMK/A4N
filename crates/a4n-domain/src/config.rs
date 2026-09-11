//! Pure configuration value normalization shared by adapters.

pub fn required_config_value(value: &str, kind: &str, field: &str) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(format!("{kind}缺少 {field}"));
    }
    Ok(value)
}

pub fn normalize_optional_config_value(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn positive_i32(value: i32, field: &str) -> Result<i32, String> {
    if value <= 0 {
        return Err(format!("{field} 必须大于 0"));
    }
    Ok(value)
}

pub fn normalize_config_name(value: impl AsRef<str>, kind: &str) -> Result<String, String> {
    let value = value.as_ref().trim().to_string();
    if value.is_empty() {
        return Err(format!("{kind}名称不能为空"));
    }
    Ok(value)
}
