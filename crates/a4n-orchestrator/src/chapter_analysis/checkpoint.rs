use a4n_core::error::AppError;
use serde_json::Value;

pub(super) fn restored_chapter_analysis_stage_output(
    checkpoint_json: Value,
    context: &str,
) -> Result<Value, AppError> {
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_parsed_output".to_string(),
            operation: format!("读取章节分析 {context} 调用级断点"),
        })?;
    checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_response_content".to_string(),
            operation: format!("读取章节分析 {context} 调用级断点"),
        })?;
    if !parsed_output.is_object() {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_not_object".to_string(),
            operation: format!("校验章节分析 {context} 调用级断点"),
        });
    }
    Ok(parsed_output)
}

#[cfg(test)]
mod tests {
    use super::restored_chapter_analysis_stage_output;

    #[test]
    fn restores_matching_chapter_analysis_checkpoint() {
        let checkpoint = serde_json::json!({
            "response_content": r#"{"facts":[]}"#,
            "parsed_output": {"facts": []}
        });

        let output = restored_chapter_analysis_stage_output(checkpoint, "事实抽取阶段")
            .expect("matching checkpoint should restore");

        assert_eq!(output, serde_json::json!({"facts": []}));
    }

    #[test]
    fn rejects_non_object_chapter_analysis_checkpoint_output() {
        let checkpoint = serde_json::json!({
            "response_content": r#"{"facts":[]}"#,
            "parsed_output": []
        });

        let error = restored_chapter_analysis_stage_output(checkpoint, "事实抽取阶段")
            .expect_err("non-object checkpoint should fail");

        match error {
            a4n_core::error::AppError::InvalidState { current, operation } => {
                assert_eq!(current, "checkpoint_parsed_output_not_object");
                assert_eq!(operation, "校验章节分析 事实抽取阶段 调用级断点");
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
