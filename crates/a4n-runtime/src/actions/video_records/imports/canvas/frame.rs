use super::super::processing::{canvas_frame_output_path, run_ffmpeg_extract_frame};
use crate::AppState;
use crate::actions::access;
use crate::actions::video_generation::media::append_secondary_error;
use crate::cleanup_outbox::enqueue_file_artifact_cleanup_by_project;
use a4n_api_contract::image::GeneratedImageSummary;
use a4n_domain::CanvasFrameKind;
use a4n_storage::{canonicalize_data_file_path, persist_created_media_file};

pub(crate) async fn extract_canvas_video_frame(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    video_id: i64,
    frame_kind: CanvasFrameKind,
    at_secs: Option<f32>,
) -> Result<GeneratedImageSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let plan = state
        .video_record_application()
        .prepare_canvas_frame(project_id, video_id, frame_kind, at_secs)
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
    let output = canvas_frame_output_path(
        state.data_dir(),
        project_id,
        plan.video.id,
        plan.frame_kind.as_str(),
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
        .video_record_application()
        .create_canvas_frame_record(plan, output_path_str.clone(), file_size)
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
    Ok(image)
}
