use std::{io, path::Path, process::Command};

use thiserror::Error;

use crate::ffmpeg::stderr_preview;

const ASSEMBLY_STDERR_PREVIEW_CHARS: usize = 800;

#[derive(Debug, Clone, Copy)]
pub struct VideoAssemblyClip<'a> {
    pub path: &'a Path,
    pub duration_secs: f32,
    pub has_audio: bool,
}

#[derive(Debug)]
pub struct VideoAssemblyRequest<'a> {
    pub clips: &'a [VideoAssemblyClip<'a>],
    pub output_path: &'a Path,
    pub width: i32,
    pub height: i32,
    pub fps: i32,
    pub audio_enabled: bool,
}

#[derive(Debug, Error)]
pub enum VideoAssemblyError {
    #[error("video assembly requires at least one clip")]
    EmptyClips,
    #[error("video assembly dimensions must be positive: {width}x{height}")]
    InvalidDimensions { width: i32, height: i32 },
    #[error("video assembly fps must be positive: {fps}")]
    InvalidFps { fps: i32 },
    #[error("failed to start ffmpeg video assembly: {source}")]
    Unavailable {
        #[source]
        source: io::Error,
    },
    #[error("ffmpeg video assembly failed: {stderr}")]
    Failed { stderr: String },
}

pub fn build_video_assembly_filter(
    clips: &[VideoAssemblyClip<'_>],
    width: i32,
    height: i32,
    fps: i32,
    audio_enabled: bool,
) -> Result<String, VideoAssemblyError> {
    validate_assembly(clips, width, height, fps)?;
    let clip_count = clips.len();
    let mut filter = String::new();
    for index in 0..clip_count {
        filter.push_str(&format!(
            "[{index}:v:0]scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[v{index}];"
        ));
    }

    if audio_enabled {
        for (index, clip) in clips.iter().enumerate() {
            if clip.has_audio {
                filter.push_str(&format!(
                    "[{index}:a:0]aresample=async=1:first_pts=0,aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo,asetpts=N/SR/TB[a{index}];"
                ));
            } else {
                filter.push_str(&format!(
                    "anullsrc=channel_layout=stereo:sample_rate=48000,atrim=duration={},asetpts=N/SR/TB[a{index}];",
                    ffmpeg_duration_literal(clip.duration_secs)
                ));
            }
        }
    }

    for index in 0..clip_count {
        if audio_enabled {
            filter.push_str(&format!("[v{index}][a{index}]"));
        } else {
            filter.push_str(&format!("[v{index}]"));
        }
    }
    if audio_enabled {
        filter.push_str(&format!("concat=n={clip_count}:v=1:a=1[outv][outa]"));
    } else {
        filter.push_str(&format!("concat=n={clip_count}:v=1:a=0[outv]"));
    }
    Ok(filter)
}

pub fn assemble_videos(request: VideoAssemblyRequest<'_>) -> Result<(), VideoAssemblyError> {
    let filter = build_video_assembly_filter(
        request.clips,
        request.width,
        request.height,
        request.fps,
        request.audio_enabled,
    )?;
    let mut command = Command::new("ffmpeg");
    command
        .arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error");
    for clip in request.clips {
        command.arg("-i").arg(clip.path);
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
        .arg("yuv420p");
    if request.audio_enabled {
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
        .map_err(|source| VideoAssemblyError::Unavailable { source })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(VideoAssemblyError::Failed {
            stderr: stderr_preview(&output, ASSEMBLY_STDERR_PREVIEW_CHARS),
        })
    }
}

pub fn ffmpeg_duration_literal(value: f32) -> String {
    let value = if value.is_finite() && value > 0.0 {
        value
    } else {
        0.1
    };
    format!("{value:.3}")
}

fn validate_assembly(
    clips: &[VideoAssemblyClip<'_>],
    width: i32,
    height: i32,
    fps: i32,
) -> Result<(), VideoAssemblyError> {
    if clips.is_empty() {
        return Err(VideoAssemblyError::EmptyClips);
    }
    if width <= 0 || height <= 0 {
        return Err(VideoAssemblyError::InvalidDimensions { width, height });
    }
    if fps <= 0 {
        return Err(VideoAssemblyError::InvalidFps { fps });
    }
    Ok(())
}
