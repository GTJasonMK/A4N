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
    let err = required_json_text(&blank, "description", "项目简介").expect_err("blank text fails");
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
