use std::ffi::OsString;
use std::path::PathBuf;

use crate::error::AppError;

/// 读取可选字符串环境变量。未设置返回 `None`，已设置为空则明确报错。
pub fn optional_non_empty_env_string(key: &str) -> Result<Option<String>, AppError> {
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
        Err(error) => Err(AppError::ConfigError(format!("{key} 无法读取: {error}"))),
    }
}

/// 读取可选路径环境变量。未设置返回 `None`，已设置为空则明确报错。
pub fn optional_non_empty_env_path(key: &str) -> Result<Option<PathBuf>, AppError> {
    parse_optional_non_empty_env_path(key, std::env::var_os(key))
}

fn parse_optional_non_empty_env_path(
    key: &str,
    value: Option<OsString>,
) -> Result<Option<PathBuf>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.to_string_lossy().trim().is_empty() {
        return Err(AppError::ConfigError(format!("{key} 不能为空")));
    }
    Ok(Some(PathBuf::from(value)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_path_distinguishes_unset_empty_and_configured_values() {
        assert_eq!(
            parse_optional_non_empty_env_path("A4N_PATH", None).unwrap(),
            None
        );

        let error = parse_optional_non_empty_env_path("A4N_PATH", Some(OsString::from("   ")))
            .expect_err("configured empty path must fail");
        assert!(error.to_string().contains("A4N_PATH 不能为空"));

        assert_eq!(
            parse_optional_non_empty_env_path("A4N_PATH", Some(OsString::from("data/cache")),)
                .unwrap(),
            Some(PathBuf::from("data/cache"))
        );
    }
}
