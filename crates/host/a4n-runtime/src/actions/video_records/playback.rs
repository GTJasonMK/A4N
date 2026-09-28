use super::VideoPlaybackData;
use crate::AppState;
use a4n_api_contract::video::VideoScrubFramesData;
use a4n_domain::media::video_mime_from_path;
use a4n_storage::canonicalize_data_file_path;
use base64::Engine as _;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub(in crate::actions::video_records) const MAX_INLINE_PLAYBACK_VIDEO_BYTES: u64 =
    256 * 1024 * 1024;
// Bump when the codec/GOP contract changes so existing proxies are rebuilt.
const SCRUB_PROXY_CACHE_VERSION: u8 = 6;

pub async fn load_video_playback_data(
    state: &AppState,
    user_id: i64,
    video_id: i64,
) -> Result<VideoPlaybackData, String> {
    let path = authorized_video_path(state, user_id, video_id).await?;
    read_video_playback_data(&path, None)
}

pub async fn load_video_scrub_playback_data(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    include_audio: bool,
) -> Result<VideoPlaybackData, String> {
    let source_path = authorized_video_path(state, user_id, video_id).await?;
    let (proxy_path, frames_path, current_prefix) =
        scrub_cache_paths(state.temp_dir(), video_id, &source_path, include_audio)?;
    if !is_valid_scrub_proxy(state, &proxy_path).await {
        remove_scrub_temp_file(&proxy_path);
        generate_scrub_proxy(state, &source_path, &proxy_path, include_audio).await?;
    }
    if !include_audio && !is_non_empty_file(&frames_path) {
        remove_scrub_temp_file(&frames_path);
        generate_scrub_frames(state, &source_path, &frames_path).await?;
    }
    remove_stale_scrub_assets(&proxy_path, video_id, &current_prefix);
    read_video_playback_data(
        &proxy_path,
        (!include_audio).then_some(frames_path.as_path()),
    )
}

async fn authorized_video_path(
    state: &AppState,
    user_id: i64,
    video_id: i64,
) -> Result<PathBuf, String> {
    let video = state
        .video_record_read_application()
        .playback_source(user_id, video_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    let file_path = video
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| format!("视频 #{} 没有本地文件", video.id))?;
    canonicalize_data_file_path(state.data_dir(), file_path, &format!("视频 #{}", video.id))
}

fn read_video_playback_data(
    path: &Path,
    scrub_frames_path: Option<&Path>,
) -> Result<VideoPlaybackData, String> {
    let bytes = read_inline_file(path, "视频文件")?;
    let scrub_frames = scrub_frames_path
        .map(|path| {
            read_inline_file(path, "剪辑拖动帧数据").map(|bytes| VideoScrubFramesData {
                data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
                frames_per_second: a4n_media_contract::VIDEO_SCRUB_FRAMES_PER_SECOND,
                mime_type: "image/jpeg".to_string(),
            })
        })
        .transpose()?;
    Ok(VideoPlaybackData {
        mime_type: video_mime_from_path(path)?.to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        scrub_frames,
    })
}

fn read_inline_file(path: &Path, label: &str) -> Result<Vec<u8>, String> {
    let metadata =
        std::fs::metadata(path).map_err(|error| format!("无法读取{label}元数据：{error}"))?;
    if metadata.len() > MAX_INLINE_PLAYBACK_VIDEO_BYTES {
        return Err(format!(
            "{label}超过 {}MB，无法以内置播放器 Blob 方式加载",
            MAX_INLINE_PLAYBACK_VIDEO_BYTES / 1024 / 1024
        ));
    }
    std::fs::read(path).map_err(|error| format!("无法读取{label}：{error}"))
}

