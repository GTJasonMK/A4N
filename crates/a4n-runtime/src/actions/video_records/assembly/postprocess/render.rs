use std::path::Path;

use a4n_domain::AssemblyPostprocessOperation;

pub(in crate::actions::video_records::assembly) struct MediaPostprocessEditInput<'a> {
    pub(in crate::actions::video_records::assembly) base_path: &'a Path,
    pub(in crate::actions::video_records::assembly) source_path: Option<&'a Path>,
    pub(in crate::actions::video_records::assembly) output_path: &'a Path,
    pub(in crate::actions::video_records::assembly) operation: AssemblyPostprocessOperation,
    pub(in crate::actions::video_records::assembly) start_secs: f32,
    pub(in crate::actions::video_records::assembly) end_secs: f32,
    pub(in crate::actions::video_records::assembly) base_duration_secs: f32,
    pub(in crate::actions::video_records::assembly) source_duration_secs: Option<f32>,
    pub(in crate::actions::video_records::assembly) width: i32,
    pub(in crate::actions::video_records::assembly) height: i32,
    pub(in crate::actions::video_records::assembly) fps: i32,
}

pub(in crate::actions::video_records::assembly) fn run_media_postprocess_edit(
    input: MediaPostprocessEditInput<'_>,
) -> Result<(), String> {
    let MediaPostprocessEditInput {
        base_path,
        source_path,
        output_path,
        operation,
        start_secs,
        end_secs,
        base_duration_secs,
        source_duration_secs,
        width,
        height,
        fps,
    } = input;
    let base_has_audio = a4n_media::probe_video_has_audio(base_path)
        .map_err(|error| format!("无法探测后处理基底视频音频：{error}"))?;
    let source_has_audio = source_path
        .map(a4n_media::probe_video_has_audio)
        .transpose()
        .map_err(|error| format!("无法探测后处理来源视频音频：{error}"))?
        .unwrap_or(false);
    a4n_media::postprocess_video(a4n_media::VideoPostprocessRequest {
        base_path,
        source_path,
        output_path,
        operation,
        start_secs,
        end_secs,
        base_duration_secs,
        base_has_audio,
        source_duration_secs,
        source_has_audio,
        width,
        height,
        fps,
    })
    .map_err(|error| match error {
        a4n_media::VideoPostprocessError::InvalidBaseDuration { duration_secs } => {
            format!("后处理基底视频时长无效：{duration_secs}")
        }
        a4n_media::VideoPostprocessError::InvalidTimeRange {
            start_secs,
            end_secs,
            base_duration_secs,
        } => format!(
            "后处理时间范围无效：{start_secs}..{end_secs}，基底时长 {base_duration_secs}"
        ),
        a4n_media::VideoPostprocessError::InvalidDimensions { width, height } => {
            format!("后处理视频尺寸无效：{width}x{height}")
        }
        a4n_media::VideoPostprocessError::InvalidFps { fps } => {
            format!("后处理视频帧率无效：{fps}")
        }
        a4n_media::VideoPostprocessError::MissingSource => "插入或替换操作缺少来源视频".to_string(),
        a4n_media::VideoPostprocessError::UnexpectedSource => {
            "删除操作不接受来源视频".to_string()
        }
        a4n_media::VideoPostprocessError::SourceNotFile => "插入或替换来源视频不是文件".to_string(),
        a4n_media::VideoPostprocessError::InvalidSourceDuration => {
            "插入或替换来源视频时长无效".to_string()
        }
        a4n_media::VideoPostprocessError::Unavailable { source } => {
            format!("后处理剪辑失败：未能启动 FFmpeg：{source}")
        }
        a4n_media::VideoPostprocessError::Failed { stderr } => {
            format!("后处理剪辑失败：{stderr}")
        }
    })
}
