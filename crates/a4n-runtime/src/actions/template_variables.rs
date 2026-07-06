use a4n_core::error::AppError;
use serde_json::Value;
use std::collections::HashMap;

const DEFAULT_INVALID_MESSAGE: &str = "variables 必须是对象或 null";

pub(crate) fn parse_template_variables(
    value: Option<&Value>,
) -> Result<HashMap<String, Value>, String> {
    parse_template_variables_with_message(value, DEFAULT_INVALID_MESSAGE)
}

fn parse_template_variables_with_message(
    value: Option<&Value>,
    invalid_message: &str,
) -> Result<HashMap<String, Value>, String> {
    match value {
        None | Some(Value::Null) => Ok(HashMap::new()),
        Some(Value::Object(map)) => Ok(map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        Some(_) => Err(invalid_message.to_string()),
    }
}

pub(crate) fn parse_template_variables_app_error(
    value: &Option<Value>,
) -> Result<HashMap<String, Value>, AppError> {
    parse_template_variables(value.as_ref()).map_err(AppError::InvalidArgument)
}

pub(crate) fn parse_template_variables_app_error_from_ref_with_message(
    value: Option<&Value>,
    invalid_message: &str,
) -> Result<HashMap<String, Value>, AppError> {
    parse_template_variables_with_message(value, invalid_message).map_err(AppError::InvalidArgument)
}
