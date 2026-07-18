use a4n_core::import_reverse::json::optional_string;
use serde_json::Value;

pub(crate) fn required_string(value: &Value, key: &str, context: &str) -> Result<String, String> {
    optional_string(value, key).ok_or_else(|| format!("{context} 缺少必填字符串字段 {key}"))
}

pub(crate) fn optional_i32(value: &Value, key: &str, context: &str) -> Result<Option<i32>, String> {
    let Some(value) = value.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let raw = value
        .as_i64()
        .ok_or_else(|| format!("{context}.{key} 必须是整数"))?;
    i32::try_from(raw)
        .map(Some)
        .map_err(|_| format!("{context}.{key} 超出 i32 可表示范围"))
}

pub(crate) fn required_i32(value: &Value, key: &str, context: &str) -> Result<i32, String> {
    optional_i32(value, key, context)?.ok_or_else(|| format!("{context} 缺少必填整数字段 {key}"))
}

fn optional_array<'a>(value: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    value.get(key).and_then(Value::as_array)
}

pub(crate) fn required_array<'a>(
    value: &'a Value,
    key: &str,
    context: &str,
) -> Result<&'a Vec<Value>, String> {
    optional_array(value, key).ok_or_else(|| format!("{context} 缺少必填数组字段 {key}"))
}

pub(crate) fn optional_field_clone(value: &Value, key: &str) -> Option<Value> {
    value.get(key).filter(|v| !v.is_null()).cloned()
}
