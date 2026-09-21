pub(super) fn validate_positive_i64(value: i64, field: &str) -> Result<(), String> {
    if value <= 0 {
        Err(format!("{field} 必须大于 0"))
    } else {
        Ok(())
    }
}

pub(super) fn required_text(value: String, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(format!("{field} 不能为空"))
    } else {
        Ok(trimmed.to_string())
    }
}
