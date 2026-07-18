use super::*;

pub(super) fn optional_top_object<'a>(
    value: &'a Value,
    field: &str,
    label: &str,
) -> Result<Option<&'a Value>, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(field_value) if field_value.is_object() => Ok(Some(field_value)),
        Some(_) => Err(format!("{label}字段 {field} 必须是对象或 null")),
    }
}

pub(super) fn optional_top_array<'a>(
    value: &'a Value,
    field: &str,
    label: &str,
) -> Result<&'a [Value], String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(&[]),
        Some(field_value) => field_value
            .as_array()
            .map(Vec::as_slice)
            .ok_or_else(|| format!("{label}字段 {field} 必须是数组或 null")),
    }
}

pub(super) fn optional_json_value(value: &Value, field: &str) -> Option<Value> {
    match value.get(field) {
        None | Some(Value::Null) => None,
        Some(field_value) => Some(field_value.clone()),
    }
}

pub(super) fn optional_json_object_clone(
    value: &Value,
    field: &str,
    label: &str,
) -> Result<Option<Value>, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(field_value) if field_value.is_object() => Ok(Some(field_value.clone())),
        Some(_) => Err(format!("{label}字段 {field} 必须是对象或 null")),
    }
}

pub(super) fn required_json_bool(value: &Value, field: &str, label: &str) -> Result<bool, String> {
    value
        .get(field)
        .ok_or_else(|| format!("{label}缺少字段: {field}"))?
        .as_bool()
        .ok_or_else(|| format!("{label}字段 {field} 必须是布尔值"))
}

pub(super) fn optional_json_positive_i64_field(
    value: &Value,
    field: &str,
    label: &str,
) -> Result<Option<i64>, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(field_value) => {
            let number = field_value
                .as_i64()
                .ok_or_else(|| format!("{label}字段 {field} 必须是整数"))?;
            if number > 0 {
                Ok(Some(number))
            } else {
                Err(format!("{label}字段 {field} 必须为正整数"))
            }
        }
    }
}

pub(super) fn optional_json_f32_field(
    value: &Value,
    field: &str,
    label: &str,
) -> Result<Option<f32>, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(field_value) => {
            let number = field_value
                .as_f64()
                .ok_or_else(|| format!("{label}字段 {field} 必须是数字"))?;
            if !number.is_finite() || number < f32::MIN as f64 || number > f32::MAX as f64 {
                return Err(format!("{label}字段 {field} 超出 f32 范围"));
            }
            Ok(Some(number as f32))
        }
    }
}

pub(super) fn import_chapter_status(value: &Value, label: &str) -> Result<String, String> {
    let status = optional_json_string(value, "status", label)?
        .map(|status| required_text(status, &format!("{label}状态")))
        .transpose()?
        .unwrap_or(chapter::STATUS_IMPORTED)
        .to_string();
    if !chapter::is_valid_status(&status) {
        return Err(format!("{label}状态无效: {status}"));
    }
    if status == chapter::STATUS_GENERATING {
        return Err(format!(
            "{label}状态不能是生成中的瞬态状态: {}",
            chapter::STATUS_GENERATING
        ));
    }
    Ok(status)
}

pub(super) fn import_chapter_outline_status(value: &Value, label: &str) -> Result<String, String> {
    let status = optional_json_string(value, "status", label)?
        .map(|status| required_text(status, &format!("{label}状态")))
        .transpose()?
        .unwrap_or(chapter_outline::STATUS_DRAFT)
        .to_string();
    if !chapter_outline::is_valid_status(&status) {
        return Err(format!("{label}状态无效: {status}"));
    }
    Ok(status)
}
