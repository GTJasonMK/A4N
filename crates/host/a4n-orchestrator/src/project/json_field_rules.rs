//! 项目包校验共用的 JSON 对象字段读取规则。

use serde_json::{Map, Value};

pub fn required_json_i64_field(
    object: &Map<String, Value>,
    field: &str,
    label: &str,
) -> Result<i64, String> {
    object
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{label}.{field} 必须是整数"))
}

pub fn required_json_i32_field(
    object: &Map<String, Value>,
    field: &str,
    label: &str,
) -> Result<i32, String> {
    let value = required_json_i64_field(object, field, label)?;
    i32::try_from(value).map_err(|_| format!("{label}.{field} 超出 i32 范围: {value}"))
}

pub fn required_json_string_field<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    label: &str,
) -> Result<&'a str, String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{label}.{field} 必须是字符串"))
}

pub fn required_optional_json_string_field<'a>(
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
