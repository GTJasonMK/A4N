use a4n_storage::{data_dir_relative_file_path, remove_created_file_on_error_sync};
use std::path::{Path, PathBuf};

fn video_thumbnail_path(video_id: i64, video_path: &Path) -> Result<PathBuf, String> {
    let dir = video_path
        .parent()
        .ok_or_else(|| format!("视频路径缺少父目录：{}", video_path.display()))?;
    Ok(dir.join(format!("thumb_{video_id}.jpg")))
}

fn generate_video_thumbnail(
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
    video_path: &Path,
    output: &Path,
) -> Result<(), String> {
    a4n_media::generate_video_thumbnail(video_path, output).map_err(|error| match error {
        a4n_media::FfmpegCommandError::Unavailable { source } => {
            format!("生成视频缩略图失败：未能启动 FFmpeg：{source}")
        }
        a4n_media::FfmpegCommandError::Failed { stderr } => format!(
            "生成视频缩略图失败 project={project_id} chapter={chapter_id} video={video_id}：{stderr}"
        ),
    })
}

pub(crate) fn best_effort_video_thumbnail_path(
    data_dir: &Path,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
    video_path: &Path,
) -> Option<String> {
    let output = match video_thumbnail_path(video_id, video_path) {
        Ok(output) => output,
        Err(err) => {
            tracing::warn!(
                %err,
                project_id,
                chapter_id,
                video_id,
                "生成视频缩略图失败，继续保留视频结果"
            );
            return None;
        }
    };
    match persist_video_thumbnail_path(data_dir, &output, || {
        generate_video_thumbnail(project_id, chapter_id, video_id, video_path, &output)
    }) {
        Ok(path) => Some(path),
        Err(err) => {
            tracing::warn!(
                %err,
                project_id,
                chapter_id,
                video_id,
                "生成或持久化视频缩略图失败，继续保留视频结果"
            );
            None
        }
    }
}

pub(in crate::actions::video_generation::media) fn persist_video_thumbnail_path<F>(
    data_dir: &Path,
    output: &Path,
    generate: F,
) -> Result<String, String>
where
    F: FnOnce() -> Result<(), String>,
{
    remove_created_file_on_error_sync(output, "已写出的视频缩略图", || {
        generate()?;
        data_dir_relative_file_path(data_dir, output, "视频缩略图")
    })
}
