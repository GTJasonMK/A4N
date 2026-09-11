mod assembly;
mod frame;

use super::contracts::ImportedVideoSource;
use super::source::{
    import_video_source, persist_imported_video_file, probe_imported_video_properties,
    uploaded_video_source,
};
use crate::AppState;
use crate::actions::video_records::{complete_video_record, video_record_output_path};
use a4n_canvas_contract::CanvasVideoImportHostInput;
use chrono::Utc;

pub(crate) use assembly::assemble_canvas_videos;
pub(crate) use frame::extract_canvas_video_frame;

pub(crate) async fn import_canvas_video(
    state: &AppState,
    input: CanvasVideoImportHostInput,
) -> Result<i64, String> {
    let project_id = input.project_id;
    let source = match (input.file_path.as_deref(), input.data_base64.as_deref()) {
        (Some(_), Some(_)) => {
            return Err("导入画布视频只能提供 file_path 或 data_base64，不能同时提供".to_string());
        }
        (Some(_), None) if input.file_name.is_some() || input.mime_type.is_some() => {
            return Err("file_path 画布视频导入不能同时提供 file_name 或 mime_type".to_string());
        }
        (Some(file_path), None) => import_video_source(file_path, "画布视频导入")?,
        (None, Some(data_base64)) => {
            let file_name = input
                .file_name
                .as_deref()
                .ok_or_else(|| "上传画布视频缺少 file_name".to_string())?;
            let mime_type = input
                .mime_type
                .as_deref()
                .ok_or_else(|| "上传画布视频缺少 mime_type".to_string())?;
            uploaded_video_source(
                state.temp_dir(),
                file_name,
                mime_type,
                data_base64,
                "画布视频导入",
            )?
        }
        (None, None) => return Err("导入画布视频缺少 file_path 或 data_base64".to_string()),
    };
    create_imported_canvas_video(state, project_id, source).await
}

async fn create_imported_canvas_video(
    state: &AppState,
    project_id: i64,
    source: ImportedVideoSource,
) -> Result<i64, String> {
    let properties = probe_imported_video_properties(&source, "画布导入视频")?;
    let imported_at = Utc::now().to_rfc3339();
    let record = state
        .video_record_application()
        .create_imported_canvas_video_record(
            project_id,
            properties,
            &source.file_name,
            source.extension,
            &imported_at,
        )
        .await
        .map_err(crate::actions::app_error_message)?;

    let (_output, output_path_str) = persist_imported_video_file(
        state,
        state.data_dir(),
        record.id,
        &source.path,
        || {
            video_record_output_path(
                state.data_dir(),
                project_id,
                None,
                record.id,
                "imported_canvas_video",
                source.extension,
            )
        },
        "复制画布导入视频失败",
        "画布导入视频",
        "画布导入视频文件",
    )
    .await?;
    let updated = complete_video_record(
        state,
        record.id,
        &output_path_str,
        None,
        source.file_size,
        None,
        None,
        "更新画布导入视频记录失败",
    )
    .await?;
    Ok(updated.id)
}
