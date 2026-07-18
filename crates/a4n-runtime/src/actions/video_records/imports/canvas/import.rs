use super::super::contracts::{ImportCanvasVideoInput, ImportedVideoSource};
use super::super::prelude::{
    AppState, CreateVideoRecord, GeneratedVideoSummary, Utc, access, json, video_to_summary,
};
use super::super::source::{
    import_project_video_output_path, import_video_source, mark_imported_video_completed,
    persist_imported_video_file, probe_video_properties, uploaded_video_source,
};

pub async fn import_canvas_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: ImportCanvasVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let project = state
        .orchestrator
        .project_by_id(project_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .ok_or_else(|| format!("项目不存在: {project_id}"))?;
    if project.project_kind != "video" {
        return Err("画布视频导入只支持视频项目".to_string());
    }
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
            uploaded_video_source(state, file_name, mime_type, data_base64, "画布视频导入")?
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
    let prompt = prompt
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("导入画布视频")
        .to_string();
    let metadata = json!({
        "canvas_import": {
            "external": true,
            "source_file_name": &source.file_name,
            "source_extension": source.extension,
            "imported_at": Utc::now().to_rfc3339()
        }
    });
    let record = state
        .orchestrator
        .create_video_record(CreateVideoRecord {
            project_id,
            chapter_id: None,
            video_type: "canvas_video".to_string(),
            workflow_mode: Some("canvas_video".to_string()),
            prompt,
            negative_prompt: None,
            model: "external-import".to_string(),
            width: probed.width,
            height: probed.height,
            duration_secs: probed.duration_secs,
            fps: probed.fps,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(metadata),
        })
        .await
        .map_err(crate::actions::app_error_message)?;

    let (output, output_path_str) = persist_imported_video_file(
        state,
        record.id,
        &source.path,
        || {
            import_project_video_output_path(
                state,
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
        &output,
        &output_path_str,
        None,
        source.file_size,
        None,
        None,
        None,
        "更新画布导入视频记录失败",
        "画布导入视频文件",
        "画布导入视频缩略图",
    )
    .await?;
    Ok(video_to_summary(&updated))
}
