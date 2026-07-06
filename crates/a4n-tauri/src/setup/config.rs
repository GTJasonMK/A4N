use a4n_core::config::app_config::{AppConfig, portable_home_dir, resolve_path_in_portable_home};
use a4n_core::error::AppError;
use std::env::VarError;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

pub(super) fn load_config(_app: &AppHandle) -> Result<AppConfig, AppError> {
    let portable_home = portable_home_dir();
    let default_config = AppConfig::default();
    let config_path = config_file_path(&default_config, &portable_home)?;
    let mut config = load_config_from_file(default_config, &config_path, &portable_home)?;

    apply_env_overrides(&mut config, &portable_home)?;
    validate_postgres_database_url(&config.database_url)?;
    Ok(config)
}

fn load_config_from_file(
    default_config: AppConfig,
    config_path: &Path,
    portable_home: &Path,
) -> Result<AppConfig, AppError> {
    if config_path.exists() {
        let content = std::fs::read_to_string(config_path).map_err(AppError::Io)?;
        let mut config: AppConfig = toml::from_str(&content)
            .map_err(|e| AppError::ConfigError(format!("配置文件解析失败：{e}")))?;
        normalize_storage_paths(&mut config, portable_home)?;

        Ok(config)
    } else {
        tracing::info!("配置文件不存在，使用默认配置：{config_path:?}");
        write_config_file(config_path, &default_config)?;
        Ok(default_config)
    }
}

fn config_file_path(default_config: &AppConfig, portable_home: &Path) -> Result<PathBuf, AppError> {
    let data_dir =
        env_path("A4N_DATA_DIR", portable_home)?.unwrap_or_else(|| default_config.data_dir.clone());
    Ok(data_dir.join("config.toml"))
}

fn normalize_storage_paths(config: &mut AppConfig, portable_home: &Path) -> Result<(), AppError> {
    config.data_dir = resolve_path_in_portable_home(&config.data_dir, portable_home);
    config.cache_dir = resolve_path_in_portable_home(&config.cache_dir, portable_home);
    config.temp_dir = resolve_path_in_portable_home(&config.temp_dir, portable_home);
    config.log_dir = resolve_path_in_portable_home(&config.log_dir, portable_home);
    if let Some(path) = config.embedding.local_model_path.take() {
        config.embedding.local_model_path =
            Some(resolve_path_in_portable_home(path, portable_home));
    }
    Ok(())
}

fn validate_postgres_database_url(url: &str) -> Result<(), AppError> {
    let url = url.trim();
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        return Ok(());
    }
    Err(AppError::ConfigError(
        "A4N_DATABASE_URL 必须是 postgres:// 或 postgresql:// 连接串".to_string(),
    ))
}

fn write_config_file(path: &Path, config: &AppConfig) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(AppError::Io)?;
    }
    let content = toml::to_string_pretty(config)
        .map_err(|e| AppError::ConfigError(format!("配置文件序列化失败：{e}")))?;
    let temp_path = path.with_extension("toml.tmp");
    std::fs::write(&temp_path, content).map_err(AppError::Io)?;
    replace_file_with_backup(path, &temp_path)?;
    Ok(())
}

fn replace_file_with_backup(path: &Path, temp_path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        std::fs::rename(temp_path, path).map_err(AppError::Io)?;
        return Ok(());
    }

    let backup_path = path.with_extension("toml.bak");
    if backup_path.exists() {
        std::fs::remove_file(&backup_path).map_err(AppError::Io)?;
    }
    std::fs::rename(path, &backup_path).map_err(AppError::Io)?;
    match std::fs::rename(temp_path, path) {
        Ok(()) => {
            let _ = std::fs::remove_file(&backup_path);
            Ok(())
        }
        Err(replace_err) => match std::fs::rename(&backup_path, path) {
            Ok(()) => Err(AppError::Io(replace_err)),
            Err(restore_err) => Err(AppError::ConfigError(format!(
                "配置文件替换失败：{replace_err}；旧配置恢复失败：{restore_err}"
            ))),
        },
    }
}

fn apply_env_overrides(config: &mut AppConfig, portable_home: &Path) -> Result<(), AppError> {
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

fn env_string(key: &str) -> Result<Option<String>, AppError> {
    match std::env::var(key) {
        Ok(value) => Ok(Some(validate_env_value(key, value)?)),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(AppError::ConfigError(format!(
            "{key} 必须是有效 Unicode 字符串"
        ))),
    }
}

fn validate_env_value(key: &str, value: String) -> Result<String, AppError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(AppError::ConfigError(format!("{key} 不能为空")));
    }
    Ok(value)
}

