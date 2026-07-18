use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

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
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|e| format!("无法解析数据目录：{e}"))?;
    let canonical_path =
        canonicalize_path_in_data_dir(data_dir, path, label).map_err(|error| error.to_string())?;
    canonical_path
        .strip_prefix(&canonical_data_dir)
        .map_err(|_| format!("{label}不在应用数据目录内"))
        .map(path_to_db_string)
}

pub fn path_to_db_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
