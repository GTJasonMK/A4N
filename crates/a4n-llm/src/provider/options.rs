use a4n_core::error::AppError;
use serde_json::{Map, Value};

const RESERVED_REQUEST_OPTION_KEYS: &[&str] = &[
    "model",
    "messages",
    "stream",
    "temperature",
    "max_tokens",
    "response_format",
    "prompt_cache_key",
    "tools",
    "tool_choice",
    "stop",
];

pub fn normalize_request_options_json(value: Option<Value>) -> Result<Option<Value>, AppError> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Value::Object(map) = value else {
        return Err(AppError::ConfigError(
            "LLM request_options_json 必须是 JSON 对象".to_string(),
        ));
    };
    for key in map.keys() {
        if RESERVED_REQUEST_OPTION_KEYS
            .iter()
            .any(|reserved| key.eq_ignore_ascii_case(reserved))
        {
            return Err(AppError::ConfigError(format!(
                "LLM request_options_json 不能包含系统保留字段：{key}"
            )));
        }
    }
    Ok((!map.is_empty()).then_some(Value::Object(map)))
}

pub fn normalize_request_options(
    request_options: Option<Value>,
) -> Result<Option<Map<String, Value>>, AppError> {
    match normalize_request_options_json(request_options)? {
        Some(Value::Object(map)) => Ok(Some(map)),
        _ => Ok(None),
    }
}
