use super::contracts::VideoPlaybackData;
use super::records::required_generated_video;
use crate::AppState;
use crate::actions::access;
use crate::actions::video_generation::media::video_mime_from_path;
use crate::file_utils::canonicalize_data_file_path;
use base64::Engine as _;

pub(in crate::actions::video_records) const MAX_INLINE_PLAYBACK_VIDEO_BYTES: u64 =
    256 * 1024 * 1024;

pub async fn load_video_playback_data(
    state: &AppState,
    user_id: i64,
    video_id: i64,
) -> Result<VideoPlaybackData, String> {
    let video = required_generated_video(state, video_id).await?;
    access::verify_project_editor_string(state, user_id, video.project_id).await?;
    let file_path = video
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| format!("视频 #{} 没有本地文件", video.id))?;
    let path = canonicalize_data_file_path(
        &state.config.data_dir,
        file_path,
        &format!("视频 #{}", video.id),
    )?;
    let metadata =
        std::fs::metadata(&path).map_err(|error| format!("无法读取视频文件元数据：{error}"))?;
    if metadata.len() > MAX_INLINE_PLAYBACK_VIDEO_BYTES {
        return Err(format!(
            "视频文件超过 {}MB，无法以内置播放器 Blob 方式加载",
            MAX_INLINE_PLAYBACK_VIDEO_BYTES / 1024 / 1024
        ));
    }
    let bytes = std::fs::read(&path).map_err(|error| format!("无法读取视频文件：{error}"))?;
    Ok(VideoPlaybackData {
        mime_type: video_mime_from_path(&path)?.to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}
