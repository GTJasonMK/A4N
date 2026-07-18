use crate::file_utils::{
    paths::canonicalize_path_in_data_dir,
    rules::{normalize_relative_data_path, validate_missing_delete_target_parent},
};
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

pub fn deletable_data_file_path(
    data_dir: &Path,
    path: &str,
    label: &str,
) -> Result<Option<PathBuf>, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(format!("{label}路径不能为空"));
    }
    let raw_path = Path::new(trimmed);
    if raw_path.is_absolute() {
        return Err(format!("{label}路径必须是应用数据目录内的相对路径"));
    }
    let relative_path = normalize_relative_data_path(raw_path, label)?;
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|error| format!("无法解析数据目录：{error}"))?;
    let candidate = canonical_data_dir.join(relative_path);
    match std::fs::metadata(&candidate) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(format!("{label}不是文件"));
            }
            canonicalize_path_in_data_dir(&canonical_data_dir, &candidate, label)
                .map(Some)
                .map_err(|error| error.to_string())
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            validate_missing_delete_target_parent(&canonical_data_dir, &candidate, label)?;
            Ok(None)
        }
        Err(error) => Err(format!("无法读取{label}元数据：{error}")),
    }
}

pub fn deletable_data_dir_path(
    data_dir: &Path,
    path: &Path,
    label: &str,
) -> Result<Option<PathBuf>, String> {
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|error| format!("无法解析数据目录：{error}"))?;
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        canonical_data_dir.join(normalize_relative_data_path(path, label)?)
    };
    match std::fs::symlink_metadata(&candidate) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(format!("{label}不能是符号链接"));
            }
            if !metadata.is_dir() {
                return Err(format!("{label}不是目录"));
            }
            let canonical_path =
                canonicalize_path_in_data_dir(&canonical_data_dir, &candidate, label)
                    .map_err(|error| error.to_string())?;
            if canonical_path == canonical_data_dir {
                return Err(format!("{label}不能是应用数据目录"));
            }
            Ok(Some(canonical_path))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            validate_missing_delete_target_parent(&canonical_data_dir, &candidate, label)?;
            Ok(None)
        }
        Err(error) => Err(format!("无法读取{label}元数据：{error}")),
    }
}
