use serde_json::Value;

use crate::structured_llm::{
    parse_strict_json_response,
    schema_repair::{
        contract::StructuredRepairMode, execution::contracts::StructuredValueRepairInput,
    },
};

use super::super::contracts::StructuredOutputRepairResult;
use super::contracts::{InitialRepairOutcome, StructuredRepairState};

pub(super) fn parse_initial_structured_output<T, P>(
    input: &StructuredValueRepairInput<'_, P>,
) -> InitialRepairOutcome<T>
where
    P: Fn(&Value) -> Result<T, a4n_core::error::AppError>,
{
    let raw_value = serde_json::from_str::<Value>(input.raw_response);
    if let Ok(value) = raw_value.as_ref() {
        if let Ok(output) = (input.parse_output)(value) {
            return InitialRepairOutcome::Complete(StructuredOutputRepairResult {
                output,
                final_value: value.clone(),
                mode: StructuredRepairMode::None,
                restored: false,
                schema_sha256: None,
                field_patch_error: None,
            });
        }
    }

    let initial_error = match raw_value.as_ref() {
        Ok(value) => (input.parse_output)(value).err().unwrap_or_else(|| {
            a4n_core::error::AppError::InvalidArgument("结构化输出校验失败".into())
        }),
        Err(error) => match parse_strict_json_response::<Value>(input.raw_response, input.label) {
            Err(error) => error,
            Ok(_) => a4n_core::error::AppError::InvalidArgument(format!(
                "{}结构化输出 JSON 解析失败：{}",
                input.label, error
            )),
        },
    };
    InitialRepairOutcome::Repair(StructuredRepairState {
        candidate: raw_value.ok(),
        initial_error,
        field_patch_error: None,
        schema_sha256: None,
    })
}

#[cfg(test)]
mod tests {
    use a4n_core::error::AppError;
    use a4n_llm::provider::ChatRequest;
    use serde_json::json;

    use super::*;

    fn input<'a, P>(raw_response: &'a str, parse_output: &'a P) -> StructuredValueRepairInput<'a, P>
    where
        P: Fn(&Value) -> Result<String, AppError>,
    {
        StructuredValueRepairInput {
            request: ChatRequest::new(Vec::new(), None, None, None),
            raw_response,
            label: "测试输出",
            schema: json!({}),
            checkpoint: None,
            cancel_token: None,
            progress: None,
            parse_output,
        }
    }

    #[test]
    fn valid_initial_output_returns_without_repair() {
        let parse_output = |value: &Value| {
            value
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| AppError::InvalidArgument("name 缺失".into()))
        };
        let outcome = parse_initial_structured_output(&input(r#"{"name":"Ada"}"#, &parse_output));
        let InitialRepairOutcome::Complete(result) = outcome else {
            panic!("valid output must not enter repair");
        };
        assert_eq!(result.output, "Ada");
        assert_eq!(result.final_value, json!({"name": "Ada"}));
        assert_eq!(result.mode, StructuredRepairMode::None);
        assert!(!result.restored);
    }

    #[test]
    fn invalid_value_keeps_json_candidate_for_repair() {
        let parse_output = |_value: &Value| -> Result<String, AppError> {
            Err(AppError::InvalidArgument("name 缺失".into()))
        };
        let outcome = parse_initial_structured_output(&input(r#"{"other":1}"#, &parse_output));
        let InitialRepairOutcome::Repair(state) = outcome else {
            panic!("invalid value must enter repair");
        };
        assert_eq!(state.candidate, Some(json!({"other": 1})));
        assert_eq!(state.initial_error.to_string(), "参数无效：name 缺失");
        assert_eq!(state.field_patch_error, None);
        assert_eq!(state.schema_sha256, None);
    }

    #[test]
    fn malformed_json_enters_repair_without_candidate() {
        let parse_output = |_value: &Value| Ok::<_, AppError>("unused".to_string());
        let outcome = parse_initial_structured_output(&input("{", &parse_output));
        let InitialRepairOutcome::Repair(state) = outcome else {
            panic!("malformed JSON must enter repair");
        };
        assert_eq!(state.candidate, None);
        assert!(state.initial_error.to_string().contains("严格 JSON 对象"));
    }
}
