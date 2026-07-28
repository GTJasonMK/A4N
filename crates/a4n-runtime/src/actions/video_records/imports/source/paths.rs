use super::super::contracts::ImportedVideoSource;
use super::super::prelude::{Path, PathBuf, ProjectPaths, ensure_chapter_videos_dir};
use a4n_orchestrator::video::import_video_extension;

pub(in crate::actions::video_records::imports) fn import_video_source(
    file_path: &str,
    label: &str,
) -> Result<ImportedVideoSource, String> {
    let path = import_video_source_path(file_path)?;
    let metadata = std::fs::metadata(&path).map_err(|e| format!("无法读取{label}元数据：{e}"))?;
    let file_size =
        i64::try_from(metadata.len()).map_err(|_| format!("{label}文件过大，无法记录文件大小"))?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("video")
        .to_string();
    let extension = import_video_extension(&path)?;
    Ok(ImportedVideoSource {
        path,
        file_size,
        file_name,
        extension,
        temporary_path: None,
    })
}

pub(in crate::actions::video_records) fn import_video_source_path(
    file_path: &str,
) -> Result<PathBuf, String> {
    let trimmed = file_path.trim();
    if trimmed.is_empty() {
        return Err("请选择要导入的视频文件".to_string());
    }
    let path = PathBuf::from(trimmed);
    let metadata = std::fs::metadata(&path).map_err(|e| format!("无法读取导入视频文件：{e}"))?;
    if !metadata.is_file() {
        return Err("导入路径不是文件".to_string());
    }
    import_video_extension(&path)?;
    path.canonicalize()
        .map_err(|e| format!("无法解析导入视频路径：{e}"))
}

pub(in crate::actions::video_records) fn import_video_output_path(
    data_dir: &Path,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
    prefix: &str,
    extension: &str,
) -> Result<PathBuf, String> {
    let dir = ensure_chapter_videos_dir(data_dir, project_id, chapter_id)?;
    Ok(dir.join(format!("{prefix}_{video_id}.{extension}")))
}

pub(in crate::actions::video_records::imports) fn import_project_video_output_path(
    data_dir: &Path,
    project_id: i64,
    video_id: i64,
    prefix: &str,
    extension: &str,
) -> Result<PathBuf, String> {
    let dir = ProjectPaths::new(data_dir, project_id).videos_dir();
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建项目视频目录：{err}"))?;
    Ok(dir.join(format!("{prefix}_{video_id}.{extension}")))
}
