//! 项目目录结构与路径解析
//!
//! 定义 A4N 项目文件系统的标准目录结构：
//!
//! ```text
//! {data_dir}/
//! └── projects/
//!     └── {project_id}/
//!         ├── images/             # 项目级图片与章节图片
//!         │   └── chapter_{id}/
//!         ├── videos/             # 章节视频
//!         │   └── chapter_{id}/
//!         ├── portraits/          # 角色立绘
//!         └── audio/              # Galgame/画布音频
//! ```

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

mod app;
mod project;

pub use app::AppPaths;
pub use project::ProjectPaths;

#[derive(Debug)]
pub enum CanonicalDataPathError {
    DataDirectory(io::Error),
    Target { label: String, source: io::Error },
    OutsideDataDirectory { label: String },
}

impl CanonicalDataPathError {
    pub fn is_outside_data_directory(&self) -> bool {
        matches!(self, Self::OutsideDataDirectory { .. })
    }
}

impl fmt::Display for CanonicalDataPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DataDirectory(source) => write!(formatter, "无法解析数据目录：{source}"),
            Self::Target { label, source } => write!(formatter, "无法读取{label}：{source}"),
            Self::OutsideDataDirectory { label } => {
                write!(formatter, "{label}不在应用数据目录内")
            }
        }
    }
}

impl std::error::Error for CanonicalDataPathError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DataDirectory(source) | Self::Target { source, .. } => Some(source),
            Self::OutsideDataDirectory { .. } => None,
        }
    }
}

pub fn canonicalize_path_in_data_dir(
    data_dir: &Path,
    path: &Path,
    label: &str,
) -> Result<PathBuf, CanonicalDataPathError> {
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(CanonicalDataPathError::DataDirectory)?;
    let canonical_path = path
        .canonicalize()
        .map_err(|source| CanonicalDataPathError::Target {
            label: label.to_string(),
            source,
        })?;
    if !canonical_path.starts_with(&canonical_data_dir) {
        return Err(CanonicalDataPathError::OutsideDataDirectory {
            label: label.to_string(),
        });
    }
    Ok(canonical_path)
}

pub fn canonicalize_data_file_path(
    data_dir: &Path,
    path: &str,
    label: &str,
) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(format!("{label}路径不能为空"));
    }
    let raw_path = Path::new(trimmed);
    if raw_path.is_absolute() {
        return Err(format!("{label}路径必须是应用数据目录内的相对路径"));
    }
    let candidate = data_dir.join(raw_path);
    let canonical_path = canonicalize_path_in_data_dir(data_dir, &candidate, label)
        .map_err(|error| error.to_string())?;
    let metadata =
        std::fs::metadata(&canonical_path).map_err(|e| format!("无法读取{label}元数据：{e}"))?;
    if !metadata.is_file() {
        return Err(format!("{label}不是文件"));
    }
    Ok(canonical_path)
}

pub fn data_dir_relative_file_path(
    data_dir: &Path,
    path: &Path,
    label: &str,
) -> Result<String, String> {
    let canonical_path =
        canonicalize_path_in_data_dir(data_dir, path, label).map_err(|error| error.to_string())?;
    data_dir_relative_path(data_dir, &canonical_path, label)
}

pub fn data_dir_relative_path(data_dir: &Path, path: &Path, label: &str) -> Result<String, String> {
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|e| format!("无法解析数据目录：{e}"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("{label}路径缺少文件名"))?;
    let canonical_parent = path
        .parent()
        .ok_or_else(|| format!("{label}路径缺少父目录"))?
        .canonicalize()
        .map_err(|error| format!("无法解析{label}目录：{error}"))?;
    if !canonical_parent.starts_with(&canonical_data_dir) {
        return Err(format!("{label}不在应用数据目录内"));
    }
    let canonical_path = canonical_parent.join(file_name);
    let relative_path = canonical_path
        .strip_prefix(&canonical_data_dir)
        .map_err(|_| format!("{label}不在应用数据目录内"))?;
    path_to_db_string(relative_path)
}

pub fn path_to_db_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(|value| value.replace('\\', "/"))
        .ok_or_else(|| "文件路径包含非 UTF-8 字符".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_project_paths() {
        let paths = ProjectPaths::new(Path::new("/tmp/a4n"), 42);
        assert_eq!(paths.root(), Path::new("/tmp/a4n/projects/42"));
        assert_eq!(
            paths.chapter_images_dir(7),
            Path::new("/tmp/a4n/projects/42/images/chapter_7")
        );
        assert_eq!(
            paths.chapter_videos_dir(7),
            Path::new("/tmp/a4n/projects/42/videos/chapter_7")
        );
        assert_eq!(
            paths.portraits_dir(),
            Path::new("/tmp/a4n/projects/42/portraits")
        );
        assert_eq!(paths.audio_dir(), Path::new("/tmp/a4n/projects/42/audio"));
    }

    #[test]
    fn app_paths_create_all_portable_dirs() {
        let tmp = std::env::temp_dir().join(format!(
            "a4n_app_paths_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let data_dir = tmp.join("data");
        let paths = AppPaths::new(
            data_dir.clone(),
            data_dir.join("cache"),
            data_dir.join("tmp"),
            data_dir.join("logs"),
        );

        paths.ensure_dirs().unwrap();

        for dir in [
            &paths.data_dir,
            &paths.cache_dir,
            &paths.temp_dir,
            &paths.log_dir,
            &paths.exports_dir(),
            &paths.backups_dir(),
            &paths.webview_dir(),
        ] {
            assert!(dir.is_dir(), "expected directory to exist: {dir:?}");
            assert!(
                dir.starts_with(&data_dir),
                "directory escaped data dir: {dir:?}"
            );
        }
        assert_eq!(
            paths.frontend_state_file(),
            data_dir.join("frontend-state.json")
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
