use a4n_config::app_config::{AppConfig, resolve_path_in_portable_home};
use a4n_config::optional_non_empty_env_path;
use a4n_error::AppError;
use std::path::{Path, PathBuf};

pub(super) fn load_config_from_file(
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

pub(super) fn config_file_path(
    default_config: &AppConfig,
    portable_home: &Path,
) -> Result<PathBuf, AppError> {
    let data_dir = optional_non_empty_env_path("A4N_DATA_DIR")?
        .map(|path| resolve_path_in_portable_home(path, portable_home))
        .unwrap_or_else(|| default_config.data_dir.clone());
    Ok(data_dir.join("config.toml"))
}

pub(super) fn normalize_storage_paths(
    config: &mut AppConfig,
    portable_home: &Path,
) -> Result<(), AppError> {
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

pub(super) fn write_config_file(path: &Path, config: &AppConfig) -> Result<(), AppError> {
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

pub(super) fn replace_file_with_backup(path: &Path, temp_path: &Path) -> Result<(), AppError> {
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
        Ok(()) => std::fs::remove_file(&backup_path).map_err(|error| {
            AppError::ConfigError(format!(
                "配置文件已替换，但删除旧配置备份失败：{}：{error}",
                backup_path.display()
            ))
        }),
        Err(replace_err) => match std::fs::rename(&backup_path, path) {
            Ok(()) => Err(AppError::Io(replace_err)),
            Err(restore_err) => Err(AppError::ConfigError(format!(
                "配置文件替换失败：{replace_err}；旧配置恢复失败：{restore_err}"
            ))),
        },
    }
}
