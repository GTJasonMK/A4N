use std::collections::HashMap;

use a4n_core::error::AppError;
use serde_json::Value;

const DEFAULT_INVALID_MESSAGE: &str = "variables 必须是对象或 null";

pub fn parse_template_variables(value: Option<&Value>) -> Result<HashMap<String, Value>, String> {
    parse_template_variables_with_message(value, DEFAULT_INVALID_MESSAGE)
}

fn parse_template_variables_with_message(
    value: Option<&Value>,
    invalid_message: &str,
) -> Result<HashMap<String, Value>, String> {
    match value {
        None | Some(Value::Null) => Ok(HashMap::new()),
        Some(Value::Object(map)) => Ok(map
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()),
        Some(_) => Err(invalid_message.to_string()),
    }
}

pub fn parse_template_variables_app_error(
    value: &Option<Value>,
) -> Result<HashMap<String, Value>, AppError> {
    parse_template_variables(value.as_ref()).map_err(AppError::InvalidArgument)
}

pub fn parse_template_variables_app_error_from_ref_with_message(
    value: Option<&Value>,
    invalid_message: &str,
) -> Result<HashMap<String, Value>, AppError> {
    parse_template_variables_with_message(value, invalid_message).map_err(AppError::InvalidArgument)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn missing_and_null_variables_produce_empty_maps() {
        assert!(parse_template_variables(None).unwrap().is_empty());
        assert!(
            parse_template_variables(Some(&Value::Null))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn object_variables_preserve_values() {
        let variables = parse_template_variables(Some(&json!({
            "tone": "warm",
            "limits": {"max": 3},
        })))
        .expect("object variables should parse");

        assert_eq!(variables["tone"], "warm");
        assert_eq!(variables["limits"]["max"], 3);
    }

    #[test]
    fn invalid_values_preserve_default_and_custom_error_contracts() {
        let value = json!(["tone"]);
        assert_eq!(
            parse_template_variables(Some(&value)).unwrap_err(),
            DEFAULT_INVALID_MESSAGE
        );
        let error = parse_template_variables_app_error_from_ref_with_message(
            Some(&value),
            "自定义变量错误",
        )
        .unwrap_err();
        assert!(matches!(
            error,
            AppError::InvalidArgument(message) if message == "自定义变量错误"
        ));
    }
}
