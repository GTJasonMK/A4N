use crate::actions::video_generation::media::{
    append_secondary_error, ensure_ffmpeg, remove_partial_video_file,
};
use std::path::Path;

pub(in crate::actions::video_records::assembly) struct MediaPostprocessEditInput<'a> {
    pub(in crate::actions::video_records::assembly) base_path: &'a Path,
    pub(in crate::actions::video_records::assembly) source_path: Option<&'a Path>,
    pub(in crate::actions::video_records::assembly) output_path: &'a Path,
    pub(in crate::actions::video_records::assembly) operation: &'a str,
    pub(in crate::actions::video_records::assembly) start_secs: f32,
    pub(in crate::actions::video_records::assembly) end_secs: f32,
    pub(in crate::actions::video_records::assembly) base_duration_secs: f32,
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
        width,
        height,
        fps,
    } = input;
    ensure_ffmpeg().map_err(|err| format!("{err}，无法执行后处理剪辑"))?;
    let operation = match operation {
        "insert" => a4n_media::VideoPostprocessOperation::Insert,
        "delete" => a4n_media::VideoPostprocessOperation::Delete,
        "replace" => a4n_media::VideoPostprocessOperation::Replace,
        other => return Err(format!("后处理操作无效：{other}")),
    };
    a4n_media::postprocess_video(a4n_media::VideoPostprocessRequest {
        base_path,
        source_path,
        output_path,
        operation,
        start_secs,
        end_secs,
        base_duration_secs,
        width,
        height,
        fps,
    })
    .map_err(|error| match error {
        a4n_media::VideoPostprocessError::MissingSource => "插入或替换操作缺少来源视频".to_string(),
        a4n_media::VideoPostprocessError::SourceNotFile => "插入或替换来源视频不是文件".to_string(),
        a4n_media::VideoPostprocessError::EmptyResult => "后处理剪辑结果为空".to_string(),
        a4n_media::VideoPostprocessError::Unavailable { source } => {
            format!("后处理剪辑失败：未能启动 FFmpeg：{source}")
        }
        a4n_media::VideoPostprocessError::Failed { stderr } => {
            let cleanup_error = remove_partial_video_file(output_path, "后处理剪辑输出文件").err();
            append_secondary_error(format!("后处理剪辑失败：{stderr}"), cleanup_error)
        }
    })
}
