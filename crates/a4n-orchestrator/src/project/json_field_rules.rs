//! 导入/包校验共用的 JSON 字段读取纯规则（无 DB、无 Host）。

use a4n_core::models::chapter::{chapter, chapter_outline};
use serde_json::{Map, Value};

use super::required_text;

/// 必填对象字段。
pub fn required_json_object<'a>(
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

/// 必填数组字段。
pub fn required_json_array<'a>(
    value: &'a Value,
    field: &str,
    label: &str,
) -> Result<&'a Vec<Value>, String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label}缺少数组字段: {field}"))
}

/// 必填字符串字段（不 trim）。
pub fn required_json_string<'a>(
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

/// 必填非空文本字段（经 `required_text`）。
pub fn required_json_text<'a>(
    value: &'a Value,
    field: &str,
    label: &str,
) -> Result<&'a str, String> {
    required_text(required_json_string(value, field, label)?, label)
}

/// 可选字符串字段（null/缺失 → None）。
pub fn optional_json_string<'a>(
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

/// 可选顶层对象字段。
pub fn optional_top_object<'a>(
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

/// 可选顶层数组字段（null/缺失 → 空切片）。
pub fn optional_top_array<'a>(
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

/// 可选 JSON 值（null/缺失 → None，否则 clone）。
pub fn optional_json_value(value: &Value, field: &str) -> Option<Value> {
    match value.get(field) {
        None | Some(Value::Null) => None,
        Some(field_value) => Some(field_value.clone()),
    }
}

/// 可选对象 clone。
pub fn optional_json_object_clone(
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

/// 必填布尔字段。
pub fn required_json_bool(value: &Value, field: &str, label: &str) -> Result<bool, String> {
    value
        .get(field)
        .ok_or_else(|| format!("{label}缺少字段: {field}"))?
        .as_bool()
        .ok_or_else(|| format!("{label}字段 {field} 必须是布尔值"))
}

fn required_json_i32(value: &Value, field: &str, label: &str) -> Result<i32, String> {
    let number = value
        .get(field)
        .ok_or_else(|| format!("{label}缺少字段: {field}"))?
        .as_i64()
        .ok_or_else(|| format!("{label}字段 {field} 必须是整数"))?;
    i32::try_from(number).map_err(|_| format!("{label}字段 {field} 超出 i32 范围"))
}

/// 必填正 i32 字段。
pub fn required_json_positive_i32_field(
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

/// 必填非负 i32 字段。
pub fn required_json_nonnegative_i32_field(
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

/// 可选正 i32 字段。
pub fn optional_json_positive_i32_field(
    value: &Value,
    field: &str,
    label: &str,
) -> Result<Option<i32>, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(_) => required_json_positive_i32_field(value, field, label).map(Some),
    }
}

/// 可选正 i64 字段。
pub fn optional_json_positive_i64_field(
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

/// 可选 f32 字段（有限且在 f32 范围）。
pub fn optional_json_f32_field(
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

/// 从 JSON 对象 Map 读取必填 i64。
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

/// 从 JSON 对象 Map 读取必填 i32。
pub fn required_json_i32_field(
    object: &Map<String, Value>,
    field: &str,
    label: &str,
) -> Result<i32, String> {
    let value = required_json_i64_field(object, field, label)?;
    i32::try_from(value).map_err(|_| format!("{label}.{field} 超出 i32 范围: {value}"))
}

/// 从 JSON 对象 Map 读取必填字符串。
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

/// 从 JSON 对象 Map 读取可选字符串（字段必须存在；null → None）。
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

/// 导入章节状态：缺省 imported；拒绝 generating 瞬态。
pub fn import_chapter_status(value: &Value, label: &str) -> Result<String, String> {
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

/// 导入章节大纲状态：缺省 draft。
pub fn import_chapter_outline_status(value: &Value, label: &str) -> Result<String, String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn shape_and_numbers() {
        let root = json!({"obj": {"a": 1}, "arr": [1], "name": " hi ", "n": 3, "flag": true});
        assert!(required_json_object(&root, "obj", "t").is_ok());
        assert_eq!(required_json_array(&root, "arr", "t").unwrap().len(), 1);
        assert_eq!(required_json_text(&root, "name", "名称").unwrap(), "hi");
        assert_eq!(
            required_json_positive_i32_field(&root, "n", "t").unwrap(),
            3
        );
        assert!(required_json_bool(&root, "flag", "t").unwrap());
        assert_eq!(
            optional_json_positive_i32_field(&json!({}), "n", "t").unwrap(),
            None
        );

        // 形状拒绝
        let err = required_json_object(&json!({"title": "x"}), "project", "导入文件").unwrap_err();
        assert!(err.contains("导入文件缺少对象字段: project"));
        let err = required_json_object(&json!({"project": []}), "project", "导入文件").unwrap_err();
        assert!(err.contains("导入文件字段 project 必须是对象"));
        let err =
            required_json_array(&json!({"chapters": {}}), "chapters", "导入文件").unwrap_err();
        assert!(err.contains("导入文件缺少数组字段: chapters"));

        // 文本/可选字符串
        let err = required_json_text(&json!({}), "description", "项目简介").unwrap_err();
        assert!(err.contains("项目简介缺少字段: description"));
        let err = required_json_text(&json!({"description": " \n\t "}), "description", "项目简介")
            .unwrap_err();
        assert!(err.contains("不能为空"));
        assert_eq!(
            required_json_string(&json!({"content": ""}), "content", "第1章").unwrap(),
            ""
        );
        let err = optional_json_string(
            &json!({"api_key_encrypted": 123}),
            "api_key_encrypted",
            "API Key",
        )
        .unwrap_err();
        assert!(err.contains("API Key字段 api_key_encrypted 必须是文本或 null"));

        // 数值边界
        let err = required_json_positive_i32_field(
            &json!({"chapter_number": 0}),
            "chapter_number",
            "导入章节",
        )
        .unwrap_err();
        assert!(err.contains("导入章节字段 chapter_number 必须为正整数"));
        let err =
            required_json_nonnegative_i32_field(&json!({"word_count": -1}), "word_count", "第1章")
                .unwrap_err();
        assert!(err.contains("第1章字段 word_count 不能为负数"));
    }

    #[test]
    fn import_status_defaults_and_rejects_generating() {
        assert_eq!(
            import_chapter_status(&json!({}), "章节").unwrap(),
            chapter::STATUS_IMPORTED
        );
        assert!(
            import_chapter_status(&json!({"status": chapter::STATUS_GENERATING}), "章节").is_err()
        );
        assert_eq!(
            import_chapter_outline_status(&json!({}), "大纲").unwrap(),
            chapter_outline::STATUS_DRAFT
        );
    }
}
