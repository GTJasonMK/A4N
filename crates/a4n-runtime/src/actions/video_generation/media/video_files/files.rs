use super::paths::ensure_chapter_videos_dir;
use crate::file_utils::{persist_created_media_file, remove_created_file_on_error_sync};
use a4n_core::files::paths::ProjectPaths;
use a4n_core::models::ai::generated_video;
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
    match format.to_ascii_lowercase().as_str() {
        "mp4" => Ok("mp4"),
        "webm" => Ok("webm"),
        "mov" => Ok("mov"),
        "m4v" => Ok("m4v"),
        _ => Err(format!("视频 provider 返回了不支持的文件格式：{format}")),
    }
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
    let dir = ProjectPaths::new(data_dir, project_id).videos_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("无法创建画布视频目录 {}：{error}", dir.display()))?;
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

pub(crate) fn video_mime_from_path(path: &Path) -> Result<&'static str, String> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("mp4" | "m4v") => Ok("video/mp4"),
        Some("webm") => Ok("video/webm"),
        Some("mov") => Ok("video/quicktime"),
        Some(other) => Err(format!("视频文件扩展名不支持播放：{other}")),
        None => Err("视频文件缺少扩展名，无法确定 MIME 类型".to_string()),
    }
}

pub(crate) fn remove_partial_video_file(path: &Path, label: &str) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("{label}删除失败：{err}")),
    }
}

pub(crate) fn append_secondary_error(mut message: String, secondary: Option<String>) -> String {
    if let Some(secondary) = secondary {
        message.push('；');
        message.push_str(&secondary);
    }
    message
}

pub(crate) fn combine_optional_errors(
    primary: Option<String>,
    secondary: Option<String>,
) -> Option<String> {
    match primary {
        Some(primary) => Some(append_secondary_error(primary, secondary)),
        None => secondary,
    }
}

pub(crate) fn prompt_id_from_video(video: &generated_video::Model) -> Option<i64> {
    video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("prompt_id"))
        .and_then(|value| value.as_i64())
}
