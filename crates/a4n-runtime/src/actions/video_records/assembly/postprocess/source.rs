use super::super::contracts::{ApplyAssemblyPostprocessEditInput, PostprocessEditSource};
use crate::AppState;
use crate::actions::app_error_message;
use crate::actions::video_records::imports::uploaded_video_source;
use crate::file_utils::canonicalize_data_file_path;
use a4n_orchestrator::video::{AssemblyPostprocessPlan, AssemblyPostprocessSourcePlan};
use std::path::Path;

pub(in crate::actions::video_records::assembly) async fn postprocess_edit_source(
    state: &AppState,
    plan: &AssemblyPostprocessPlan,
    input: &ApplyAssemblyPostprocessEditInput,
) -> Result<Option<PostprocessEditSource>, String> {
    let has_uploaded_source = [
        input.source_file_name.as_deref(),
        input.source_mime_type.as_deref(),
        input.source_data_base64.as_deref(),
    ]
    .iter()
    .any(|value| value.map(str::trim).is_some_and(|value| !value.is_empty()));
    match state
        .video()
        .prepare_assembly_postprocess_source_validated(
            plan,
            input.source_video_id,
            has_uploaded_source,
        )
        .await
        .map_err(app_error_message)?
    {
        AssemblyPostprocessSourcePlan::None => Ok(None),
        AssemblyPostprocessSourcePlan::Existing(video) => {
            let file_path = video
                .file_path
                .as_deref()
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .ok_or_else(|| format!("替换/插入来源视频 #{} 没有本地文件", video.id))?;
            let path = canonicalize_data_file_path(
                state.data_dir(),
                file_path,
                &format!("替换/插入来源视频 #{}", video.id),
            )?;
            Ok(Some(PostprocessEditSource {
                path,
                file_name: video
                    .file_path
                    .as_deref()
                    .and_then(|path| Path::new(path).file_name())
                    .and_then(|value| value.to_str())
                    .map(ToString::to_string),
                source_video_id: Some(video.id),
                _uploaded_source: None,
            }))
        }
        AssemblyPostprocessSourcePlan::Upload => {
            let file_name = input
                .source_file_name
                .as_deref()
                .ok_or_else(|| "上传后处理来源视频缺少文件名".to_string())?;
            let mime_type = input
                .source_mime_type
                .as_deref()
                .ok_or_else(|| "上传后处理来源视频缺少 MIME".to_string())?;
            let data_base64 = input
                .source_data_base64
                .as_deref()
                .ok_or_else(|| "上传后处理来源视频缺少数据".to_string())?;
            let source = uploaded_video_source(
                state.temp_dir(),
                file_name,
                mime_type,
                data_base64,
                "后处理来源视频",
            )?;
            Ok(Some(PostprocessEditSource {
                path: source.path.clone(),
                file_name: Some(source.file_name.clone()),
                source_video_id: None,
                _uploaded_source: Some(source),
            }))
        }
    }
}
