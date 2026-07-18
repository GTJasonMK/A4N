use std::path::Path;

pub(in crate::actions::video_generation::media) fn capture_video_tail_frame(
    video_path: &Path,
    output: &Path,
) -> Result<(), String> {
    a4n_media::extract_video_tail_frame(video_path, output).map_err(|error| match error {
        a4n_media::FfmpegCommandError::Unavailable { source } => {
            format!("采集视频尾帧失败：未能启动 FFmpeg：{source}")
        }
        a4n_media::FfmpegCommandError::Failed { stderr } => {
            format!("采集视频尾帧失败：{stderr}")
        }
    })
}

pub(crate) fn ensure_ffmpeg() -> Result<(), String> {
    a4n_media::check_ffmpeg_available().map_err(|error| match error {
        a4n_media::FfmpegAvailabilityError::Unavailable { .. } => {
            "未检测到 FFmpeg，无法裁剪视频".to_string()
        }
        a4n_media::FfmpegAvailabilityError::Unhealthy => "FFmpeg 不可用，无法裁剪视频".to_string(),
    })
}

pub(crate) fn run_ffmpeg_trim(
    input: &Path,
    output: &Path,
    start_secs: f32,
    end_secs: f32,
) -> Result<(), String> {
    a4n_media::trim_video_segment(input, output, start_secs, end_secs).map_err(
        |error| match error {
            a4n_media::FfmpegTrimError::InvalidTimeRange => {
                "FFmpeg 裁剪失败：裁剪时间范围无效".to_string()
            }
            a4n_media::FfmpegTrimError::CopyUnavailable { source } => {
                format!("启动 FFmpeg 失败：{source}")
            }
            a4n_media::FfmpegTrimError::RecodeUnavailable { source } => {
                format!("启动 FFmpeg 重编码失败：{source}")
            }
            a4n_media::FfmpegTrimError::Failed { stderr } => {
                format!("FFmpeg 裁剪失败：{stderr}")
            }
        },
    )
}
