pub(super) fn project_list_offset(page: u64, limit: u64) -> Result<u64, String> {
    page.checked_sub(1)
        .and_then(|page_index| page_index.checked_mul(limit))
        .ok_or_else(|| "项目分页参数过大".to_string())
}

pub(super) fn required_text<'a>(value: &'a str, field: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{field} 不能为空"))
    } else {
        Ok(value)
    }
}

pub(super) fn required_optional_text<'a>(
    value: Option<&'a str>,
    field: &str,
) -> Result<&'a str, String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{field} 不能为空"))
}

pub(super) fn optional_text(value: Option<&str>) -> String {
    value.map(str::trim).unwrap_or_default().to_string()
}
