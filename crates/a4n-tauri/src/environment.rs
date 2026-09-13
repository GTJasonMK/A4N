use a4n_config::app_config::{AppConfig, resolve_portable_path};
use a4n_config::{env_flag_enabled, optional_non_empty_env_path};
use std::{env::VarError, path::PathBuf};

pub(crate) fn early_log_dir() -> Result<PathBuf, String> {
    optional_non_empty_env_path("A4N_LOG_DIR")
        .map_err(|error| error.to_string())
        .map(|path| {
            path.map(resolve_portable_path)
                .unwrap_or_else(|| AppConfig::default().log_dir)
        })
}

pub(crate) enum ApiMode {
    Native,
    Http { remote_only: bool },
}

pub(crate) fn api_mode() -> Result<ApiMode, String> {
    if env_flag_enabled("A4N_TAURI_LOCAL_SERVER").map_err(|error| error.to_string())? {
        return Ok(ApiMode::Http { remote_only: false });
    }
    if env_flag_enabled("A4N_REMOTE_API_ONLY").map_err(|error| error.to_string())? {
        return Ok(ApiMode::Http { remote_only: true });
    }
    match std::env::var("VITE_A4N_API_TRANSPORT") {
        Ok(value) => Ok(if parse_api_transport(&value)? {
            ApiMode::Http { remote_only: true }
        } else {
            ApiMode::Native
        }),
        Err(VarError::NotPresent) => Ok(ApiMode::Native),
        Err(error) => Err(format!("读取 VITE_A4N_API_TRANSPORT 失败：{error}")),
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
}
