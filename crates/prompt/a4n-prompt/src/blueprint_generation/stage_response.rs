use a4n_error::AppError;
use serde_json::Value;

pub fn parse_blueprint_stage_json_object(
    content: &str,
    context: &str,
    output_schema: &str,
) -> Result<Value, AppError> {
    let value =
        a4n_domain::json_contract::parse_strict_json_value(content.trim()).map_err(|error| {
            AppError::InvalidArgument(format!("{context} 输出无法解析为严格 JSON 对象: {error}"))
        })?;
    let output_schema = parse_blueprint_stage_output_schema(output_schema, context)?;
    validate_stage_json_object(value, context, &output_schema)
}

fn validate_stage_json_object(
    value: Value,
    context: &str,
    output_schema: &Value,
) -> Result<Value, AppError> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context} 输出必须是 JSON 对象")))?;
    let expected = output_schema
        .as_object()
        .expect("parse_blueprint_stage_output_schema guarantees an object");
    for key in object.keys() {
        if !expected.contains_key(key) {
            return Err(AppError::InvalidArgument(format!(
                "{context} 输出包含未声明顶层字段 {key}"
            )));
        }
    }
    for key in expected.keys() {
        if !object.contains_key(key) {
            return Err(AppError::InvalidArgument(format!(
                "{context} 输出缺少必填顶层字段 {key}"
            )));
        }
    }
    Ok(value)
}

pub(in crate::blueprint_generation) fn parse_blueprint_stage_output_schema(
    output_schema: &str,
    context: &str,
) -> Result<Value, AppError> {
    let value: Value = serde_json::from_str(output_schema).map_err(|error| {
        AppError::InvalidArgument(format!("{context} 输出 schema 不是合法 JSON: {error}"))
    })?;
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument(format!("{context} 输出 schema 必须是 JSON 对象"))
    })?;
    if object.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{context} 输出 schema 不能为空 JSON 对象"
        )));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_object_response() {
        let error = parse_blueprint_stage_json_object("[]", "阶段测试", r#"{"result":{}}"#)
            .expect_err("non-object response must fail");

        assert!(error.to_string().contains("必须是 JSON 对象"));
    }

    #[test]
    fn rejects_unexpected_top_level_key() {
        let error = parse_blueprint_stage_json_object(
            r#"{"result":{},"extra":true}"#,
            "阶段测试",
            r#"{"result":{}}"#,
        )
        .expect_err("unexpected key must fail");

        assert!(error.to_string().contains("未声明顶层字段 extra"));
    }

    #[test]
    fn rejects_missing_top_level_key() {
        let error = parse_blueprint_stage_json_object(
            r#"{"result":{}}"#,
            "阶段测试",
            r#"{"result":{},"metadata":{}}"#,
        )
        .expect_err("missing key must fail");

        assert!(error.to_string().contains("缺少必填顶层字段 metadata"));
    }

    #[test]
    fn rejects_non_object_output_schema() {
        let error = parse_blueprint_stage_json_object(r#"{"result":{}}"#, "阶段测试", "[]")
            .expect_err("stage schema must be an object");

        assert!(error.to_string().contains("输出 schema 必须是 JSON 对象"));
    }
}
