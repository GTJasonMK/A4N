use std::{io, path::Path, process::Command};

use a4n_media_contract::{MediaExecuteInput, MediaExecuteOutput};
use thiserror::Error;

use crate::MediaRoots;

const SCRUB_PROXY_STDERR_PREVIEW_CHARS: usize = 800;
pub const VIDEO_SCRUB_FRAMES_PER_SECOND: u32 = 12;
const STDERR_PREVIEW_CHARS: usize = 800;

#[derive(Debug, Error)]
pub enum VideoScrubProxyError {
    #[error("video scrub proxy source is not a file")]
    SourceNotFile,
    #[error("failed to start ffmpeg video scrub proxy generation: {source}")]
    Unavailable {
        #[source]
        source: io::Error,
    },
    #[error("ffmpeg video scrub proxy generation failed: {stderr}")]
    Failed { stderr: String },
}

pub fn execute(roots: &MediaRoots, input: MediaExecuteInput) -> Result<MediaExecuteOutput, String> {
    match input {
        MediaExecuteInput::GenerateVideoScrubProxy {
            input,
            output,
            include_audio,
        } => {
            let input = roots.input(&input)?;
            let output = roots.output(&output)?;
            generate_video_scrub_proxy(&input, &output, include_audio)
                .map_err(|error| error.to_string())?;
        }
        MediaExecuteInput::GenerateVideoScrubFrames { input, output } => {
            let input = roots.input(&input)?;
            let output = roots.output(&output)?;
            generate_video_scrub_frames(&input, &output).map_err(|error| error.to_string())?;
        }
    }
    Ok(MediaExecuteOutput::Unit)
}

/// Produces a high-quality H.264 editing source with frequent keyframes for random seeking.
pub fn generate_video_scrub_proxy(
    input_path: &Path,
    output_path: &Path,
    include_audio: bool,
) -> Result<(), VideoScrubProxyError> {
    if !input_path.is_file() {
        return Err(VideoScrubProxyError::SourceNotFile);
    }
    let mut command = Command::new("ffmpeg");
    command
        .arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-i")
        .arg(input_path)
        .arg("-map")
        .arg("0:v:0")
        .arg("-c:v")
        .arg("libx264")
        // WebKitGTK/GStreamer can retain the previous reference frame after
        // rapid reverse seeks. Scrub proxies are temporary editing media, so
        // encode every frame independently to make each seek deterministic.
        .arg("-profile:v")
        .arg("main")
        .arg("-bf")
        .arg("0")
        .arg("-refs")
        .arg("1")
        .arg("-preset")
        .arg("veryfast")
        .arg("-crf")
        .arg("16")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-g")
        .arg("1")
        .arg("-keyint_min")
        .arg("1")
        .arg("-sc_threshold")
        .arg("0");
    if include_audio {
        command
            .arg("-map")
            .arg("0:a:0?")
            .arg("-c:a")
            .arg("aac")
            .arg("-b:a")
            .arg("128k");
    } else {
        command.arg("-an");
    }
    let output = command
        .arg("-movflags")
        .arg("+faststart")
        .arg(output_path)
        .output()
        .map_err(|source| VideoScrubProxyError::Unavailable { source })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(VideoScrubProxyError::Failed {
            stderr: stderr_preview(&output, SCRUB_PROXY_STDERR_PREVIEW_CHARS),
        })
    }
}

fn stderr_preview(output: &std::process::Output, limit: usize) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let chars = stderr.chars().collect::<Vec<_>>();
    let start = chars.len().saturating_sub(limit.min(STDERR_PREVIEW_CHARS));
    chars[start..].iter().collect()
}

/// Produces a compact concatenated JPEG sequence for deterministic scrubbing.
pub fn generate_video_scrub_frames(
    input_path: &Path,
    output_path: &Path,
) -> Result<(), VideoScrubProxyError> {
    if !input_path.is_file() {
        return Err(VideoScrubProxyError::SourceNotFile);
    }
    let output = Command::new("ffmpeg")
        .arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-i")
        .arg(input_path)
        .arg("-map")
        .arg("0:v:0")
        .arg("-vf")
        .arg(format!("fps={VIDEO_SCRUB_FRAMES_PER_SECOND},scale=-2:270"))
        .arg("-c:v")
        .arg("mjpeg")
        .arg("-q:v")
        .arg("7")
        .arg("-an")
        .arg("-f")
        .arg("image2pipe")
        .arg(output_path)
        .output()
        .map_err(|source| VideoScrubProxyError::Unavailable { source })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(VideoScrubProxyError::Failed {
            stderr: stderr_preview(&output, SCRUB_PROXY_STDERR_PREVIEW_CHARS),
        })
    }
}
