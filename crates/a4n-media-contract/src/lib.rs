//! Wire contract for local video preview processing.

use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

pub const MEDIA_SERVICE_NAME: &str = "media";
pub const MEDIA_FFMPEG_CAPABILITY_VERSION: &str = "media-ffmpeg-v1";
pub const MEDIA_SERVICE_ADDR_ENV: &str = "A4N_MEDIA_SERVICE_ADDR";
pub const MEDIA_SERVICE_DEFAULT_ADDR: &str = "127.0.0.1:47649";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaRoot {
    Data,
    Temp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MediaFileRef {
    pub root: MediaRoot,
    pub relative_path: String,
}

impl MediaFileRef {
    pub fn new(root: MediaRoot, relative_path: impl Into<String>) -> Result<Self, String> {
        let file = Self {
            root,
            relative_path: relative_path.into(),
        };
        file.validate()?;
        Ok(file)
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_relative_media_path(&self.relative_path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MediaExecuteInput {
    GenerateVideoScrubProxy {
        input: MediaFileRef,
        output: MediaFileRef,
        include_audio: bool,
    },
    GenerateVideoScrubFrames {
        input: MediaFileRef,
        output: MediaFileRef,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaExecuteOutput {
    Unit,
}

pub fn validate_relative_media_path(path: &str) -> Result<(), String> {
    let raw = Path::new(path.trim());
    if path.trim().is_empty() || raw.is_absolute() || path.contains('\\') {
        return Err("媒体路径必须是数据或临时目录内的相对路径".to_string());
    }
    if raw.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_) | Component::CurDir
        )
    }) {
        return Err("媒体路径不能包含路径跳转".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MediaExecuteInput, MediaFileRef, MediaRoot, validate_relative_media_path};

    #[test]
    fn media_contract_rejects_unknown_operation_fields() {
        let error = serde_json::from_value::<MediaExecuteInput>(serde_json::json!({
            "kind": "generate_video_scrub_frames",
            "input": {"root": "data", "relative_path": "projects/1/video.mp4"},
            "output": {"root": "temp", "relative_path": "video.mjpeg"},
            "legacy_mode": true
        }))
        .expect_err("unknown media operation fields must fail");
        assert!(error.to_string().contains("unknown field `legacy_mode`"));
    }

    #[test]
    fn media_file_refs_reject_absolute_and_traversal_paths() {
        assert!(MediaFileRef::new(MediaRoot::Data, "projects/1/video.mp4").is_ok());
        for path in [
            "",
            "/tmp/video.mp4",
            "../video.mp4",
            "projects/../video.mp4",
            "projects\\video.mp4",
        ] {
            assert!(validate_relative_media_path(path).is_err(), "path={path}");
        }
    }
}
