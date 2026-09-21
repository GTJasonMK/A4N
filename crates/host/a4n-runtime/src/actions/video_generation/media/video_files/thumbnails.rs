use crate::AppState;
use a4n_media_contract::{MediaExecuteInput, MediaExecuteOutput};
#[cfg(test)]
use a4n_storage::remove_created_file_on_error_sync;
use a4n_storage::{data_dir_relative_file_path, remove_created_file_on_error};
use std::path::{Path, PathBuf};

fn video_thumbnail_path(video_id: i64, video_path: &Path) -> Result<PathBuf, String> {
    let dir = video_path
        .parent()
        .ok_or_else(|| format!("视频路径缺少父目录：{}", video_path.display()))?;
    Ok(dir.join(format!("thumb_{video_id}.jpg")))
}

async fn generate_video_thumbnail(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
    video_path: &Path,
    output: &Path,
) -> Result<(), String> {
    let input = state.data_media_file_ref(video_path, "缩略图源视频")?;
    let output_ref = state.data_media_file_ref(output, "缩略图输出")?;
    match state
        .execute_media(MediaExecuteInput::GenerateVideoThumbnail {
            input,
            output: output_ref,
        })
        .await?
    {
        MediaExecuteOutput::Unit => Ok(()),
        output => Err(format!(
            "生成视频缩略图失败 project={project_id} chapter={chapter_id} video={video_id}：返回了不匹配的输出 {output:?}"
        )),
    }
}

pub(crate) async fn best_effort_video_thumbnail_path(
    state: &AppState,
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
    match remove_created_file_on_error(&output, "已写出的视频缩略图", async {
        generate_video_thumbnail(state, project_id, chapter_id, video_id, video_path, &output)
            .await?;
        data_dir_relative_file_path(state.data_dir(), &output, "视频缩略图")
    })
    .await
    {
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

#[cfg(test)]
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
