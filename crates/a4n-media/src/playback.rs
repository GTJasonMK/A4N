use std::{io, path::Path, process::Command};

use thiserror::Error;

use crate::ffmpeg::stderr_preview;

const SCRUB_PROXY_STDERR_PREVIEW_CHARS: usize = 800;

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

/// Produces a high-quality H.264 editing source with frequent keyframes for random seeking.
/// Resolution and audio are retained so scrubbing does not visibly degrade the preview.
pub fn generate_video_scrub_proxy(
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
        .arg("-map")
        .arg("0:a:0?")
        .arg("-c:v")
        .arg("libx264")
        // WebKitGTK can complete a seek while its GStreamer decoder is still
        // draining a long GOP. During rapid reverse scrubbing that leaves the
        // native video layer stuck on an earlier frame. Keep the GOP to three
        // frames so each seek has at most two frames to decode, while retaining
        // the source resolution and frame rate.
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
        .arg("3")
        .arg("-keyint_min")
        .arg("3")
        .arg("-sc_threshold")
        .arg("0")
        .arg("-c:a")
        .arg("aac")
        .arg("-b:a")
        .arg("128k")
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
