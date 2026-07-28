use super::super::contracts::{ImportCanvasVideoInput, ImportedVideoSource};
use super::super::prelude::{AppState, GeneratedVideoSummary, Utc, access, video_to_summary};
use super::super::source::{
    import_project_video_output_path, import_video_source, mark_imported_video_completed,
    persist_imported_video_file, probe_video_properties, uploaded_video_source,
};
use a4n_orchestrator::video::ImportedVideoProperties;

pub async fn import_canvas_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: ImportCanvasVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let source = match input
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        Some(file_path) => import_video_source(file_path, "画布视频导入")?,
        None => {
            let file_name = input
                .file_name
                .as_deref()
                .ok_or_else(|| "上传画布视频缺少 file_name".to_string())?;
            let mime_type = input
                .mime_type
                .as_deref()
                .ok_or_else(|| "上传画布视频缺少 mime_type".to_string())?;
            let data_base64 = input
                .data_base64
                .as_deref()
                .ok_or_else(|| "上传画布视频缺少 data_base64".to_string())?;
            uploaded_video_source(
                state.temp_dir(),
                file_name,
                mime_type,
                data_base64,
                "画布视频导入",
            )?
        }
    };
    create_imported_canvas_video(state, project_id, input.prompt, source).await
}

async fn create_imported_canvas_video(
    state: &AppState,
    project_id: i64,
    prompt: Option<String>,
    source: ImportedVideoSource,
) -> Result<GeneratedVideoSummary, String> {
    let probed = probe_video_properties(&source.path, "画布导入视频")?;
    let imported_at = Utc::now().to_rfc3339();
    let record = state
        .video()
        .create_imported_canvas_video_record_validated(
            project_id,
            prompt.as_deref(),
            ImportedVideoProperties {
                width: probed.width,
                height: probed.height,
                duration_secs: probed.duration_secs,
                fps: probed.fps,
            },
            &source.file_name,
            source.extension,
            &imported_at,
        )
        .await
        .map_err(crate::actions::app_error_message)?;

    let (_output, output_path_str) = persist_imported_video_file(
        &state.video(),
        state.data_dir(),
        record.id,
        &source.path,
        || {
            import_project_video_output_path(
                state.data_dir(),
                project_id,
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
    let updated = mark_imported_video_completed(
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
    Ok(video_to_summary(&updated))
}
