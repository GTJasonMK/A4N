use serde_json::{Map, Value};

pub(in crate::actions::project_transfer) fn required_json_string_field<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    label: &str,
) -> Result<&'a str, String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{label}.{field} 必须是字符串"))
}

pub(in crate::actions::project_transfer) fn required_optional_json_string_field<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    label: &str,
) -> Result<Option<&'a str>, String> {
    let value = object
        .get(field)
        .ok_or_else(|| format!("{label}.{field} 必须存在"))?;
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_str()
        .map(Some)
        .ok_or_else(|| format!("{label}.{field} 必须是字符串或 null"))
}
