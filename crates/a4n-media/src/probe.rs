use std::{
    io,
    path::Path,
    process::{Command, Output},
};

use serde_json::Value;
use thiserror::Error;

const STDERR_PREVIEW_CHARS: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VideoProbeProperties {
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
}

#[derive(Debug, Error)]
pub enum FfprobeCommandError {
    #[error("failed to start ffprobe: {source}")]
    Unavailable {
        #[source]
        source: io::Error,
    },
    #[error("ffprobe command failed: {stderr}")]
    Failed { stderr: String },
}

#[derive(Debug, Error)]
pub enum VideoProbeError {
    #[error(transparent)]
    Command(#[from] FfprobeCommandError),
    #[error("ffprobe output is not valid JSON: {source}")]
    InvalidJson {
        #[source]
        source: serde_json::Error,
    },
    #[error("ffprobe output does not contain a video stream")]
    MissingVideoStream,
    #[error("ffprobe output does not contain a valid video width")]
    InvalidWidth,
    #[error("ffprobe output does not contain a valid video height")]
    InvalidHeight,
    #[error("ffprobe output does not contain a valid video duration")]
    InvalidDuration,
}

pub fn probe_video_properties(path: &Path) -> Result<VideoProbeProperties, VideoProbeError> {
    let output = run_ffprobe(
        &[
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,r_frame_rate",
            "-show_entries",
            "format=duration",
            "-of",
            "json",
        ],
        path,
    )?;
    parse_video_probe_properties(&output.stdout)
}

pub fn probe_video_has_audio(path: &Path) -> Result<bool, FfprobeCommandError> {
    let output = run_ffprobe(
        &[
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=index",
            "-of",
            "csv=p=0",
        ],
        path,
    )?;
    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

pub fn parse_video_probe_properties(
    stdout: &[u8],
) -> Result<VideoProbeProperties, VideoProbeError> {
    let value: Value =
        serde_json::from_slice(stdout).map_err(|source| VideoProbeError::InvalidJson { source })?;
    let stream = value
        .get("streams")
        .and_then(Value::as_array)
        .and_then(|streams| streams.first())
        .ok_or(VideoProbeError::MissingVideoStream)?;
    let width = stream
        .get("width")
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or(VideoProbeError::InvalidWidth)?;
    let height = stream
        .get("height")
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or(VideoProbeError::InvalidHeight)?;
    let duration_secs = value
        .get("format")
        .and_then(|format| format.get("duration"))
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or(VideoProbeError::InvalidDuration)?;
    let fps = stream
        .get("r_frame_rate")
        .and_then(Value::as_str)
        .and_then(parse_frame_rate)
        .unwrap_or(24);

    Ok(VideoProbeProperties {
        width,
        height,
        duration_secs,
        fps,
    })
}

pub fn parse_frame_rate(value: &str) -> Option<i32> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let fps = if let Some((left, right)) = value.split_once('/') {
        let numerator = left.trim().parse::<f64>().ok()?;
        let denominator = right.trim().parse::<f64>().ok()?;
        if denominator <= 0.0 {
            return None;
        }
        numerator / denominator
    } else {
        value.parse::<f64>().ok()?
    };
    let rounded = fps.round();
    if !rounded.is_finite() || rounded <= 0.0 || rounded > f64::from(i32::MAX) {
        return None;
    }
    Some(rounded as i32)
}

fn run_ffprobe(args: &[&str], path: &Path) -> Result<Output, FfprobeCommandError> {
    let output = Command::new("ffprobe")
        .args(args)
        .arg(path)
        .output()
        .map_err(|source| FfprobeCommandError::Unavailable { source })?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(FfprobeCommandError::Failed {
            stderr: String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(STDERR_PREVIEW_CHARS)
                .collect(),
        })
    }
}
