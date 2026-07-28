use a4n_core::files::paths::ProjectPaths;
use a4n_core::models::ai::generated_video;
use std::path::{Path, PathBuf};

pub(in crate::actions::video_generation::media) fn required_trim_chapter_id(
    source: &generated_video::Model,
) -> Result<i64, String> {
    source
        .chapter_id
        .filter(|id| *id > 0)
        .ok_or_else(|| format!("源视频 #{} 缺少 chapter_id，无法裁剪", source.id))
}

pub(crate) fn ensure_chapter_videos_dir(
    data_dir: &Path,
    project_id: i64,
    chapter_id: i64,
) -> Result<PathBuf, String> {
    let dir = chapter_videos_dir(data_dir, project_id, chapter_id)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建视频目录 {dir:?}：{e}"))?;
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
    source: &generated_video::Model,
    start_secs: f32,
    end_secs: f32,
) -> Result<PathBuf, String> {
    let chapter_id = required_trim_chapter_id(source)?;
    let dir = ensure_chapter_videos_dir(data_dir, source.project_id, chapter_id)?;
    Ok(dir.join(format!(
        "trim_{}_{}_{}.mp4",
        source.id,
        millis_label(start_secs)?,
        millis_label(end_secs)?
    )))
}

fn millis_label(value: f32) -> Result<i32, String> {
    let millis = (f64::from(value) * 1000.0).round();
    if !millis.is_finite() || millis < f64::from(i32::MIN) || millis > f64::from(i32::MAX) {
        return Err(format!("视频时间无法转换为 i32 毫秒标签：{value}"));
    }
    Ok(millis as i32)
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
