use std::{io, path::Path, process::Command};

use thiserror::Error;

use crate::{assembly::ffmpeg_duration_literal, ffmpeg::stderr_preview};

const POSTPROCESS_STDERR_PREVIEW_CHARS: usize = 800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoPostprocessOperation {
    Insert,
    Delete,
    Replace,
}

#[derive(Debug)]
pub struct VideoPostprocessRequest<'a> {
    pub base_path: &'a Path,
    pub source_path: Option<&'a Path>,
    pub output_path: &'a Path,
    pub operation: VideoPostprocessOperation,
    pub start_secs: f32,
    pub end_secs: f32,
    pub base_duration_secs: f32,
    pub width: i32,
    pub height: i32,
    pub fps: i32,
}

#[derive(Debug, Error)]
pub enum VideoPostprocessError {
    #[error("insert or replace video postprocess requires a source video")]
    MissingSource,
    #[error("video postprocess source is not a file")]
    SourceNotFile,
    #[error("video postprocess result would be empty")]
    EmptyResult,
    #[error("failed to start ffmpeg video postprocess: {source}")]
    Unavailable {
        #[source]
        source: io::Error,
    },
    #[error("ffmpeg video postprocess failed: {stderr}")]
    Failed { stderr: String },
}

pub fn build_video_postprocess_filter(
    request: &VideoPostprocessRequest<'_>,
) -> Result<String, VideoPostprocessError> {
    let width = even_positive_dimension(request.width, 1280);
    let height = even_positive_dimension(request.height, 720);
    let fps = if request.fps > 0 { request.fps } else { 24 };
    let base_duration_secs = request.base_duration_secs.max(0.1);
    let start_secs = request.start_secs.clamp(0.0, base_duration_secs);
    let end_secs = request.end_secs.clamp(start_secs, base_duration_secs);
    let mut filter = String::new();
    let mut labels = Vec::new();

    if start_secs > 0.01 {
        filter.push_str(&format!(
            "[0:v:0]trim=start=0:end={},setpts=PTS-STARTPTS,scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[v0];",
            ffmpeg_duration_literal(start_secs)
        ));
        labels.push("[v0]");
    }

    if matches!(
        request.operation,
        VideoPostprocessOperation::Insert | VideoPostprocessOperation::Replace
    ) {
        if request.source_path.is_none() {
            return Err(VideoPostprocessError::MissingSource);
        }
        filter.push_str(&format!(
            "[1:v:0]scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[vsrc];"
        ));
        labels.push("[vsrc]");
    }

    let tail_start = if request.operation == VideoPostprocessOperation::Insert {
        start_secs
    } else {
        end_secs
    };
    if tail_start < base_duration_secs - 0.01 {
        filter.push_str(&format!(
            "[0:v:0]trim=start={}:end={},setpts=PTS-STARTPTS,scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[vtail];",
            ffmpeg_duration_literal(tail_start),
            ffmpeg_duration_literal(base_duration_secs)
        ));
        labels.push("[vtail]");
    }

    if labels.is_empty() {
        return Err(VideoPostprocessError::EmptyResult);
    }
    for label in &labels {
        filter.push_str(label);
    }
    filter.push_str(&format!("concat=n={}:v=1:a=0[outv]", labels.len()));
    Ok(filter)
}

pub fn postprocess_video(
    request: VideoPostprocessRequest<'_>,
) -> Result<(), VideoPostprocessError> {
    let filter = build_video_postprocess_filter(&request)?;
    let uses_source = matches!(
        request.operation,
        VideoPostprocessOperation::Insert | VideoPostprocessOperation::Replace
    );
    let source_path = if uses_source {
        let source_path = request
            .source_path
            .ok_or(VideoPostprocessError::MissingSource)?;
        if !source_path.is_file() {
            return Err(VideoPostprocessError::SourceNotFile);
        }
        Some(source_path)
    } else {
        None
    };

    let mut command = Command::new("ffmpeg");
    command
        .arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-i")
        .arg(request.base_path);
    if let Some(source_path) = source_path {
        command.arg("-i").arg(source_path);
    }
    let output = command
        .arg("-filter_complex")
        .arg(filter)
        .arg("-map")
        .arg("[outv]")
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("veryfast")
        .arg("-crf")
        .arg("20")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-an")
        .arg("-movflags")
        .arg("+faststart")
        .arg(request.output_path)
        .output()
        .map_err(|source| VideoPostprocessError::Unavailable { source })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(VideoPostprocessError::Failed {
            stderr: stderr_preview(&output, POSTPROCESS_STDERR_PREVIEW_CHARS),
        })
    }
}

fn even_positive_dimension(value: i32, default_value: i32) -> i32 {
    let value = if value > 0 { value } else { default_value };
    if value % 2 == 0 { value } else { value + 1 }
}
