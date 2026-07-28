use super::super::prelude::{Path, PathBuf, ProjectPaths, Utc};

pub(in crate::actions::video_records::imports) fn canvas_frame_output_path(
    data_dir: &Path,
    project_id: i64,
    video_id: i64,
    frame_kind: &str,
) -> Result<PathBuf, String> {
    let dir = ProjectPaths::new(data_dir, project_id).images_dir();
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建画布抽帧图片目录：{err}"))?;
    let stamp = Utc::now().timestamp_millis();
    Ok(dir.join(format!(
        "canvas_frame_video_{video_id}_{frame_kind}_{stamp}.jpg"
    )))
}

pub(in crate::actions::video_records::imports) fn run_ffmpeg_extract_frame(
    video_path: &Path,
    output: &Path,
    at_secs: f32,
) -> Result<(), String> {
    a4n_media::extract_video_frame(video_path, output, at_secs).map_err(|error| match error {
        a4n_media::FfmpegCommandError::Unavailable { source } => {
            format!("画布抽帧失败：未能启动 FFmpeg：{source}")
        }
        a4n_media::FfmpegCommandError::Failed { stderr } => {
            format!("画布抽帧失败：{stderr}")
        }
    })
}
