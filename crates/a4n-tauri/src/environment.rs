use a4n_core::config::app_config::{AppConfig, resolve_portable_path};
use std::{env::VarError, path::PathBuf};

pub(crate) fn early_log_dir() -> Result<PathBuf, String> {
    match std::env::var("A4N_LOG_DIR") {
        Ok(value) => Ok(resolve_portable_path(PathBuf::from(validate_env_value(
            "A4N_LOG_DIR",
            value,
        )?))),
        Err(VarError::NotPresent) => Ok(AppConfig::default().log_dir),
        Err(VarError::NotUnicode(_)) => Err("A4N_LOG_DIR 必须是有效 Unicode 字符串".into()),
    }
}

pub(crate) fn validate_env_value(key: &str, value: String) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(format!("{key} 不能为空"));
    }
    Ok(value)
}

pub(crate) fn remote_api_only() -> Result<bool, String> {
    if env_flag_enabled("A4N_REMOTE_API_ONLY")? {
        return Ok(true);
    }
    match std::env::var("VITE_A4N_API_TRANSPORT") {
        Ok(value) => parse_api_transport(&value),
        Err(VarError::NotPresent) => Ok(false),
        Err(error) => Err(format!("读取 VITE_A4N_API_TRANSPORT 失败：{error}")),
    }
}

fn env_flag_enabled(key: &str) -> Result<bool, String> {
    match std::env::var(key) {
        Ok(value) => parse_env_flag(key, &value),
        Err(VarError::NotPresent) => Ok(false),
        Err(error) => Err(format!("读取 {key} 失败：{error}")),
    }
}

fn parse_api_transport(value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "web" | "http" | "remote" => Ok(true),
        "tauri" | "desktop" | "local" => Ok(false),
        "" => Err("VITE_A4N_API_TRANSPORT 不能为空".into()),
        other => Err(format!(
            "VITE_A4N_API_TRANSPORT 无效：{other}；允许值为 web、http、remote、tauri、desktop 或 local"
        )),
    }
}

fn parse_env_flag(key: &str, value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        "" => Err(format!("{key} 不能为空")),
        other => Err(format!("{key} 无效：{other}；允许值为 true 或 false")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_transport_rejects_blank_and_unknown_values() {
        assert!(parse_api_transport(" ").is_err());
        assert!(parse_api_transport("auto").is_err());
        assert!(parse_api_transport("web").unwrap());
        assert!(!parse_api_transport("tauri").unwrap());
    }

    #[test]
    fn environment_flag_rejects_blank_and_unknown_values() {
        assert!(parse_env_flag("A4N_FLAG", " ").is_err());
        assert!(parse_env_flag("A4N_FLAG", "enabled").is_err());
        assert!(parse_env_flag("A4N_FLAG", "yes").unwrap());
        assert!(!parse_env_flag("A4N_FLAG", "off").unwrap());
    }
}
