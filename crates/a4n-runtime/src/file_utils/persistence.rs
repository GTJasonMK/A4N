use std::{io::Write, path::Path};

use super::{
    paths::canonicalize_path_in_data_dir,
    remove_created_file_on_error_sync,
    rules::{normalize_relative_data_path, validate_missing_delete_target_parent},
};

pub use a4n_core::files::{data_file_record_facts, persist_created_media_file};

pub fn probe_created_audio_duration(path: &Path, file_label: &str) -> Result<f32, String> {
    remove_created_file_on_error_sync(path, file_label, || {
        a4n_media::probe_audio_duration_secs(path)
            .map_err(|error| format!("无法读取{file_label}时长：{error}"))
    })
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
    validate_missing_delete_target_parent(&canonical_data_dir, &target, label)?;
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
    remove_created_file_on_error_sync(&target, label, move || {
        let result = file
            .write_all(bytes)
            .map_err(|error| format!("写入{label}失败：{target_display}：{error}"));
        drop(file);
        result
    })
}
