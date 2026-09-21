use std::{
    fs::File,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use super::{
    deletable_data_file_path, paths::data_dir_relative_file_path, remove_created_file_on_error_sync,
};

pub fn persist_created_media_file<R>(
    data_dir: &Path,
    output: &Path,
    create_file: R,
    file_label: &str,
    relative_path_label: &str,
    metadata_label: &str,
) -> Result<(String, i64), String>
where
    R: FnOnce(&Path) -> Result<(), String>,
{
    remove_created_file_on_error_sync(output, file_label, || {
        create_file(output)?;
        data_file_record_facts(data_dir, output, relative_path_label, metadata_label)
    })
}

pub fn data_file_record_facts(
    data_dir: &Path,
    path: &Path,
    relative_path_label: &str,
    metadata_label: &str,
) -> Result<(String, i64), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("无法读取{metadata_label}元数据：{error}"))?;
    if !metadata.is_file() {
        return Err(format!("{metadata_label}不是文件"));
    }
    let file_size = i64::try_from(metadata.len())
        .map_err(|_| format!("{metadata_label}过大，无法记录文件大小"))?;
    let relative_path = data_dir_relative_file_path(data_dir, path, relative_path_label)?;
    Ok((relative_path, file_size))
}

/// Reads the durable facts used by publication recovery and cleanup fencing.
pub fn inspect_data_file_facts(
    data_dir: &Path,
    relative_path: &str,
    label: &str,
) -> Result<Option<(PathBuf, i64, String)>, String> {
    let Some(path) = deletable_data_file_path(data_dir, relative_path, label)? else {
        return Ok(None);
    };
    let bytes =
        std::fs::read(&path).map_err(|error| format!("读取{}失败：{error}", path.display()))?;
    let file_size =
        i64::try_from(bytes.len()).map_err(|_| format!("{label}过大，无法记录文件大小"))?;
    Ok(Some((
        path,
        file_size,
        format!("{:x}", Sha256::digest(bytes)),
    )))
}

/// Promotes a staged file without weakening the data-directory boundary.
/// Publication rows remain the crash-recovery source of truth if the process
/// stops after rename and before the database commit.
pub fn promote_data_file(
    data_dir: &Path,
    staging_path: &str,
    final_path: &str,
    label: &str,
) -> Result<(), String> {
    let source = deletable_data_file_path(data_dir, staging_path, label)?
        .ok_or_else(|| format!("{label}暂存文件不存在"))?;
    if deletable_data_file_path(data_dir, final_path, label)?.is_some() {
        return Err(format!("{label}最终文件已存在"));
    }
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|error| format!("无法解析数据目录：{error}"))?;
    let target = canonical_data_dir.join(Path::new(final_path.trim()));
    std::fs::rename(&source, &target).map_err(|error| format!("{label}原子提升失败：{error}"))?;
    File::open(&target)
        .and_then(|file| file.sync_all())
        .map_err(|error| format!("{label}提升后同步失败：{error}"))
}
