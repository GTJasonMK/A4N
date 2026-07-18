use serde_json::{Map, Value};

fn required_json_i32(value: &Value, field: &str, label: &str) -> Result<i32, String> {
    let number = value
        .get(field)
        .ok_or_else(|| format!("{label}缺少字段: {field}"))?
        .as_i64()
        .ok_or_else(|| format!("{label}字段 {field} 必须是整数"))?;
    i32::try_from(number).map_err(|_| format!("{label}字段 {field} 超出 i32 范围"))
}

pub(in crate::actions::project_transfer) fn required_json_positive_i32_field(
    value: &Value,
    field: &str,
    label: &str,
) -> Result<i32, String> {
    let number = required_json_i32(value, field, label)?;
    if number > 0 {
        Ok(number)
    } else {
        Err(format!("{label}字段 {field} 必须为正整数"))
    }
}

pub(in crate::actions::project_transfer) fn required_json_nonnegative_i32_field(
    value: &Value,
    field: &str,
    label: &str,
) -> Result<i32, String> {
    let number = required_json_i32(value, field, label)?;
    if number >= 0 {
        Ok(number)
    } else {
        Err(format!("{label}字段 {field} 不能为负数"))
    }
}

pub(in crate::actions::project_transfer) fn optional_json_positive_i32_field(
    value: &Value,
    field: &str,
    label: &str,
) -> Result<Option<i32>, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(_) => required_json_positive_i32_field(value, field, label).map(Some),
    }
}

pub(in crate::actions::project_transfer) fn required_json_i64_field(
    object: &Map<String, Value>,
    field: &str,
    label: &str,
) -> Result<i64, String> {
    object
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{label}.{field} 必须是整数"))
}

pub(in crate::actions::project_transfer) fn required_json_i32_field(
    object: &Map<String, Value>,
    field: &str,
    label: &str,
) -> Result<i32, String> {
    let value = required_json_i64_field(object, field, label)?;
    i32::try_from(value).map_err(|_| format!("{label}.{field} 超出 i32 范围: {value}"))
}
