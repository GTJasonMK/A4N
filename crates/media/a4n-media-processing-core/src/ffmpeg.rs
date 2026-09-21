use std::{
    io,
    path::Path,
    process::{Command, Output},
};

use a4n_domain::VIDEO_TAIL_FRAME_OFFSET_SECS;
use thiserror::Error;

const STDERR_PREVIEW_CHARS: usize = 500;

#[derive(Debug, Error)]
pub enum FfmpegCommandError {
    #[error("failed to start ffmpeg: {source}")]
    Unavailable {
        #[source]
        source: io::Error,
    },
    #[error("ffmpeg command failed: {stderr}")]
    Failed { stderr: String },
}

#[derive(Debug, Error)]
pub enum FfmpegFrameError {
    #[error("video frame timestamp is invalid")]
    InvalidTimestamp,
    #[error(transparent)]
    Command(#[from] FfmpegCommandError),
}

#[derive(Debug, Error)]
pub enum FfmpegTrimError {
    #[error("video trim range is invalid")]
    InvalidTimeRange,
    #[error("failed to start ffmpeg video trim: {source}")]
    Unavailable {
        #[source]
        source: io::Error,
    },
    #[error("ffmpeg video trim failed: {stderr}")]
    Failed { stderr: String },
}

pub fn generate_video_thumbnail(input: &Path, output: &Path) -> Result<(), FfmpegCommandError> {
    let command_output = Command::new("ffmpeg")
        .arg("-y")
        .arg("-ss")
        .arg("0.8")
        .arg("-i")
        .arg(input)
        .arg("-frames:v")
        .arg("1")
        .arg("-vf")
        .arg("scale=640:-2")
        .arg("-q:v")
        .arg("3")
        .arg("-update")
        .arg("1")
        .arg(output)
        .output()
        .map_err(|source| FfmpegCommandError::Unavailable { source })?;
    require_success(command_output)
}

pub fn extract_video_tail_frame(input: &Path, output: &Path) -> Result<(), FfmpegCommandError> {
    let command_output = Command::new("ffmpeg")
        .arg("-y")
        .arg("-sseof")
        .arg(format!("-{VIDEO_TAIL_FRAME_OFFSET_SECS:.3}"))
        .arg("-i")
        .arg(input)
        .arg("-frames:v")
        .arg("1")
        .arg("-q:v")
        .arg("2")
        .arg("-update")
        .arg("1")
        .arg(output)
        .output()
        .map_err(|source| FfmpegCommandError::Unavailable { source })?;
    require_success(command_output)
}

pub fn extract_video_frame(
    input: &Path,
    output: &Path,
    at_secs: f32,
) -> Result<(), FfmpegFrameError> {
    if !at_secs.is_finite() || at_secs < 0.0 {
        return Err(FfmpegFrameError::InvalidTimestamp);
    }
    let command_output = Command::new("ffmpeg")
        .arg("-y")
        .arg("-ss")
        .arg(seconds_literal(at_secs))
        .arg("-i")
        .arg(input)
        .arg("-frames:v")
        .arg("1")
        .arg("-q:v")
        .arg("2")
        .arg("-update")
        .arg("1")
        .arg(output)
        .output()
        .map_err(|source| FfmpegCommandError::Unavailable { source })?;
    require_success(command_output)?;
    Ok(())
}

pub fn trim_video_segment(
    input: &Path,
    output: &Path,
    start_secs: f32,
    end_secs: f32,
) -> Result<(), FfmpegTrimError> {
    if !start_secs.is_finite()
        || !end_secs.is_finite()
        || start_secs < 0.0
        || end_secs <= start_secs
    {
        return Err(FfmpegTrimError::InvalidTimeRange);
    }
    let start = seconds_literal(start_secs);
    let end = seconds_literal(end_secs);
    let output = Command::new("ffmpeg")
        .args(["-y", "-ss", &start, "-to", &end, "-i"])
        .arg(input)
        .args(["-c:v", "libx264", "-c:a", "aac"])
        .arg(output)
        .output()
        .map_err(|source| FfmpegTrimError::Unavailable { source })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(FfmpegTrimError::Failed {
            stderr: stderr_preview(&output, STDERR_PREVIEW_CHARS),
        })
    }
}

fn require_success(output: Output) -> Result<(), FfmpegCommandError> {
    if output.status.success() {
        Ok(())
    } else {
        Err(FfmpegCommandError::Failed {
            stderr: stderr_preview(&output, STDERR_PREVIEW_CHARS),
        })
    }
}

pub(crate) fn stderr_preview(output: &Output, limit: usize) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let chars = stderr.chars().collect::<Vec<_>>();
    let start = chars.len().saturating_sub(limit);
    chars[start..].iter().collect()
}

pub(crate) fn seconds_literal(value: f32) -> String {
    format!("{value:.3}")
}
