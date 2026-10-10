use super::paths::{ensure_chapter_videos_dir, ensure_project_videos_dir};
use a4n_domain::media::canonical_video_extension;
use a4n_storage::{persist_created_media_file, remove_created_file_on_error_sync};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub(crate) fn save_video_bytes(
    data_dir: &Path,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
    bytes: &[u8],
    format: &str,
) -> Result<PathBuf, String> {
    let dir = ensure_chapter_videos_dir(data_dir, project_id, chapter_id)?;
    let extension = required_video_file_extension(format)?;
    let path = dir.join(format!("video_{video_id}.{extension}"));
    remove_created_file_on_error_sync(&path, "视频文件", || {
        std::fs::write(&path, bytes).map_err(|e| format!("无法写入视频：{e}"))
    })?;
    Ok(path)
}

pub(crate) fn required_video_file_extension(format: &str) -> Result<&'static str, String> {
    let format = format.trim();
    if format.is_empty() {
        return Err("视频 provider 未返回文件格式，无法保存视频".to_string());
    }
    canonical_video_extension(format)
        .ok_or_else(|| format!("视频 provider 返回了不支持的文件格式：{format}"))
}

pub(crate) fn persist_canvas_video_bytes(
    data_dir: &Path,
    project_id: i64,
    video_id: i64,
    bytes: &[u8],
    format: &str,
) -> Result<(String, i64), String> {
    if bytes.is_empty() {
        return Err("画布视频 provider 返回了空文件，无法保存生成结果".to_string());
    }
    let extension = required_video_file_extension(format)?;
    let dir = ensure_project_videos_dir(data_dir, project_id)?;
    let output = dir.join(format!("canvas_video_{video_id}.{extension}"));
    persist_created_media_file(
        data_dir,
        &output,
        |path| std::fs::write(path, bytes).map_err(|error| format!("无法写入画布视频：{error}")),
        "已写出的画布视频文件",
        "画布视频文件",
        "画布视频文件",
    )
}

pub(crate) fn remove_partial_video_file(path: &Path, label: &str) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("{label}删除失败：{err}")),
    }
}
