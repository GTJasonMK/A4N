use a4n_storage::{create_unique_timestamped_file, paths::ProjectPaths};
use std::path::{Path, PathBuf};

pub(crate) fn ensure_chapter_videos_dir(
    data_dir: &Path,
    project_id: i64,
    chapter_id: i64,
) -> Result<PathBuf, String> {
    let dir = chapter_videos_dir(data_dir, project_id, chapter_id)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建视频目录 {dir:?}：{e}"))?;
    Ok(dir)
}

pub(crate) fn ensure_project_videos_dir(
    data_dir: &Path,
    project_id: i64,
) -> Result<PathBuf, String> {
    let dir = ProjectPaths::new(data_dir, project_id).videos_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("无法创建项目视频目录 {}：{error}", dir.display()))?;
    Ok(dir)
}

pub(in crate::actions::video_generation::media) fn video_tail_frame_path(
    data_dir: &Path,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
) -> Result<PathBuf, String> {
    let dir = chapter_tail_frame_images_dir(data_dir, project_id, chapter_id)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建图片目录 {dir:?}：{e}"))?;
    Ok(dir.join(format!("video_tail_frame_{video_id}.jpg")))
}

pub(crate) fn trimmed_video_path(
    data_dir: &Path,
    project_id: i64,
    chapter_id: Option<i64>,
    source_id: i64,
) -> Result<PathBuf, String> {
    let dir = match chapter_id {
        Some(chapter_id) => ensure_chapter_videos_dir(data_dir, project_id, chapter_id)?,
        None => ensure_project_videos_dir(data_dir, project_id)?,
    };
    let (path, file) = create_unique_timestamped_file(&dir, &format!("trim_{source_id}"), "mp4")
        .map_err(|error| format!("创建裁剪视频文件失败：{error}"))?;
    drop(file);
    Ok(path)
}

pub(in crate::actions::video_generation::media) fn chapter_videos_dir(
    data_dir: &Path,
    project_id: i64,
    chapter_id: i64,
) -> Result<PathBuf, String> {
    required_asset_chapter_id(chapter_id)?;
    Ok(ProjectPaths::new(data_dir, project_id).chapter_videos_dir(chapter_id))
}

pub(in crate::actions::video_generation::media) fn chapter_tail_frame_images_dir(
    data_dir: &Path,
    project_id: i64,
    chapter_id: i64,
) -> Result<PathBuf, String> {
    required_asset_chapter_id(chapter_id)?;
    Ok(ProjectPaths::new(data_dir, project_id).chapter_images_dir(chapter_id))
}

fn required_asset_chapter_id(chapter_id: i64) -> Result<(), String> {
    if chapter_id > 0 {
        Ok(())
    } else {
        Err(format!(
            "章节资产目录需要有效 chapter_id，收到 {chapter_id}"
        ))
    }
}