fn env_path(key: &str, portable_home: &Path) -> Result<Option<PathBuf>, AppError> {
    env_string(key).map(|value| {
        value.map(|path| resolve_path_in_portable_home(PathBuf::from(path), portable_home))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "{prefix}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn test_database_url() -> String {
        "postgres://a4n:a4n@127.0.0.1:5432/a4n_test".to_string()
    }

    fn test_config(root: &Path) -> AppConfig {
        let data_dir = root.join("data");
        AppConfig {
            database_url: test_database_url(),
            data_dir: data_dir.clone(),
            cache_dir: data_dir.join("cache"),
            temp_dir: data_dir.join("tmp"),
            log_dir: data_dir.join("logs"),
            ..AppConfig::default()
        }
    }

    #[test]
    fn load_config_from_file_creates_portable_config_when_missing() {
        let root = unique_temp_dir("a4n_config_missing");
        let default_config = test_config(&root);
        let config_path = default_config.data_dir.join("config.toml");

        let loaded = load_config_from_file(default_config.clone(), &config_path, &root).unwrap();

        assert_eq!(loaded.database_url, default_config.database_url);
        assert_eq!(loaded.data_dir, default_config.data_dir);
        assert_eq!(loaded.cache_dir, default_config.cache_dir);
        assert_eq!(loaded.temp_dir, default_config.temp_dir);
        assert_eq!(loaded.log_dir, default_config.log_dir);
        assert!(config_path.is_file());

        let persisted: AppConfig =
            toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(persisted.database_url, default_config.database_url);
        assert_eq!(persisted.data_dir, default_config.data_dir);
        assert_eq!(persisted.cache_dir, default_config.cache_dir);
        assert_eq!(persisted.temp_dir, default_config.temp_dir);
        assert_eq!(persisted.log_dir, default_config.log_dir);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn write_config_file_replaces_existing_config_without_temp_or_backup_leftovers() {
        let root = unique_temp_dir("a4n_config_replace");
        let default_config = test_config(&root);
        let config_path = default_config.data_dir.join("config.toml");
        let temp_path = config_path.with_extension("toml.tmp");
        let backup_path = config_path.with_extension("toml.bak");

        let mut first = default_config.clone();
        first.llm.model = "first-model".into();
        write_config_file(&config_path, &first).unwrap();

        let mut second = default_config;
        second.llm.model = "second-model".into();
        write_config_file(&config_path, &second).unwrap();

        let persisted: AppConfig =
            toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(persisted.llm.model, "second-model");
        assert!(!temp_path.exists());
        assert!(!backup_path.exists());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn load_config_from_file_preserves_explicit_absolute_storage_paths() {
        let root = unique_temp_dir("a4n_config_explicit_paths");
        let default_config = test_config(&root);
        let config_path = default_config.data_dir.join("config.toml");
        let explicit_data = root.join("explicit-data");
        let mut explicit_config = default_config.clone();
        explicit_config.database_url = "postgres://a4n:a4n@127.0.0.1:5432/explicit".into();
        explicit_config.data_dir = explicit_data.clone();
        explicit_config.cache_dir = root.join("explicit-cache");
        explicit_config.temp_dir = root.join("explicit-tmp");
        explicit_config.log_dir = root.join("explicit-logs");
        explicit_config.llm.model = "explicit-path-config-kept".into();
        write_config_file(&config_path, &explicit_config).unwrap();

        let loaded = load_config_from_file(default_config, &config_path, &root).unwrap();

        assert_eq!(
            loaded.database_url,
            "postgres://a4n:a4n@127.0.0.1:5432/explicit"
        );
        assert_eq!(loaded.data_dir, explicit_data);
        assert_eq!(loaded.cache_dir, root.join("explicit-cache"));
        assert_eq!(loaded.temp_dir, root.join("explicit-tmp"));
        assert_eq!(loaded.log_dir, root.join("explicit-logs"));
        assert_eq!(loaded.llm.model, "explicit-path-config-kept");

        let persisted: AppConfig =
            toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(persisted.database_url, loaded.database_url);
        assert_eq!(persisted.data_dir, loaded.data_dir);
        assert_eq!(persisted.cache_dir, loaded.cache_dir);
        assert_eq!(persisted.temp_dir, loaded.temp_dir);
        assert_eq!(persisted.log_dir, loaded.log_dir);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn load_config_from_file_resolves_relative_storage_paths_to_portable_home() {
        let root = unique_temp_dir("a4n_config_relative_paths");
        let default_config = test_config(&root);
        let config_path = default_config.data_dir.join("config.toml");
        let mut relative_config = default_config.clone();
        relative_config.database_url = "postgres://a4n:a4n@127.0.0.1:5432/relative".into();
        relative_config.data_dir = PathBuf::from("data");
        relative_config.cache_dir = PathBuf::from("data/cache");
        relative_config.temp_dir = PathBuf::from("data/tmp");
        relative_config.log_dir = PathBuf::from("data/logs");
        relative_config.embedding.local_model_path = Some(PathBuf::from("data/models/embed"));
        write_config_file(&config_path, &relative_config).unwrap();

        let loaded = load_config_from_file(default_config, &config_path, &root).unwrap();

        assert_eq!(
            loaded.database_url,
            "postgres://a4n:a4n@127.0.0.1:5432/relative"
        );
        assert_eq!(loaded.data_dir, root.join("data"));
        assert_eq!(loaded.cache_dir, root.join("data/cache"));
        assert_eq!(loaded.temp_dir, root.join("data/tmp"));
        assert_eq!(loaded.log_dir, root.join("data/logs"));
        assert_eq!(
            loaded.embedding.local_model_path,
            Some(root.join("data/models/embed"))
        );

        let persisted: AppConfig =
            toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            persisted.database_url,
            "postgres://a4n:a4n@127.0.0.1:5432/relative"
        );
        assert_eq!(persisted.data_dir, PathBuf::from("data"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn env_override_values_must_not_be_empty() {
        let err = validate_env_value("A4N_DATA_DIR", "  ".into()).unwrap_err();
        assert!(err.to_string().contains("A4N_DATA_DIR 不能为空"));
    }

    #[test]
    fn env_override_values_are_trimmed() {
        let value = validate_env_value("A4N_DATA_DIR", " /tmp/a4n-data ".into()).unwrap();
        assert_eq!(value, "/tmp/a4n-data");
    }
}
