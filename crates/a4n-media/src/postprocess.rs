use std::{io, path::Path, process::Command};

use a4n_domain::AssemblyPostprocessOperation;
use thiserror::Error;

use crate::ffmpeg::{seconds_literal, stderr_preview};

const POSTPROCESS_STDERR_PREVIEW_CHARS: usize = 800;

#[derive(Debug)]
pub struct VideoPostprocessRequest<'a> {
    pub base_path: &'a Path,
    pub source_path: Option<&'a Path>,
    pub output_path: &'a Path,
    pub operation: AssemblyPostprocessOperation,
    pub start_secs: f32,
    pub end_secs: f32,
    pub base_duration_secs: f32,
    pub base_has_audio: bool,
    pub source_duration_secs: Option<f32>,
    pub source_has_audio: bool,
    pub width: i32,
    pub height: i32,
    pub fps: i32,
}

#[derive(Debug, Error)]
pub enum VideoPostprocessError {
    #[error("video postprocess base duration must be positive and finite: {duration_secs}")]
    InvalidBaseDuration { duration_secs: f32 },
    #[error(
        "video postprocess range is invalid: start={start_secs}, end={end_secs}, base={base_duration_secs}"
    )]
    InvalidTimeRange {
        start_secs: f32,
        end_secs: f32,
        base_duration_secs: f32,
    },
    #[error("video postprocess dimensions must be positive: {width}x{height}")]
    InvalidDimensions { width: i32, height: i32 },
    #[error("video postprocess fps must be positive: {fps}")]
    InvalidFps { fps: i32 },
    #[error("insert or replace video postprocess requires a source video")]
    MissingSource,
    #[error("delete video postprocess does not accept a source video")]
    UnexpectedSource,
    #[error("video postprocess source is not a file")]
    SourceNotFile,
    #[error("insert or replace video postprocess requires a valid source duration")]
    InvalidSourceDuration,
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
    if request.width <= 0 || request.height <= 0 {
        return Err(VideoPostprocessError::InvalidDimensions {
            width: request.width,
            height: request.height,
        });
    }
    let width = even_dimension(request.width).ok_or(
        VideoPostprocessError::InvalidDimensions {
            width: request.width,
            height: request.height,
        },
    )?;
    let height = even_dimension(request.height).ok_or(
        VideoPostprocessError::InvalidDimensions {
            width: request.width,
            height: request.height,
        },
    )?;
    if request.fps <= 0 {
        return Err(VideoPostprocessError::InvalidFps { fps: request.fps });
    }
    if !request.base_duration_secs.is_finite() || request.base_duration_secs <= 0.0 {
        return Err(VideoPostprocessError::InvalidBaseDuration {
            duration_secs: request.base_duration_secs,
        });
    }
    let base_duration_secs = request.base_duration_secs;
    let start_secs = request.start_secs;
    let end_secs = request.end_secs;
    let uses_source = matches!(
        request.operation,
        AssemblyPostprocessOperation::Insert | AssemblyPostprocessOperation::Replace
    );
    let valid_range = start_secs.is_finite()
        && end_secs.is_finite()
        && start_secs >= 0.0
        && end_secs <= base_duration_secs
        && match request.operation {
            AssemblyPostprocessOperation::Insert => start_secs == end_secs,
            AssemblyPostprocessOperation::Delete | AssemblyPostprocessOperation::Replace => {
                end_secs > start_secs
                    && (request.operation != AssemblyPostprocessOperation::Delete
                        || end_secs - start_secs < base_duration_secs)
            }
        };
    if !valid_range {
        return Err(VideoPostprocessError::InvalidTimeRange {
            start_secs,
            end_secs,
            base_duration_secs,
        });
    }
    if uses_source && request.source_path.is_none() {
        return Err(VideoPostprocessError::MissingSource);
    }
    if !uses_source
        && (request.source_path.is_some()
            || request.source_duration_secs.is_some()
            || request.source_has_audio)
    {
        return Err(VideoPostprocessError::UnexpectedSource);
    }
    let audio_enabled = request.base_has_audio || (uses_source && request.source_has_audio);
    let source_duration_secs = request
        .source_duration_secs
        .filter(|value| value.is_finite() && *value > 0.0);
    if uses_source && source_duration_secs.is_none() {
        return Err(VideoPostprocessError::InvalidSourceDuration);
    }
    let mut filter = String::new();
    let mut labels = Vec::new();

    if start_secs > 0.0 {
        filter.push_str(&format!(
            "[0:v:0]trim=start=0:end={},setpts=PTS-STARTPTS,scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[v0];",
            seconds_literal(start_secs),
            fps = request.fps,
        ));
        if audio_enabled {
            append_audio_segment(
                &mut filter,
                0,
                request.base_has_audio,
                0.0,
                start_secs,
                "a0",
            );
        }
        labels.push(("v0", "a0"));
    }

    if uses_source {
        filter.push_str(&format!(
            "[1:v:0]scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[vsrc];",
            fps = request.fps,
        ));
        if audio_enabled {
            let source_duration_secs =
                source_duration_secs.ok_or(VideoPostprocessError::InvalidSourceDuration)?;
            append_audio_segment(
                &mut filter,
                1,
                request.source_has_audio,
                0.0,
                source_duration_secs,
                "asrc",
            );
        }
        labels.push(("vsrc", "asrc"));
    }

    let tail_start = if request.operation == AssemblyPostprocessOperation::Insert {
        start_secs
    } else {
        end_secs
    };
    if tail_start < base_duration_secs {
        filter.push_str(&format!(
            "[0:v:0]trim=start={}:end={},setpts=PTS-STARTPTS,scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[vtail];",
            seconds_literal(tail_start),
            seconds_literal(base_duration_secs),
            fps = request.fps,
        ));
        if audio_enabled {
            append_audio_segment(
                &mut filter,
                0,
                request.base_has_audio,
                tail_start,
                base_duration_secs,
                "atail",
            );
        }
        labels.push(("vtail", "atail"));
    }

    for (video_label, audio_label) in &labels {
        filter.push_str(&format!("[{video_label}]"));
        if audio_enabled {
            filter.push_str(&format!("[{audio_label}]"));
        }
    }
    if audio_enabled {
        filter.push_str(&format!("concat=n={}:v=1:a=1[outv][outa]", labels.len()));
    } else {
        filter.push_str(&format!("concat=n={}:v=1:a=0[outv]", labels.len()));
    }
    Ok(filter)
}

