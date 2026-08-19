use chrono::{DateTime, Duration, Utc};
use std::path::{Path, PathBuf};

use super::super::super::state::ServerState;
use super::config::temp_media_ttl_seconds;
use super::rules::TEMP_MEDIA_ARTIFACT_EXTENSIONS;

pub(super) const TEMP_MEDIA_RELATIVE_DIR: &str = "temp-media/video-inputs";

pub(super) fn temp_media_dir(state: &ServerState) -> PathBuf {
    state
        .runtime()
        .data_dir()
        .join(TEMP_MEDIA_RELATIVE_DIR)
}

pub(super) fn find_temp_media_path(state: &ServerState, artifact_id: &str) -> Option<PathBuf> {
    TEMP_MEDIA_ARTIFACT_EXTENSIONS
        .iter()
        .copied()
        .map(|extension| temp_media_dir(state).join(format!("{artifact_id}.{extension}")))
        .find(|path| path.is_file())
}

pub(super) fn temp_media_file_expired(path: &Path) -> Result<Option<bool>, String> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("无法读取临时媒体信息：{err}")),
    };
    let modified = metadata
        .modified()
        .map_err(|err| format!("无法读取临时媒体修改时间：{err}"))?;
    let modified: DateTime<Utc> = modified.into();
    Ok(Some(
        Utc::now() > modified + Duration::seconds(temp_media_ttl_seconds()?),
    ))
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
        match temp_media_file_expired(&path) {
            Ok(Some(true)) => {
                if let Err(error) = std::fs::remove_file(&path) {
                    tracing::warn!(path = %path.display(), %error, "清理过期临时媒体失败");
                }
            }
            Ok(Some(false) | None) => {}
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "检查临时媒体过期状态失败")
            }
        }
    }
}
