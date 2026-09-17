use crate::AppState;
use a4n_domain::AssemblyPostprocessOperation;
use a4n_media_contract::{
    MediaExecuteInput, MediaExecuteOutput, MediaPostprocessOperation, MediaRoot,
};
use std::path::Path;

pub(in crate::actions::video_records::assembly) struct MediaPostprocessEditInput<'a> {
    pub(in crate::actions::video_records::assembly) base_path: &'a Path,
    pub(in crate::actions::video_records::assembly) source_path: Option<&'a Path>,
    pub(in crate::actions::video_records::assembly) source_root: Option<MediaRoot>,
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

pub(in crate::actions::video_records::assembly) async fn run_media_postprocess_edit(
    state: &AppState,
    input: MediaPostprocessEditInput<'_>,
) -> Result<(), String> {
    let MediaPostprocessEditInput {
        base_path,
        source_path,
        source_root,
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
    let base = state.data_media_file_ref(base_path, "后处理基底视频")?;
    let source = match (source_path, source_root) {
        (Some(path), Some(MediaRoot::Data)) => {
            Some(state.data_media_file_ref(path, "后处理来源视频")?)
        }
        (Some(path), Some(MediaRoot::Temp)) => {
            Some(state.temp_media_file_ref(path, "后处理来源视频")?)
        }
        (None, None) => None,
        _ => return Err("后处理来源视频路径与根目录不匹配".to_string()),
    };
    let base_has_audio = state
        .probe_media_has_audio(base_path, MediaRoot::Data, "后处理基底视频")
        .await
        .map_err(|error| format!("无法探测后处理基底视频音频：{error}"))?;
    let source_has_audio = match source_path {
        Some(path) => state
            .probe_media_has_audio(
                path,
                source_root.ok_or_else(|| "后处理来源视频缺少根目录".to_string())?,
                "后处理来源视频",
            )
            .await
            .map_err(|error| format!("无法探测后处理来源视频音频：{error}"))?,
        None => false,
    };
    let operation = match operation {
        AssemblyPostprocessOperation::Insert => MediaPostprocessOperation::Insert,
        AssemblyPostprocessOperation::Delete => MediaPostprocessOperation::Delete,
        AssemblyPostprocessOperation::Replace => MediaPostprocessOperation::Replace,
    };
    match state
        .execute_media(MediaExecuteInput::PostprocessVideo {
            base,
            source,
            output: state.data_media_file_ref(output_path, "后处理输出视频")?,
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
        .await?
    {
        MediaExecuteOutput::Unit => Ok(()),
        output => Err(format!(
            "后处理剪辑失败：media worker 返回了不匹配的输出：{output:?}"
        )),
    }
}
