use super::super::prelude::{
    AppState, GeneratedImageSummary, access, append_secondary_error, canonicalize_data_file_path,
    ensure_ffmpeg, persist_created_media_file, to_summary,
};
use super::super::processing::{canvas_frame_output_path, run_ffmpeg_extract_frame};
use crate::cleanup_outbox::enqueue_file_artifact_cleanup_by_project;

pub async fn extract_canvas_video_frame(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    video_id: i64,
    frame_kind: String,
    at_secs: Option<f32>,
) -> Result<GeneratedImageSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let plan = state
        .orchestrator()
        .prepare_canvas_frame_validated(project_id, video_id, &frame_kind, at_secs)
        .await
        .map_err(crate::actions::app_error_message)?;
    let source_path = plan
        .video
        .file_path
        .as_deref()
        .ok_or_else(|| "源视频还没有可抽帧的本地文件".to_string())?;
    let source_path = canonicalize_data_file_path(
        state.data_dir(),
        source_path,
        &format!("源视频 #{}", plan.video.id),
    )?;
    ensure_ffmpeg()?;

    let output = canvas_frame_output_path(
        state.data_dir(),
        project_id,
        plan.video.id,
        &plan.frame_kind,
    )?;
    let (output_path_str, file_size) = persist_created_media_file(
        state.data_dir(),
        &output,
        |output| run_ffmpeg_extract_frame(&source_path, output, plan.timestamp),
        "画布抽帧图片",
        "画布抽帧图片",
        "画布抽帧图片",
    )?;
    let image = match state
        .orchestrator()
        .create_canvas_frame_record_validated(plan, output_path_str.clone(), file_size)
        .await
    {
        Ok(image) => image,
        Err(err) => {
            let cleanup_error = enqueue_file_artifact_cleanup_by_project(
                state,
                &format!("canvas-frame-create:{video_id}:{output_path_str}"),
                std::slice::from_ref(&output_path_str),
            )
            .await
            .err();
            return Err(append_secondary_error(
                format!("画布抽帧图片记录写入失败：{err}"),
                cleanup_error,
            ));
        }
    };
    Ok(to_summary(&image))
}
