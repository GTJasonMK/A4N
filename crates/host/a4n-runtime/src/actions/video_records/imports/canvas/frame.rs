use super::super::processing::canvas_frame_output_path;
use crate::AppState;
use crate::actions::access;
use crate::cleanup_outbox::enqueue_file_artifact_cleanup_by_project;
use a4n_api_contract::image::GeneratedImageSummary;
use a4n_domain::CanvasFrameKind;
use a4n_error::append_secondary_message;
use a4n_media_contract::{MediaExecuteInput, MediaExecuteOutput};
use a4n_storage::{
    canonicalize_data_file_path, data_file_record_facts, remove_created_file_on_error,
};

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
        .video_canvas_media_application()
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
    let output_ref = state.data_media_file_ref(&output, "画布抽帧图片")?;
    let (output_path_str, file_size) =
        remove_created_file_on_error(&output, "画布抽帧图片", async {
            match state
                .execute_media(MediaExecuteInput::ExtractVideoFrame {
                    input: state.data_media_file_ref(&source_path, "抽帧源视频")?,
                    output: output_ref,
                    at_secs: plan.timestamp,
                })
                .await?
            {
                MediaExecuteOutput::Unit => {}
                output => return Err(format!("画布抽帧返回了不匹配的输出：{output:?}")),
            }
            data_file_record_facts(state.data_dir(), &output, "画布抽帧图片", "画布抽帧图片")
        })
        .await?;
    let image = match state
        .video_canvas_media_application()
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
            return Err(append_secondary_message(
                format!("画布抽帧图片记录写入失败：{err}"),
                cleanup_error,
            ));
        }
    };
    Ok(image)
}
