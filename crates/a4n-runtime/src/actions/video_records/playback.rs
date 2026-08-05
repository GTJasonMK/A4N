use super::contracts::VideoPlaybackData;
use super::records::required_generated_video;
use crate::AppState;
use crate::actions::access;
use crate::actions::video_generation::media::video_mime_from_path;
use crate::file_utils::canonicalize_data_file_path;
use base64::Engine as _;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub(in crate::actions::video_records) const MAX_INLINE_PLAYBACK_VIDEO_BYTES: u64 =
    256 * 1024 * 1024;
// Bump when the codec/GOP contract changes so existing proxies are rebuilt.
const SCRUB_PROXY_CACHE_VERSION: u8 = 5;

pub async fn load_video_playback_data(
    state: &AppState,
    user_id: i64,
    video_id: i64,
) -> Result<VideoPlaybackData, String> {
    let path = authorized_video_path(state, user_id, video_id).await?;
    read_video_playback_data(&path)
}

pub async fn load_video_scrub_playback_data(
    state: &AppState,
    user_id: i64,
    video_id: i64,
) -> Result<VideoPlaybackData, String> {
    let source_path = authorized_video_path(state, user_id, video_id).await?;
    let proxy_path = scrub_proxy_path(state.temp_dir(), video_id, &source_path)?;
    if !is_valid_scrub_proxy(&proxy_path) {
        remove_scrub_temp_file(&proxy_path);
        generate_scrub_proxy(&source_path, &proxy_path).await?;
    }
    remove_stale_scrub_proxies(&proxy_path, video_id);
    read_video_playback_data(&proxy_path)
}

async fn authorized_video_path(
    state: &AppState,
    user_id: i64,
    video_id: i64,
) -> Result<PathBuf, String> {
    let video = required_generated_video(&state.video(), video_id).await?;
    access::verify_project_editor_string(state, user_id, video.project_id).await?;
    let file_path = video
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| format!("视频 #{} 没有本地文件", video.id))?;
    canonicalize_data_file_path(state.data_dir(), file_path, &format!("视频 #{}", video.id))
}

fn read_video_playback_data(path: &Path) -> Result<VideoPlaybackData, String> {
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

fn scrub_proxy_path(temp_dir: &Path, video_id: i64, source_path: &Path) -> Result<PathBuf, String> {
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
    Ok(cache_dir.join(format!(
        "video-{video_id}-v{SCRUB_PROXY_CACHE_VERSION}-{}-{modified_nanos}.mp4",
        metadata.len()
    )))
}

async fn generate_scrub_proxy(source_path: &Path, proxy_path: &Path) -> Result<(), String> {
    let temp_path =
        proxy_path.with_file_name(format!(".video-scrub-{}.tmp.mp4", uuid::Uuid::new_v4()));
    let source_path = source_path.to_path_buf();
    let render_path = temp_path.clone();
    let generation = tokio::task::spawn_blocking(move || {
        a4n_media::generate_video_scrub_proxy(&source_path, &render_path)
    })
    .await;
    let generation = match generation {
        Ok(generation) => generation,
        Err(error) => {
            remove_scrub_temp_file(&temp_path);
            return Err(format!("剪辑预览代理生成任务异常终止：{error}"));
        }
    };
    if let Err(error) = generation {
        remove_scrub_temp_file(&temp_path);
        return Err(format!("无法生成剪辑预览代理：{error}"));
    }
    if !is_non_empty_file(&temp_path) {
        remove_scrub_temp_file(&temp_path);
        return Err("剪辑预览代理生成完成但文件为空".to_string());
    }
    if is_non_empty_file(proxy_path) {
        remove_scrub_temp_file(&temp_path);
        return Ok(());
    }
    if proxy_path.exists()
        && let Err(error) = std::fs::remove_file(proxy_path)
    {
        remove_scrub_temp_file(&temp_path);
        return Err(format!("无法清理无效的剪辑预览代理：{error}"));
    }
    if let Err(error) = std::fs::rename(&temp_path, proxy_path) {
        remove_scrub_temp_file(&temp_path);
        return Err(format!("无法发布剪辑预览代理：{error}"));
    }
    Ok(())
}

fn is_non_empty_file(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.len() > 0)
        .unwrap_or(false)
}

fn is_valid_scrub_proxy(path: &Path) -> bool {
    is_non_empty_file(path) && a4n_media::probe_video_properties(path).is_ok()
}

fn remove_scrub_temp_file(path: &Path) {
    if let Err(error) = std::fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(path = %path.display(), %error, "failed to remove temporary video scrub proxy");
    }
}

fn remove_stale_scrub_proxies(current_path: &Path, video_id: i64) {
    let Some(cache_dir) = current_path.parent() else {
        return;
    };
    let prefix = format!("video-{video_id}-");
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
        let is_stale_proxy = path != current_path
            && path.is_file()
            && path.extension().and_then(|value| value.to_str()) == Some("mp4")
            && path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.starts_with(&prefix));
        if is_stale_proxy {
            if let Err(error) = std::fs::remove_file(&path) {
                tracing::warn!(path = %path.display(), %error, "failed to remove stale video scrub proxy");
            }
        }
    }
}
