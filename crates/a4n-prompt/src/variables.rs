use std::collections::HashMap;

use a4n_error::AppError;
use serde_json::Value;

const DEFAULT_INVALID_MESSAGE: &str = "variables 必须是对象或 null";

pub fn parse_template_variables(value: Option<&Value>) -> Result<HashMap<String, Value>, String> {
    match value {
        None | Some(Value::Null) => Ok(HashMap::new()),
        Some(Value::Object(map)) => Ok(map
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()),
        Some(_) => Err(DEFAULT_INVALID_MESSAGE.to_string()),
    }
}

pub fn parse_template_variables_app_error(
    value: &Option<Value>,
) -> Result<HashMap<String, Value>, AppError> {
    parse_template_variables(value.as_ref()).map_err(AppError::InvalidArgument)
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
    fn invalid_values_preserve_default_error_contract() {
        let value = json!(["tone"]);
        assert_eq!(
            parse_template_variables(Some(&value)).unwrap_err(),
            DEFAULT_INVALID_MESSAGE
        );
    }
}
