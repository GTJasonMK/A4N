use a4n_domain::LLM_RESERVED_REQUEST_OPTION_KEYS;
use a4n_error::AppError;
use serde_json::{Map, Value};

pub fn normalize_extra_body_json(value: Option<Value>) -> Result<Option<Value>, AppError> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Value::Object(map) = value else {
        return Err(AppError::ConfigError(
            "LLM extra_body_json 必须是 JSON 对象".to_string(),
        ));
    };
    for key in map.keys() {
        if key.eq_ignore_ascii_case("n") && (key != "n" || map[key].as_u64() != Some(1)) {
            return Err(AppError::ConfigError(
                "LLM 调用只支持一个候选，extra_body_json.n 必须为 1".to_string(),
            ));
        }
        if LLM_RESERVED_REQUEST_OPTION_KEYS
            .iter()
            .any(|reserved| key.eq_ignore_ascii_case(reserved))
        {
            return Err(AppError::ConfigError(format!(
                "LLM extra_body_json 不能包含系统保留字段：{key}"
            )));
        }
    }
    Ok((!map.is_empty()).then_some(Value::Object(map)))
}

pub fn normalize_extra_body(
    extra_body: Option<Value>,
) -> Result<Option<Map<String, Value>>, AppError> {
    match normalize_extra_body_json(extra_body)? {
        Some(Value::Object(map)) => Ok(Some(map)),
        _ => Ok(None),
    }
}
