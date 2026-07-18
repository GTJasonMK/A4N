use a4n_core::error::AppError;
use serde_json::Value;

pub(super) fn any_env_present(keys: &[&str]) -> Result<bool, AppError> {
    for key in keys {
        if optional_env_string(key)?.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn required_env_string(key: &str) -> Result<String, AppError> {
    optional_env_string(key)?
        .ok_or_else(|| AppError::ConfigError(format!("公共默认配置缺少 {key}")))
}

pub(super) fn env_string_or(key: &str, fallback: &str) -> Result<String, AppError> {
    Ok(optional_env_string(key)?.unwrap_or_else(|| fallback.trim().to_string()))
}

pub(super) fn optional_env_string(key: &str) -> Result<Option<String>, AppError> {
    match std::env::var(key) {
        Ok(value) => {
            let value = value.trim().to_string();
            if value.is_empty() {
                Err(AppError::ConfigError(format!("{key} 不能为空")))
            } else {
                Ok(Some(value))
            }
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(err) => Err(AppError::ConfigError(format!("{key} 无法读取: {err}"))),
    }
}

pub(super) fn optional_env_json(key: &str) -> Result<Option<Value>, AppError> {
    let Some(value) = optional_env_string(key)? else {
        return Ok(None);
    };
    serde_json::from_str(&value)
        .map(Some)
        .map_err(|err| AppError::ConfigError(format!("{key} 必须是合法 JSON：{err}")))
}

pub(super) fn app_error_plain_message(err: AppError) -> String {
    match err {
        AppError::InvalidArgument(message)
        | AppError::ConfigError(message)
        | AppError::LlmError(message)
        | AppError::Timeout(message) => message,
        other => other.to_string(),
    }
}

pub(super) fn env_i32_or(key: &str, fallback: i32) -> Result<i32, AppError> {
    let Some(value) = optional_env_string(key)? else {
        return Ok(fallback);
    };
    value
        .parse::<i32>()
        .map_err(|err| AppError::ConfigError(format!("{key} 必须是整数：{err}")))
}

pub(super) fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn required_seed_value(value: String, key: &str) -> Result<String, AppError> {
    if value.trim().is_empty() {
        Err(AppError::ConfigError(format!("{key} 不能为空")))
    } else {
        Ok(value)
    }
}

pub(super) fn positive_i32(value: i32, key: &str) -> Result<i32, AppError> {
    if value <= 0 {
        Err(AppError::ConfigError(format!("{key} 必须大于 0")))
    } else {
        Ok(value)
    }
}

pub(super) fn usize_to_i32(value: usize, key: &str) -> Result<i32, AppError> {
    i32::try_from(value).map_err(|_| AppError::ConfigError(format!("{key} 超出 i32 范围")))
}

pub(super) fn u64_to_i32(value: u64, key: &str) -> Result<i32, AppError> {
    i32::try_from(value).map_err(|_| AppError::ConfigError(format!("{key} 超出 i32 范围")))
}

pub(super) fn u32_to_i32(value: u32, key: &str) -> Result<i32, AppError> {
    i32::try_from(value).map_err(|_| AppError::ConfigError(format!("{key} 超出 i32 范围")))
}
