use a4n_core::error::AppError;
use serde_json::Value;

pub(in crate::import_reverse) fn restored_import_reverse_checkpoint(
    checkpoint_json: Value,
    prompt_name: &str,
) -> Result<Value, AppError> {
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_parsed_output".to_string(),
            operation: format!("读取导入反推 {prompt_name} 调用级断点"),
        })?;
    checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_response_content".to_string(),
            operation: format!("读取导入反推 {prompt_name} 调用级断点"),
        })?;
    crate::import_reverse::application::validate_reverse_engineer_output(
        prompt_name,
        &parsed_output,
    )?;
    Ok(parsed_output)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn restores_validated_import_reverse_checkpoint() {
        let parsed_output = json!({
            "outlines": [{
                "chapter_number": 1,
                "title": "开端",
                "summary": "故事开端"
            }]
        });
        let checkpoint = json!({
            "parsed_output": parsed_output.clone(),
            "response_content": "raw response"
        });

        let restored = restored_import_reverse_checkpoint(checkpoint, "reverse_outline").unwrap();

        assert_eq!(restored, parsed_output);
    }

    #[test]
    fn rejects_checkpoint_without_response_content() {
        let checkpoint = json!({"parsed_output": {"outlines": []}});

        let error = restored_import_reverse_checkpoint(checkpoint, "reverse_outline").unwrap_err();

        match error {
            AppError::InvalidState { current, operation } => {
                assert_eq!(current, "missing_checkpoint_response_content");
                assert_eq!(operation, "读取导入反推 reverse_outline 调用级断点");
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
