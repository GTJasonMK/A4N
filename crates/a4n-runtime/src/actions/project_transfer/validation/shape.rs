use serde_json::Value;

pub(in crate::actions::project_transfer) fn required_text<'a>(
    value: &'a str,
    label: &str,
) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label}不能为空"))
    } else {
        Ok(value)
    }
}

pub(in crate::actions::project_transfer) fn required_json_object<'a>(
    value: &'a Value,
    field: &str,
    label: &str,
) -> Result<&'a Value, String> {
    let field_value = value
        .get(field)
        .ok_or_else(|| format!("{label}缺少对象字段: {field}"))?;
    if field_value.is_object() {
        Ok(field_value)
    } else {
        Err(format!("{label}字段 {field} 必须是对象"))
    }
}

pub(in crate::actions::project_transfer) fn required_json_array<'a>(
    value: &'a Value,
    field: &str,
    label: &str,
) -> Result<&'a Vec<Value>, String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label}缺少数组字段: {field}"))
}

pub(in crate::actions::project_transfer) fn required_json_string<'a>(
    value: &'a Value,
    field: &str,
    label: &str,
) -> Result<&'a str, String> {
    value
        .get(field)
        .ok_or_else(|| format!("{label}缺少字段: {field}"))?
        .as_str()
        .ok_or_else(|| format!("{label}字段 {field} 必须是文本"))
}

pub(in crate::actions::project_transfer) fn required_json_text<'a>(
    value: &'a Value,
    field: &str,
    label: &str,
) -> Result<&'a str, String> {
    required_text(required_json_string(value, field, label)?, label)
}

pub(in crate::actions::project_transfer) fn optional_json_string<'a>(
    value: &'a Value,
    field: &str,
    label: &str,
) -> Result<Option<&'a str>, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(field_value) => field_value
            .as_str()
            .map(Some)
            .ok_or_else(|| format!("{label}字段 {field} 必须是文本或 null")),
    }
}
