use serde_json::{Map, Value};

pub(super) fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label}不能为空"))
    } else {
        Ok(value)
    }
}

pub(super) fn required_json_object<'a>(
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

pub(super) fn required_json_array<'a>(
    value: &'a Value,
    field: &str,
    label: &str,
) -> Result<&'a Vec<Value>, String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label}缺少数组字段: {field}"))
}

pub(super) fn required_json_string<'a>(
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

pub(super) fn required_json_text<'a>(
    value: &'a Value,
    field: &str,
    label: &str,
) -> Result<&'a str, String> {
    required_text(required_json_string(value, field, label)?, label)
}

pub(super) fn optional_json_string<'a>(
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

fn required_json_i32(value: &Value, field: &str, label: &str) -> Result<i32, String> {
    let number = value
        .get(field)
        .ok_or_else(|| format!("{label}缺少字段: {field}"))?
        .as_i64()
        .ok_or_else(|| format!("{label}字段 {field} 必须是整数"))?;
    i32::try_from(number).map_err(|_| format!("{label}字段 {field} 超出 i32 范围"))
}

pub(super) fn required_json_positive_i32_field(
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

pub(super) fn required_json_nonnegative_i32_field(
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

pub(super) fn optional_json_positive_i32_field(
    value: &Value,
    field: &str,
    label: &str,
) -> Result<Option<i32>, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(_) => required_json_positive_i32_field(value, field, label).map(Some),
    }
}

pub(super) fn required_json_i64_field(
    object: &Map<String, Value>,
    field: &str,
    label: &str,
) -> Result<i64, String> {
    object
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{label}.{field} 必须是整数"))
}

pub(super) fn required_json_i32_field(
    object: &Map<String, Value>,
    field: &str,
    label: &str,
) -> Result<i32, String> {
    let value = required_json_i64_field(object, field, label)?;
    if value < i32::MIN as i64 || value > i32::MAX as i64 {
        return Err(format!("{label}.{field} 超出 i32 范围: {value}"));
    }
    Ok(value as i32)
}

pub(super) fn required_json_string_field<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    label: &str,
) -> Result<&'a str, String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{label}.{field} 必须是字符串"))
}

pub(super) fn required_optional_json_string_field<'a>(
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn required_json_shape_helpers_reject_missing_or_wrong_type_fields() {
        let missing_object = json!({"title": "旧格式根对象"});
        let err = required_json_object(&missing_object, "project", "导入文件")
            .expect_err("nested project object should be required");
        assert!(err.contains("导入文件缺少对象字段: project"));

        let wrong_object = json!({"project": []});
        let err = required_json_object(&wrong_object, "project", "导入文件")
            .expect_err("project object should be object");
        assert!(err.contains("导入文件字段 project 必须是对象"));

        let wrong_array = json!({"chapters": {}});
        let err = required_json_array(&wrong_array, "chapters", "导入文件")
            .expect_err("chapters should be an array");
        assert!(err.contains("导入文件缺少数组字段: chapters"));
    }

    #[test]
    fn required_json_text_rejects_missing_or_blank_fields() {
        let missing = json!({});
        let err = required_json_text(&missing, "description", "项目简介")
            .expect_err("missing text should be rejected");
        assert!(err.contains("项目简介缺少字段: description"));

        let blank = json!({"description": " \n\t "});
        let err =
            required_json_text(&blank, "description", "项目简介").expect_err("blank text fails");
        assert!(err.contains("项目简介不能为空"));
    }

    #[test]
    fn required_json_string_accepts_empty_string_for_content_fields() {
        let value = json!({"content": ""});

        assert_eq!(required_json_string(&value, "content", "第1章"), Ok(""));
    }

    #[test]
    fn optional_json_string_rejects_non_string_values() {
        let value = json!({"api_key_encrypted": 123});

        let err = optional_json_string(&value, "api_key_encrypted", "API Key")
            .expect_err("number should be rejected");

        assert!(err.contains("API Key字段 api_key_encrypted 必须是文本或 null"));
    }

    #[test]
    fn json_positive_number_helpers_validate_bounds() {
        let zero = json!({"chapter_number": 0});
        let err = required_json_positive_i32_field(&zero, "chapter_number", "导入章节")
            .expect_err("zero should be rejected");
        assert!(err.contains("导入章节字段 chapter_number 必须为正整数"));

        let negative = json!({"word_count": -1});
        let err = required_json_nonnegative_i32_field(&negative, "word_count", "第1章")
            .expect_err("negative should be rejected");
        assert!(err.contains("第1章字段 word_count 不能为负数"));

        assert_eq!(
            optional_json_positive_i32_field(&json!({}), "total", "总数"),
            Ok(None)
        );
    }
}