fn scrub_cache_paths(
    temp_dir: &Path,
    video_id: i64,
    source_path: &Path,
    include_audio: bool,
) -> Result<(PathBuf, PathBuf, String), String> {
    let metadata = std::fs::metadata(source_path)
        .map_err(|error| format!("无法读取剪辑源视频元数据：{error}"))?;
    let modified_nanos = metadata
        .modified()
        .map_err(|error| format!("无法读取剪辑源视频修改时间：{error}"))?
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("剪辑源视频修改时间无效：{error}"))?
        .as_nanos();
    let cache_dir = temp_dir.join("video-scrub-proxies");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|error| format!("无法创建剪辑预览缓存目录：{error}"))?;
    let prefix = format!(
        "video-{video_id}-v{SCRUB_PROXY_CACHE_VERSION}-{}-{modified_nanos}",
        metadata.len()
    );
    let mode = if include_audio { "audio" } else { "silent" };
    Ok((
        cache_dir.join(format!("{prefix}-{mode}.mp4")),
        cache_dir.join(format!("{prefix}-frames.mjpeg")),
        prefix,
    ))
}

async fn generate_scrub_proxy(
    state: &AppState,
    source_path: &Path,
    proxy_path: &Path,
    include_audio: bool,
) -> Result<(), String> {
    let temp_path =
        proxy_path.with_file_name(format!(".video-scrub-{}.tmp.mp4", uuid::Uuid::new_v4()));
    if let Err(error) = state
        .render_media_scrub_proxy(source_path, &temp_path, include_audio)
        .await
    {
        remove_scrub_temp_file(&temp_path);
        return Err(format!("无法生成剪辑预览代理：{error}"));
    }
    publish_scrub_asset(&temp_path, proxy_path, "剪辑预览代理")
}

async fn generate_scrub_frames(
    state: &AppState,
    source_path: &Path,
    frames_path: &Path,
) -> Result<(), String> {
    let temp_path =
        frames_path.with_file_name(format!(".video-scrub-{}.tmp.mjpeg", uuid::Uuid::new_v4()));
    if let Err(error) = state
        .render_media_scrub_frames(source_path, &temp_path)
        .await
    {
        remove_scrub_temp_file(&temp_path);
        return Err(format!("无法生成剪辑拖动帧：{error}"));
    }
    publish_scrub_asset(&temp_path, frames_path, "剪辑拖动帧")
}

fn publish_scrub_asset(temp_path: &Path, target_path: &Path, label: &str) -> Result<(), String> {
    if !is_non_empty_file(temp_path) {
        remove_scrub_temp_file(temp_path);
        return Err(format!("{label}生成完成但文件为空"));
    }
    if is_non_empty_file(target_path) {
        remove_scrub_temp_file(temp_path);
        return Ok(());
    }
    if target_path.exists()
        && let Err(error) = std::fs::remove_file(target_path)
    {
        remove_scrub_temp_file(temp_path);
        return Err(format!("无法清理无效的{label}：{error}"));
    }
    if let Err(error) = std::fs::rename(temp_path, target_path) {
        remove_scrub_temp_file(temp_path);
        return Err(format!("无法发布{label}：{error}"));
    }
    Ok(())
}

fn is_non_empty_file(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.len() > 0)
        .unwrap_or(false)
}

async fn is_valid_scrub_proxy(state: &AppState, path: &Path) -> bool {
    is_non_empty_file(path)
        && state
            .probe_media_video_properties(path, a4n_media_contract::MediaRoot::Temp, "剪辑预览代理")
            .await
            .is_ok()
}

fn remove_scrub_temp_file(path: &Path) {
    if let Err(error) = std::fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(path = %path.display(), %error, "failed to remove temporary video scrub proxy");
    }
}

fn remove_stale_scrub_assets(current_path: &Path, video_id: i64, current_prefix: &str) {
    let Some(cache_dir) = current_path.parent() else {
        return;
    };
    let video_prefix = format!("video-{video_id}-");
    let current_prefix = format!("{current_prefix}-");
    let entries = match std::fs::read_dir(cache_dir) {
        Ok(entries) => entries,
        Err(error) => {
            tracing::warn!(path = %cache_dir.display(), %error, "failed to inspect video scrub proxy cache");
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tracing::warn!(path = %cache_dir.display(), %error, "failed to inspect video scrub proxy cache entry");
                continue;
            }
        };
        let path = entry.path();
        let is_stale_asset = path != current_path
            && path.is_file()
            && path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| {
                    name.starts_with(&video_prefix) && !name.starts_with(&current_prefix)
                });
        if is_stale_asset && let Err(error) = std::fs::remove_file(&path) {
            tracing::warn!(path = %path.display(), %error, "failed to remove stale video scrub asset");
        }
    }
}
