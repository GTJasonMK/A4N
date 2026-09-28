//! Wire contract for local video preview processing.

use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

pub const MEDIA_SERVICE_NAME: &str = "media";
pub const MEDIA_FFMPEG_CAPABILITY_VERSION: &str = "media-ffmpeg-v1";
pub const MEDIA_SERVICE_ADDR_ENV: &str = "A4N_MEDIA_SERVICE_ADDR";
pub const MEDIA_SERVICE_DEFAULT_ADDR: &str = "127.0.0.1:47649";
pub const VIDEO_SCRUB_FRAMES_PER_SECOND: u32 = 12;

/// Probe data shared by the Host and the media implementation.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoProbeProperties {
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
    pub codec_name: Option<String>,
    pub profile: Option<String>,
    pub has_b_frames: Option<i32>,
}

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
    ProbeVideoProperties {
        input: MediaFileRef,
    },
    ProbeVideoHasAudio {
        input: MediaFileRef,
    },
    ProbeAudioDuration {
        input: MediaFileRef,
    },
    GenerateVideoScrubProxy {
        input: MediaFileRef,
        output: MediaFileRef,
        include_audio: bool,
    },
    GenerateVideoScrubFrames {
        input: MediaFileRef,
        output: MediaFileRef,
    },
    ExtractVideoFrame {
        input: MediaFileRef,
        output: MediaFileRef,
        at_secs: f32,
    },
    ExtractVideoTailFrame {
        input: MediaFileRef,
        output: MediaFileRef,
    },
    GenerateVideoThumbnail {
        input: MediaFileRef,
        output: MediaFileRef,
    },
    TrimVideoSegment {
        input: MediaFileRef,
        output: MediaFileRef,
        start_secs: f32,
        end_secs: f32,
    },
    AssembleVideos {
        clips: Vec<MediaAssemblyClip>,
        output: MediaFileRef,
        width: i32,
        height: i32,
        fps: i32,
        audio_enabled: bool,
    },
    PostprocessVideo {
        base: MediaFileRef,
        source: Option<MediaFileRef>,
        output: MediaFileRef,
        operation: MediaPostprocessOperation,
        start_secs: f32,
        end_secs: f32,
        base_duration_secs: f32,
        base_has_audio: bool,
        source_duration_secs: Option<f32>,
        source_has_audio: bool,
        width: i32,
        height: i32,
        fps: i32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MediaAssemblyClip {
    pub input: MediaFileRef,
    pub duration_secs: f32,
    pub has_audio: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaPostprocessOperation {
    Insert,
    Delete,
    Replace,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MediaExecuteOutput {
    Unit,
    VideoProbeProperties {
        width: i32,
        height: i32,
        duration_secs: f32,
        fps: i32,
        codec_name: Option<String>,
        profile: Option<String>,
        has_b_frames: Option<i32>,
    },
    HasAudio {
        has_audio: bool,
    },
    AudioDuration {
        duration_secs: f32,
    },
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
