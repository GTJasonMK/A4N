use super::super::json::parse_blueprint_optimization_json;
use a4n_core::blueprint::ParsedBlueprintOptimization;
use a4n_core::error::AppError;
use a4n_prompt::blueprint_generation::ensure_blueprint_response_not_truncated;
use serde_json::Value;

pub(super) fn blueprint_refine_checkpoint_output(
    checkpoint_json: &Value,
    call_key: &str,
) -> Result<(ParsedBlueprintOptimization, Option<Value>, Option<String>), AppError> {
    checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("{call_key}.response_content 缺失或不是字符串"),
            operation: "恢复蓝图优化 LLM 调用断点".to_string(),
        })?;
    let parsed_output =
        checkpoint_json
            .get("parsed_output")
            .ok_or_else(|| AppError::InvalidState {
                current: format!("{call_key}.parsed_output 缺失"),
                operation: "恢复蓝图优化 LLM 调用断点".to_string(),
            })?;
    let parsed = parse_blueprint_optimization_json(&serde_json::to_string(parsed_output)?)?;
    let usage = checkpoint_json
        .get("usage")
        .filter(|value| !value.is_null())
        .cloned();
    let finish_reason =
        match checkpoint_json.get("finish_reason") {
            Some(Value::Null) | None => None,
            Some(value) => Some(value.as_str().map(ToString::to_string).ok_or_else(|| {
                AppError::InvalidState {
                    current: format!("{call_key}.finish_reason 必须是字符串或 null"),
                    operation: "恢复蓝图优化 LLM 调用断点".to_string(),
                }
            })?),
        };
    ensure_blueprint_response_not_truncated(finish_reason.as_deref(), "蓝图优化 checkpoint")?;
    Ok((parsed, usage, finish_reason))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn restores_matching_blueprint_refine_checkpoint() {
        let checkpoint = json!({
            "response_content": r#"{"style":"冷峻"}"#,
            "parsed_output": {"style": "冷峻"},
            "usage": {"prompt_tokens": 10},
            "finish_reason": "stop"
        });

        let (response, usage, finish_reason) =
            blueprint_refine_checkpoint_output(&checkpoint, "blueprint_refine:7").unwrap();

        assert_eq!(response.raw, json!({"style": "冷峻"}));
        assert_eq!(usage, Some(json!({"prompt_tokens": 10})));
        assert_eq!(finish_reason.as_deref(), Some("stop"));
    }

    #[test]
    fn restores_repaired_blueprint_refine_checkpoint_from_parsed_output() {
        let checkpoint = json!({
            "response_content": r#"{"style":"冷峻"}"#,
            "parsed_output": {"style": "温暖"},
            "finish_reason": null
        });

        let (response, _, _) =
            blueprint_refine_checkpoint_output(&checkpoint, "blueprint_refine:7")
                .expect("repaired checkpoint should restore parsed output");

        assert_eq!(response.raw, json!({"style": "温暖"}));
    }

    #[test]
    fn rejects_truncated_blueprint_refine_checkpoint() {
        let checkpoint = json!({
            "response_content": r#"{"style":"冷峻"}"#,
            "parsed_output": {"style": "冷峻"},
            "finish_reason": "length"
        });

        let error = blueprint_refine_checkpoint_output(&checkpoint, "blueprint_refine:7")
            .expect_err("truncated checkpoint must fail");

        assert!(
            error
                .to_string()
                .contains("蓝图优化 checkpoint 输出达到当前蓝图 max_tokens 上限")
        );
    }
}
