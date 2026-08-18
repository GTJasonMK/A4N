mod environment;
mod file_io;

use self::environment::apply_env_overrides;
#[cfg(test)]
use self::file_io::write_config_file;
use self::file_io::{config_file_path, load_config_from_file};
use a4n_core::config::app_config::{AppConfig, portable_home_dir};
use a4n_core::config::env::validate_postgres_database_url;
use a4n_core::error::AppError;
use tauri::AppHandle;

pub(super) fn load_config(_app: &AppHandle) -> Result<AppConfig, AppError> {
    let portable_home = portable_home_dir();
    let default_config = AppConfig::default();
    let config_path = config_file_path(&default_config, &portable_home)?;
    let mut config = load_config_from_file(default_config, &config_path, &portable_home)?;

    apply_env_overrides(&mut config, &portable_home)?;
    validate_postgres_database_url("A4N_DATABASE_URL", &config.database_url)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

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
}
