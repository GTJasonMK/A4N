use a4n_core::error::AppError;
use serde_json::Value;

pub fn parse_blueprint_stage_json_object(
    content: &str,
    context: &str,
    expected_top_level_keys: &[&str],
) -> Result<Value, AppError> {
    let value: Value = serde_json::from_str(content.trim()).map_err(|error| {
        AppError::InvalidArgument(format!("{context} 输出无法解析为严格 JSON 对象: {error}"))
    })?;
    validate_stage_json_object(value, context, expected_top_level_keys)
}

fn validate_stage_json_object(
    value: Value,
    context: &str,
    expected_top_level_keys: &[&str],
) -> Result<Value, AppError> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context} 输出必须是 JSON 对象")))?;
    for key in object.keys() {
        if !expected_top_level_keys.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "{context} 输出包含未声明顶层字段 {key}"
            )));
        }
    }
    for key in expected_top_level_keys {
        if !object.contains_key(*key) {
            return Err(AppError::InvalidArgument(format!(
                "{context} 输出缺少必填顶层字段 {key}"
            )));
        }
    }
    if object.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{context} 输出不能为空 JSON 对象"
        )));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_object_response() {
        let error = parse_blueprint_stage_json_object("[]", "阶段测试", &["result"])
            .expect_err("non-object response must fail");

        assert!(error.to_string().contains("必须是 JSON 对象"));
    }

    #[test]
    fn rejects_unexpected_top_level_key() {
        let error = parse_blueprint_stage_json_object(
            r#"{"result":{},"extra":true}"#,
            "阶段测试",
            &["result"],
        )
        .expect_err("unexpected key must fail");

        assert!(error.to_string().contains("未声明顶层字段 extra"));
    }

    #[test]
    fn rejects_missing_top_level_key() {
        let error = parse_blueprint_stage_json_object(
            r#"{"result":{}}"#,
            "阶段测试",
            &["result", "metadata"],
        )
        .expect_err("missing key must fail");

        assert!(error.to_string().contains("缺少必填顶层字段 metadata"));
    }
}
