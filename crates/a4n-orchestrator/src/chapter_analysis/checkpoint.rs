use a4n_core::error::AppError;
use serde_json::Value;

use crate::structured_llm::{ensure_checkpoint_parsed_output_matches, parse_strict_json_response};

pub(super) fn parse_chapter_analysis_stage_output<V>(
    response_content: &str,
    context: &str,
    validate: &V,
) -> Result<Value, AppError>
where
    V: Fn(&Value) -> Result<(), AppError>,
{
    let output: Value = parse_strict_json_response(response_content, context)?;
    if !output.is_object() {
        return Err(AppError::InvalidArgument(format!(
            "{context} 必须返回 JSON 对象"
        )));
    }
    validate(&output)?;
    Ok(output)
}

pub(super) fn restored_chapter_analysis_stage_output(
    parsed_output: &Value,
    response_content: &str,
    context: &str,
    validate: &impl Fn(&Value) -> Result<(), AppError>,
) -> Result<Value, AppError> {
    let reparsed = parse_chapter_analysis_stage_output(response_content, context, validate)?;
    ensure_checkpoint_parsed_output_matches(
        parsed_output,
        &reparsed,
        &format!("章节分析 {context}"),
    )?;
    Ok(reparsed)
}

#[cfg(test)]
mod tests {
    use super::restored_chapter_analysis_stage_output;

    #[test]
    fn restores_matching_chapter_analysis_checkpoint() {
        let output = restored_chapter_analysis_stage_output(
            &serde_json::json!({"facts": []}),
            r#"{"facts":[]}"#,
            "事实抽取阶段",
            &|_| Ok(()),
        )
        .expect("matching checkpoint should restore");

        assert_eq!(output, serde_json::json!({"facts": []}));
    }

    #[test]
    fn rejects_mismatched_chapter_analysis_checkpoint_output() {
        let error = restored_chapter_analysis_stage_output(
            &serde_json::json!({"facts": ["stale"]}),
            r#"{"facts":[]}"#,
            "事实抽取阶段",
            &|_| Ok(()),
        )
        .expect_err("mismatched checkpoint should fail");

        match error {
            a4n_core::error::AppError::InvalidState { current, operation } => {
                assert_eq!(current, "checkpoint_parsed_output_mismatch");
                assert_eq!(operation, "校验章节分析 事实抽取阶段 checkpoint");
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
