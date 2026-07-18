use super::file_io::validate_postgres_database_url;
use a4n_core::config::app_config::{AppConfig, resolve_path_in_portable_home};
use a4n_core::error::AppError;
use std::env::VarError;
use std::path::{Path, PathBuf};

pub(super) fn apply_env_overrides(
    config: &mut AppConfig,
    portable_home: &Path,
) -> Result<(), AppError> {
    let mut data_dir_overridden = false;
    let data_dir = env_path("A4N_DATA_DIR", portable_home)?;
    let cache_dir = env_path("A4N_CACHE_DIR", portable_home)?;
    let temp_dir = env_path("A4N_TEMP_DIR", portable_home)?;
    let log_dir = env_path("A4N_LOG_DIR", portable_home)?;
    let database_url = env_string("A4N_DATABASE_URL")?;
    let jwt_secret = env_string("A4N_JWT_SECRET")?;
    let openai_api_key = env_string("A4N_OPENAI_API_KEY")?;
    let openai_base_url = env_string("A4N_OPENAI_BASE_URL")?;

    if let Some(data_dir) = data_dir {
        config.data_dir = data_dir;
        data_dir_overridden = true;
    }
    if let Some(cache_dir) = cache_dir.clone() {
        config.cache_dir = cache_dir;
    }
    if let Some(temp_dir) = temp_dir.clone() {
        config.temp_dir = temp_dir;
    }
    if let Some(log_dir) = log_dir.clone() {
        config.log_dir = log_dir;
    }
    if let Some(db_url) = database_url {
        validate_postgres_database_url(&db_url)?;
        config.database_url = db_url;
    }
    if let Some(jwt_secret) = jwt_secret {
        config.security.jwt_secret = Some(jwt_secret);
    }
    if data_dir_overridden && cache_dir.is_none() {
        config.cache_dir = config.data_dir.join("cache");
    }
    if data_dir_overridden && temp_dir.is_none() {
        config.temp_dir = config.data_dir.join("tmp");
    }
    if data_dir_overridden && log_dir.is_none() {
        config.log_dir = config.data_dir.join("logs");
    }
    if let Some(api_key) = openai_api_key {
        config.llm.api_key = Some(api_key);
        config.embedding.api_key = config.llm.api_key.clone();
    }
    if let Some(base_url) = openai_base_url {
        config.llm.base_url = base_url;
    }
    Ok(())
}

pub(super) fn env_string(key: &str) -> Result<Option<String>, AppError> {
    match std::env::var(key) {
        Ok(value) => Ok(Some(validate_env_value(key, value)?)),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(AppError::ConfigError(format!(
            "{key} 必须是有效 Unicode 字符串"
        ))),
    }
}

pub(super) fn validate_env_value(key: &str, value: String) -> Result<String, AppError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(AppError::ConfigError(format!("{key} 不能为空")));
    }
    Ok(value)
}

pub(super) fn env_path(key: &str, portable_home: &Path) -> Result<Option<PathBuf>, AppError> {
    env_string(key).map(|value| {
        value.map(|path| resolve_path_in_portable_home(PathBuf::from(path), portable_home))
    })
}
