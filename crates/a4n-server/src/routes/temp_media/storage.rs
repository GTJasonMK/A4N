use a4n_runtime::host_api::storage_paths;
use chrono::{DateTime, Duration, Utc};
use std::path::{Path, PathBuf};

use super::super::super::state::ServerState;
use super::config::temp_media_ttl_minutes;

const TEMP_MEDIA_DIR: &str = "temp-media/video-inputs";

pub(super) fn temp_media_dir(state: &ServerState) -> PathBuf {
    storage_paths::data_dir_path(state.runtime().data_dir()).join(TEMP_MEDIA_DIR)
}

pub(super) fn find_temp_media_path(state: &ServerState, artifact_id: &str) -> Option<PathBuf> {
    [
        "png", "jpg", "webp", "mp4", "mov", "webm", "mp3", "wav", "m4a", "ogg",
    ]
    .into_iter()
    .map(|extension| temp_media_dir(state).join(format!("{artifact_id}.{extension}")))
    .find(|path| path.is_file())
}

pub(super) fn is_expired_file(path: &Path) -> Result<bool, String> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(err) => return Err(format!("无法读取临时媒体信息：{err}")),
    };
    let modified = metadata
        .modified()
        .map_err(|err| format!("无法读取临时媒体修改时间：{err}"))?;
    let modified: DateTime<Utc> = modified.into();
    Ok(Utc::now() > modified + Duration::minutes(temp_media_ttl_minutes()?))
}

pub(super) fn cleanup_expired_temp_media(state: &ServerState) {
    let dir = temp_media_dir(state);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            tracing::warn!(path = %dir.display(), %error, "读取临时媒体清理目录失败");
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tracing::warn!(path = %dir.display(), %error, "读取临时媒体目录项失败");
                continue;
            }
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        match is_expired_file(&path) {
            Ok(true) => {
                if let Err(error) = std::fs::remove_file(&path) {
                    tracing::warn!(path = %path.display(), %error, "清理过期临时媒体失败");
                }
            }
            Ok(false) => {}
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "检查临时媒体过期状态失败")
            }
        }
    }
}
