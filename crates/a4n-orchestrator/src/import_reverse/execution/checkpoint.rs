use a4n_api_contract::import_reverse::ReverseEngineeringTarget;
use a4n_core::error::AppError;
use serde_json::Value;

use crate::structured_llm::{ensure_checkpoint_parsed_output_matches, parse_strict_json_response};

pub(in crate::import_reverse) fn restored_import_reverse_checkpoint(
    parsed_output: &Value,
    response_content: &str,
    target: ReverseEngineeringTarget,
) -> Result<Value, AppError> {
    let prompt_name = target.prompt_name();
    let reparsed: Value = parse_strict_json_response(response_content, prompt_name)?;
    crate::import_reverse::application::validate_reverse_engineer_output(target, &reparsed)?;
    ensure_checkpoint_parsed_output_matches(
        parsed_output,
        &reparsed,
        &format!("导入反推 {prompt_name}"),
    )?;
    Ok(reparsed)
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
        let restored = restored_import_reverse_checkpoint(
            &parsed_output,
            r#"{"outlines":[{"chapter_number":1,"title":"开端","summary":"故事开端"}]}"#,
            ReverseEngineeringTarget::Outline,
        )
        .unwrap();

        assert_eq!(restored, parsed_output);
    }

    #[test]
    fn rejects_mismatched_import_reverse_checkpoint() {
        let parsed_output = json!({
            "outlines": [{
                "chapter_number": 1,
                "title": "旧标题",
                "summary": "故事开端"
            }]
        });
        let error = restored_import_reverse_checkpoint(
            &parsed_output,
            r#"{"outlines":[{"chapter_number":1,"title":"开端","summary":"故事开端"}]}"#,
            ReverseEngineeringTarget::Outline,
        )
        .err()
        .expect("mismatched checkpoint must fail");

        assert!(
            error
                .to_string()
                .contains("checkpoint_parsed_output_mismatch")
        );
    }
}
