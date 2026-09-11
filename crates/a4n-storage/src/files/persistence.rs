use std::path::Path;

use super::{paths::data_dir_relative_file_path, remove_created_file_on_error_sync};

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
