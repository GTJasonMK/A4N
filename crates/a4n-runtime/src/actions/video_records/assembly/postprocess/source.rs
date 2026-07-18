use super::super::contracts::{ApplyAssemblyPostprocessEditInput, PostprocessEditSource};
use crate::AppState;
use crate::actions::access;
use crate::actions::app_error_message;
use crate::actions::video_records::imports::uploaded_video_source;
use crate::file_utils::canonicalize_data_file_path;
use std::path::Path;

pub(in crate::actions::video_records::assembly) async fn postprocess_edit_source(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: &ApplyAssemblyPostprocessEditInput,
    operation: &str,
) -> Result<Option<PostprocessEditSource>, String> {
    if operation == "delete" {
        return Ok(None);
    }
    let has_uploaded_source = [
        input.source_file_name.as_deref(),
        input.source_mime_type.as_deref(),
        input.source_data_base64.as_deref(),
    ]
    .iter()
    .any(|value| value.map(str::trim).is_some_and(|value| !value.is_empty()));
    let source_count = [input.source_video_id.is_some(), has_uploaded_source]
        .into_iter()
        .filter(|selected| *selected)
        .count();
    if source_count > 1 {
        return Err("插入或替换后处理片段只能选择一个来源视频".to_string());
    }
    if let Some(video_id) = input.source_video_id {
        let video = state
            .orchestrator
            .generated_video_by_id(video_id)
            .await
            .map_err(app_error_message)?
            .ok_or_else(|| "视频不存在".to_string())?;
        access::verify_project_editor_string(state, user_id, video.project_id).await?;
        if video.project_id != project_id {
            return Err(format!("替换/插入来源视频 #{} 不属于当前项目", video.id));
        }
        if video.status != "completed" {
            return Err(format!("替换/插入来源视频 #{} 尚未完成", video.id));
        }
        let file_path = video
            .file_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .ok_or_else(|| format!("替换/插入来源视频 #{} 没有本地文件", video.id))?;
        let path = canonicalize_data_file_path(
            &state.config.data_dir,
            file_path,
            &format!("替换/插入来源视频 #{}", video.id),
        )?;
        return Ok(Some(PostprocessEditSource {
            path,
            file_name: video
                .file_path
                .as_deref()
                .and_then(|path| Path::new(path).file_name())
                .and_then(|value| value.to_str())
                .map(ToString::to_string),
            source_video_id: Some(video.id),
            _uploaded_source: None,
        }));
    }
    if has_uploaded_source {
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
        let source =
            uploaded_video_source(state, file_name, mime_type, data_base64, "后处理来源视频")?;
        return Ok(Some(PostprocessEditSource {
            path: source.path.clone(),
            file_name: Some(source.file_name.clone()),
            source_video_id: None,
            _uploaded_source: Some(source),
        }));
    }
    Err("插入或替换后处理片段时必须选择上传视频或 API 生成视频".to_string())
}
