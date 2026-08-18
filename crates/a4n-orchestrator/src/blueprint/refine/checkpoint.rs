use super::super::json::parse_blueprint_optimization_json;
use a4n_core::blueprint::ParsedBlueprintOptimization;
use a4n_core::error::AppError;
use serde_json::Value;

use crate::structured_llm::ensure_checkpoint_parsed_output_matches;

pub(super) fn blueprint_refine_checkpoint_output(
    parsed_output: &Value,
    response_content: &str,
    usage: Option<Value>,
    finish_reason: Option<String>,
    call_key: &str,
) -> Result<(ParsedBlueprintOptimization, Option<Value>, String), AppError> {
    let parsed = parse_blueprint_optimization_json(response_content)?;
    ensure_checkpoint_parsed_output_matches(
        parsed_output,
        &parsed.raw,
        &format!("蓝图优化 {call_key}"),
    )?;
    let finish_reason = finish_reason.ok_or_else(|| AppError::InvalidState {
        current: format!("{call_key} 断点缺少 finish_reason"),
        operation: "恢复蓝图优化调用".to_string(),
    })?;
    Ok((parsed, usage, finish_reason))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn restores_matching_blueprint_refine_checkpoint() {
        let (response, usage, finish_reason) = blueprint_refine_checkpoint_output(
            &json!({"style": "冷峻"}),
            r#"{"style":"冷峻"}"#,
            Some(json!({"prompt_tokens": 10})),
            Some("stop".to_string()),
            "blueprint_refine:7",
        )
        .unwrap();

        assert_eq!(response.raw, json!({"style": "冷峻"}));
        assert_eq!(usage, Some(json!({"prompt_tokens": 10})));
        assert_eq!(finish_reason, "stop");
    }

    #[test]
    fn rejects_mismatched_blueprint_refine_checkpoint() {
        let error = blueprint_refine_checkpoint_output(
            &json!({"style": "温暖"}),
            r#"{"style":"冷峻"}"#,
            None,
            None,
            "blueprint_refine:7",
        )
        .err()
        .expect("mismatched checkpoint must fail");

        assert!(
            error
                .to_string()
                .contains("checkpoint_parsed_output_mismatch")
        );
    }

    #[test]
    fn rejects_checkpoint_without_finish_reason() {
        let error = blueprint_refine_checkpoint_output(
            &json!({"style": "冷峻"}),
            r#"{"style":"冷峻"}"#,
            None,
            None,
            "blueprint_refine:7",
        )
        .expect_err("completed checkpoint must retain finish_reason");

        assert!(error.to_string().contains("断点缺少 finish_reason"));
    }
}
