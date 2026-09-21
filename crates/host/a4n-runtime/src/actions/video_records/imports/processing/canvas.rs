use a4n_storage::create_unique_timestamped_file;
use a4n_storage::paths::ProjectPaths;
use std::path::{Path, PathBuf};

pub(in crate::actions::video_records::imports) fn canvas_frame_output_path(
    data_dir: &Path,
    project_id: i64,
    video_id: i64,
    frame_kind: &str,
) -> Result<PathBuf, String> {
    let dir = ProjectPaths::new(data_dir, project_id).images_dir();
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建画布抽帧图片目录：{err}"))?;
    let (path, file) = create_unique_timestamped_file(
        &dir,
        &format!("canvas_frame_video_{video_id}_{frame_kind}"),
        "jpg",
    )
    .map_err(|error| format!("创建画布抽帧图片失败：{error}"))?;
    drop(file);
    Ok(path)
}
