use super::super::prelude::{
    AppState, CreateImageRecord, GeneratedImageSummary, Utc, access, append_secondary_error,
    canonicalize_data_file_path, ensure_ffmpeg, json, persist_created_media_file,
    remove_partial_video_file, to_summary,
};
use super::super::processing::{
    canvas_frame_output_path, canvas_frame_timestamp, normalize_canvas_frame_kind,
    run_ffmpeg_extract_frame,
};

pub async fn extract_canvas_video_frame(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    frame_kind: String,
    at_secs: Option<f32>,
) -> Result<GeneratedImageSummary, String> {
    let video = state
        .orchestrator
        .generated_video_by_id(video_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .ok_or_else(|| "视频不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, video.project_id).await?;
    if video.chapter_id.is_some() {
        return Err("ExtractFrameNode 只支持项目级画布视频".to_string());
    }
    let source_path = video
        .file_path
        .as_deref()
        .ok_or_else(|| "源视频还没有可抽帧的本地文件".to_string())?;
    let source_path = canonicalize_data_file_path(
        &state.config.data_dir,
        source_path,
        &format!("源视频 #{}", video.id),
    )?;
    ensure_ffmpeg()?;

    let frame_kind = normalize_canvas_frame_kind(&frame_kind)?;
    let timestamp = canvas_frame_timestamp(&video, &frame_kind, at_secs)?;
    let output = canvas_frame_output_path(state, video.project_id, video.id, &frame_kind)?;
    let (output_path_str, file_size) = persist_created_media_file(
        &state.config.data_dir,
        &output,
        |output| run_ffmpeg_extract_frame(&source_path, output, timestamp),
        "画布抽帧图片",
        "画布抽帧图片",
        "画布抽帧图片",
    )?;
    let metadata = json!({
        "canvas_frame": {
            "source_video_id": video.id,
            "frame_kind": frame_kind,
            "at_secs": timestamp,
            "source_workflow_mode": video.workflow_mode,
            "extracted_at": Utc::now().to_rfc3339(),
        },
        "source": "video_canvas",
    });
    let image = match state
        .orchestrator
        .create_generated_image_record(CreateImageRecord {
            project_id: video.project_id,
            image_type: "canvas_frame".to_string(),
            file_path: output_path_str,
            prompt: format!("视频 #{} 画布抽帧 {frame_kind}", video.id),
            model: "ffmpeg".to_string(),
            width: video.width.max(1),
            height: video.height.max(1),
            file_size: Some(file_size),
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(metadata),
            created_at: None,
        })
        .await
    {
        Ok(image) => image,
        Err(err) => {
            return Err(append_secondary_error(
                format!("画布抽帧图片记录写入失败：{err}"),
                remove_partial_video_file(&output, "画布抽帧图片").err(),
            ));
        }
    };
    Ok(to_summary(&image))
}
