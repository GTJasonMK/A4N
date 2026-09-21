use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

use super::paths::canonicalize_path_in_data_dir;

fn normalize_relative_data_path(path: &Path, label: &str) -> Result<PathBuf, String> {
    let mut output = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => output.push(value),
            Component::CurDir => {}
            Component::ParentDir => {
                if !output.pop() {
                    return Err(format!("{label}不在应用数据目录内"));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!("{label}路径必须是应用数据目录内的相对路径"));
            }
        }
    }
    if output.as_os_str().is_empty() {
        return Err(format!("{label}路径不能为空"));
    }
    Ok(output)
}

fn validate_missing_target_parent(
    canonical_data_dir: &Path,
    candidate: &Path,
    label: &str,
) -> Result<(), String> {
    for ancestor in candidate.ancestors().skip(1) {
        match std::fs::metadata(ancestor) {
            Ok(metadata) => {
                if !metadata.is_dir() {
                    return Err(format!("{label}父路径不是目录"));
                }
                let canonical_ancestor = ancestor
                    .canonicalize()
                    .map_err(|error| format!("无法解析{label}父路径：{error}"))?;
                if !canonical_ancestor.starts_with(canonical_data_dir) {
                    return Err(format!("{label}不在应用数据目录内"));
                }
                return Ok(());
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(format!("无法读取{label}父路径：{error}")),
        }
    }
    Err(format!("{label}不在应用数据目录内"))
}

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
            validate_missing_target_parent(&canonical_data_dir, &candidate, label)?;
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
            validate_missing_target_parent(&canonical_data_dir, &candidate, label)?;
            Ok(None)
        }
        Err(error) => Err(format!("无法读取{label}元数据：{error}")),
    }
}

pub fn remove_data_file_in_data_dir(
    data_dir: &Path,
    path: &str,
    label: &str,
) -> Result<(), String> {
    let Some(canonical_path) = deletable_data_file_path(data_dir, path, label)? else {
        return Ok(());
    };
    match std::fs::remove_file(&canonical_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "{label}删除失败：{}：{error}",
            canonical_path.display()
        )),
    }
}

pub fn remove_data_dir_in_data_dir(
    data_dir: &Path,
    path: &Path,
    label: &str,
) -> Result<(), String> {
    let Some(canonical_path) = deletable_data_dir_path(data_dir, path, label)? else {
        return Ok(());
    };
    match std::fs::remove_dir_all(&canonical_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "{label}删除失败：{}：{error}",
            canonical_path.display()
        )),
    }
}

pub fn write_new_data_file(
    data_dir: &Path,
    relative_path: &str,
    bytes: &[u8],
    label: &str,
) -> Result<(), String> {
    let relative = normalize_relative_data_path(Path::new(relative_path.trim()), label)?;
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|error| format!("无法解析数据目录：{error}"))?;
    let target = canonical_data_dir.join(&relative);
    validate_missing_target_parent(&canonical_data_dir, &target, label)?;
    let parent = target
        .parent()
        .ok_or_else(|| format!("{label}目标路径缺少父目录"))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("创建{label}目录失败：{}：{error}", parent.display()))?;
    let parent = canonicalize_path_in_data_dir(&canonical_data_dir, parent, label)
        .map_err(|error| error.to_string())?;
    let file_name = relative
        .file_name()
        .ok_or_else(|| format!("{label}目标路径缺少文件名"))?;
    let target = parent.join(file_name);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(|error| format!("创建{label}失败：{}：{error}", target.display()))?;
    let target_display = target.display().to_string();
    super::remove_created_file_on_error_sync(&target, label, move || {
        let result = file
            .write_all(bytes)
            .map_err(|error| format!("写入{label}失败：{target_display}：{error}"));
        drop(file);
        result
    })
}
