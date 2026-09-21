use crate::actions::video_generation::media::{
    ensure_chapter_videos_dir, ensure_project_videos_dir,
};
use std::path::{Path, PathBuf};

pub(in crate::actions::video_records) fn video_record_output_path(
    data_dir: &Path,
    project_id: i64,
    chapter_id: Option<i64>,
    video_id: i64,
    prefix: &str,
    extension: &str,
) -> Result<PathBuf, String> {
    let dir = match chapter_id {
        Some(chapter_id) => ensure_chapter_videos_dir(data_dir, project_id, chapter_id)?,
        None => ensure_project_videos_dir(data_dir, project_id)?,
    };
    Ok(dir.join(format!("{prefix}_{video_id}.{extension}")))
}
