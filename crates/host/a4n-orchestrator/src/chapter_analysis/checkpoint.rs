use a4n_error::AppError;
use serde_json::Value;

use crate::source_markers::SourceMarkerMap;
use crate::structured_llm::{
    ensure_checkpoint_parsed_output_matches, parse_source_marked_json_response,
};

pub(super) fn parse_chapter_analysis_stage_output<V>(
    response_content: &str,
    context: &str,
    markers: &SourceMarkerMap,
    validate: &V,
) -> Result<Value, AppError>
where
    V: Fn(&Value) -> Result<(), AppError>,
{
    let output = parse_source_marked_json_response::<Value>(response_content, context, markers)?;
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
    markers: &SourceMarkerMap,
    validate: &impl Fn(&Value) -> Result<(), AppError>,
) -> Result<Value, AppError> {
    let reparsed =
        parse_chapter_analysis_stage_output(response_content, context, markers, validate)?;
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
    use crate::source_markers::SourceMarkerMap;

    #[test]
    fn restores_matching_chapter_analysis_checkpoint() {
        let markers = SourceMarkerMap::from_ids(["ch1:s1:f1".to_string()]).unwrap();
        let output = restored_chapter_analysis_stage_output(
            &serde_json::json!({"facts": [{"source_span_ids": ["ch1:s1:f1"]}]}),
            r##"{"facts":[{"source_span_ids":["#0"]}]}"##,
            "事实抽取阶段",
            &markers,
            &|_| Ok(()),
        )
        .expect("matching checkpoint should restore");

        assert_eq!(
            output,
            serde_json::json!({"facts": [{"source_span_ids": ["ch1:s1:f1"]}]})
        );
    }

    #[test]
    fn rejects_mismatched_chapter_analysis_checkpoint_output() {
        let markers = SourceMarkerMap::from_ids(["ch1:s1:f1".to_string()]).unwrap();
        let error = restored_chapter_analysis_stage_output(
            &serde_json::json!({"facts": ["stale"]}),
            r#"{"facts":[]}"#,
            "事实抽取阶段",
            &markers,
            &|_| Ok(()),
        )
        .expect_err("mismatched checkpoint should fail");

        match error {
            a4n_error::AppError::InvalidState { current, operation } => {
                assert_eq!(current, "checkpoint_parsed_output_mismatch");
                assert_eq!(operation, "校验章节分析 事实抽取阶段 checkpoint");
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
