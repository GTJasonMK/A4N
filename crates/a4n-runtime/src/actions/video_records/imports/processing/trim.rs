use super::super::prelude::{
    AppState, GeneratedVideoSummary, Path, access, append_secondary_error,
    best_effort_video_thumbnail_path, canonicalize_data_file_path, data_dir_relative_file_path,
    ensure_ffmpeg, remove_created_file_on_error_sync, run_ffmpeg_trim, trimmed_video_path,
    video_to_summary,
};
use crate::cleanup_outbox::enqueue_file_artifact_cleanup_by_project;

pub async fn trim_video(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    start_secs: f32,
    end_secs: f32,
) -> Result<GeneratedVideoSummary, String> {
    let plan = state
        .video()
        .prepare_video_trim_validated(video_id, start_secs, end_secs)
        .await
        .map_err(crate::actions::app_error_message)?;
    let source = &plan.source;
    access::verify_project_editor_string(state, user_id, source.project_id).await?;
    let source_path = source
        .file_path
        .clone()
        .ok_or_else(|| "源视频还没有可裁剪的本地文件".to_string())?;
    let source_path = canonicalize_data_file_path(
        state.data_dir(),
        &source_path,
        &format!("源视频 #{}", source.id),
    )?;
    ensure_ffmpeg()?;

    let output = trimmed_video_path(state.data_dir(), &source, start_secs, end_secs)?;
    let (size, output_path_str) = persist_trimmed_video_output(state.data_dir(), &output, || {
        run_ffmpeg_trim(&source_path, &output, start_secs, end_secs)
    })?;

    let source_id = source.id;
    let source_project_id = source.project_id;
    let source_chapter_id = source.chapter_id;
    let record = match state
        .video()
        .create_trimmed_video_record_validated(plan)
        .await
    {
        Ok(record) => record,
        Err(err) => {
            let cleanup_error = enqueue_file_artifact_cleanup_by_project(
                state,
                &format!("video-trim-create:{source_id}:{output_path_str}"),
                std::slice::from_ref(&output_path_str),
            )
            .await
            .err();
            return Err(append_secondary_error(err.to_string(), cleanup_error));
        }
    };
    let thumbnail_path = source_chapter_id.and_then(|chapter_id| {
        best_effort_video_thumbnail_path(
            state.data_dir(),
            source_project_id,
            chapter_id,
            record.id,
            &output,
        )
    });
    let updated = match state
        .video()
        .mark_generated_video_completed_and_link_prompt(
            record.id,
            &output_path_str,
            thumbnail_path.as_deref(),
            size,
            None,
            None,
        )
        .await
    {
        Ok(updated) => updated,
        Err(err) => {
            let primary_error = format!("裁剪文件已生成但记录完成状态失败：{err}");
            let cleanup_error = crate::actions::video_generation::cleanup_saved_video_outputs(
                state,
                record.id,
                &output,
                thumbnail_path.as_deref(),
            )
            .await;
            let failure_message = append_secondary_error(primary_error, cleanup_error);
            let mark_failed_error = state
                .video()
                .mark_generated_video_error_status_and_prompt(
                    record.id,
                    None,
                    "failed",
                    &failure_message,
                )
                .await
                .map_err(crate::actions::app_error_message)
                .err()
                .map(|error| format!("裁剪视频失败状态写入失败：{error}"));
            return Err(append_secondary_error(failure_message, mark_failed_error));
        }
    };
    Ok(video_to_summary(&updated))
}

pub(in crate::actions::video_records) fn persist_trimmed_video_output<F>(
    data_dir: &Path,
    output: &Path,
    trim: F,
) -> Result<(i64, String), String>
where
    F: FnOnce() -> Result<(), String>,
{
    remove_created_file_on_error_sync(output, "裁剪输出文件", || {
        trim()?;
        let size = std::fs::metadata(output)
            .map_err(|error| format!("无法读取裁剪视频文件元数据：{error}"))?
            .len();
        let size = i64::try_from(size).map_err(|_| "裁剪视频文件过大，无法记录大小".to_string())?;
        let relative_path = data_dir_relative_file_path(data_dir, output, "裁剪视频")?;
        Ok((size, relative_path))
    })
}
