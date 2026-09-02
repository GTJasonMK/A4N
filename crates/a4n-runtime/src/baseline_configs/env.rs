use a4n_core::{config::env::optional_non_empty_env_string, error::AppError};
use serde_json::Value;

pub(super) fn any_env_present(keys: &[&str]) -> Result<bool, AppError> {
    for key in keys {
        if optional_non_empty_env_string(key)?.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn env_string_or(key: &str, fallback: &str) -> Result<String, AppError> {
    Ok(optional_non_empty_env_string(key)?.unwrap_or_else(|| fallback.trim().to_string()))
}

pub(super) fn optional_env_json(key: &str) -> Result<Option<Value>, AppError> {
    let Some(value) = optional_non_empty_env_string(key)? else {
        return Ok(None);
    };
    serde_json::from_str(&value)
        .map(Some)
        .map_err(|err| AppError::ConfigError(format!("{key} 必须是合法 JSON：{err}")))
}

pub(super) fn env_i32_or(key: &str, fallback: i32) -> Result<i32, AppError> {
    let Some(value) = optional_non_empty_env_string(key)? else {
        return Ok(fallback);
    };
    value
        .parse::<i32>()
        .map_err(|err| AppError::ConfigError(format!("{key} 必须是整数：{err}")))
}

pub(super) fn usize_to_i32(value: usize, key: &str) -> Result<i32, AppError> {
    i32::try_from(value).map_err(|_| AppError::ConfigError(format!("{key} 超出 i32 范围")))
}

pub(super) fn u64_to_i32(value: u64, key: &str) -> Result<i32, AppError> {
    i32::try_from(value).map_err(|_| AppError::ConfigError(format!("{key} 超出 i32 范围")))
}