pub fn postprocess_video(
    request: VideoPostprocessRequest<'_>,
) -> Result<(), VideoPostprocessError> {
    let filter = build_video_postprocess_filter(&request)?;
    let uses_source = matches!(
        request.operation,
        AssemblyPostprocessOperation::Insert | AssemblyPostprocessOperation::Replace
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
    command
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
        .arg("-g")
        .arg(request.fps.to_string());
    let audio_enabled = request.base_has_audio || (uses_source && request.source_has_audio);
    if audio_enabled {
        command
            .arg("-map")
            .arg("[outa]")
            .arg("-c:a")
            .arg("aac")
            .arg("-b:a")
            .arg("192k");
    } else {
        command.arg("-an");
    }
    let output = command
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

fn append_audio_segment(
    filter: &mut String,
    input_index: usize,
    has_audio: bool,
    start_secs: f32,
    end_secs: f32,
    label: &str,
) {
    let duration = seconds_literal(end_secs - start_secs);
    if has_audio {
        filter.push_str(&format!(
            "[{input_index}:a:0]atrim=start={}:end={},asetpts=PTS-STARTPTS,aresample=async=1:first_pts=0,aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo,apad,atrim=duration={duration},asetpts=N/SR/TB[{label}];",
            seconds_literal(start_secs),
            seconds_literal(end_secs),
        ));
    } else {
        filter.push_str(&format!(
            "anullsrc=channel_layout=stereo:sample_rate=48000,atrim=duration={duration},asetpts=N/SR/TB[{label}];"
        ));
    }
}

fn even_dimension(value: i32) -> Option<i32> {
    if value % 2 == 0 {
        Some(value)
    } else {
        value.checked_add(1)
    }
}
