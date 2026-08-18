use std::path::{Path, PathBuf};

use crate::config::env::optional_non_empty_env_path;
use crate::error::AppError;

use super::contracts::AppConfig;

/// 返回应用旁的便携式根目录。
///
/// 默认所有可写数据都会落到 `{portable_home}/data/`。解析顺序：
/// 1. `A4N_PORTABLE_HOME` 显式指定的目录；
/// 2. Linux AppImage 的 `APPIMAGE` 文件所在目录；
/// 3. 当前可执行文件所在目录（macOS `.app` 和 `bin/` 打包布局会提升到包根目录）；
/// 4. 如果运行时无法解析可执行文件路径，启动失败。
pub fn portable_home_dir() -> PathBuf {
    resolve_portable_home_dir().expect("无法解析 A4N 便携式根目录")
}

/// 把相对路径解析到当前便携式应用目录下，绝对路径保持不变。
pub fn resolve_portable_path(path: impl AsRef<Path>) -> PathBuf {
    resolve_path_in_portable_home(path, &portable_home_dir())
}

/// 把相对路径解析到指定便携式应用目录下，绝对路径保持不变。
pub fn resolve_path_in_portable_home(path: impl AsRef<Path>, portable_home: &Path) -> PathBuf {
    let path = path.as_ref();
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        portable_home.join(path)
    }
}

pub fn apply_storage_env_overrides(
    config: &mut AppConfig,
    portable_home: &Path,
) -> Result<(), AppError> {
    let data_dir = optional_non_empty_env_path("A4N_DATA_DIR")?
        .map(|path| resolve_path_in_portable_home(path, portable_home));
    let cache_dir = optional_non_empty_env_path("A4N_CACHE_DIR")?
        .map(|path| resolve_path_in_portable_home(path, portable_home));
    let temp_dir = optional_non_empty_env_path("A4N_TEMP_DIR")?
        .map(|path| resolve_path_in_portable_home(path, portable_home));
    let log_dir = optional_non_empty_env_path("A4N_LOG_DIR")?
        .map(|path| resolve_path_in_portable_home(path, portable_home));
    let data_dir_overridden = data_dir.is_some();

    if let Some(path) = data_dir {
        config.data_dir = path;
    }
    if let Some(path) = cache_dir {
        config.cache_dir = path;
    } else if data_dir_overridden {
        config.cache_dir = config.data_dir.join("cache");
    }
    if let Some(path) = temp_dir {
        config.temp_dir = path;
    } else if data_dir_overridden {
        config.temp_dir = config.data_dir.join("tmp");
    }
    if let Some(path) = log_dir {
        config.log_dir = path;
    } else if data_dir_overridden {
        config.log_dir = config.data_dir.join("logs");
    }
    Ok(())
}

fn resolve_portable_home_dir() -> std::io::Result<PathBuf> {
    if let Some(path) = optional_non_empty_env_path("A4N_PORTABLE_HOME")
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error.to_string()))?
    {
        return Ok(path);
    }

    if let Some(parent) = optional_non_empty_env_path("APPIMAGE")
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error.to_string()))?
        .and_then(|appimage| appimage.parent().map(std::path::Path::to_path_buf))
    {
        return Ok(parent);
    }

    let exe = std::env::current_exe()?;
    let parent = exe.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "当前可执行文件路径缺少父目录")
    })?;

    portable_home_from_exe_parent(parent)
}

fn portable_home_from_exe_parent(parent: &Path) -> std::io::Result<PathBuf> {
    if let Some(package_root) = parent
        .file_name()
        .is_some_and(|name| name == "bin")
        .then(|| parent.parent())
        .flatten()
        .filter(|package_root| looks_like_a4n_package_root(package_root))
    {
        return Ok(package_root.to_path_buf());
    }

    let components: Vec<_> = parent.components().collect();
    if components.len() >= 3 {
        let maybe_macos = components[components.len() - 1].as_os_str();
        let maybe_contents = components[components.len() - 2].as_os_str();
        let maybe_app = components[components.len() - 3].as_os_str();
        if maybe_macos == "MacOS"
            && maybe_contents == "Contents"
            && maybe_app.to_string_lossy().ends_with(".app")
        {
            return parent
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .map(Path::to_path_buf)
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "macOS .app 路径缺少包根目录")
                });
        }
    }
    Ok(parent.to_path_buf())
}

fn looks_like_a4n_package_root(path: &Path) -> bool {
    path.join("data").is_dir() || path.join("prompts").is_dir() || path.join("static").is_dir()
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

    #[test]
    fn resolves_relative_paths_inside_portable_home() {
        let root = unique_temp_dir("a4n_portable_path");

        assert_eq!(
            resolve_path_in_portable_home("data/cache", &root),
            root.join("data/cache")
        );
        assert_eq!(
            resolve_path_in_portable_home(Path::new("/tmp/a4n-cache"), &root),
            PathBuf::from("/tmp/a4n-cache")
        );
    }

    #[test]
    fn portable_home_from_bin_package_returns_package_root() {
        let root = unique_temp_dir("a4n_portable_bin");
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(root.join("data")).unwrap();

        assert_eq!(portable_home_from_exe_parent(&bin).unwrap(), root);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn portable_home_from_plain_bin_directory_stays_in_bin() {
        let root = unique_temp_dir("a4n_plain_bin");
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).unwrap();

        assert_eq!(portable_home_from_exe_parent(&bin).unwrap(), bin);

        let _ = std::fs::remove_dir_all(&root);
    }
}
