use super::super::prelude::{
    AppState, GeneratedVideoSummary, Path, access, append_secondary_error,
    best_effort_video_thumbnail_path, canonicalize_data_file_path, data_file_record_facts,
    remove_created_file_on_error_sync, run_ffmpeg_trim, trimmed_video_path, video_to_summary,
};
use crate::actions::video_records::enqueue_unadopted_video_file_cleanup;
use crate::cleanup_outbox::enqueue_file_artifact_cleanup_by_project;
use a4n_core::models::ai::generated_video;

pub async fn trim_video(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    start_secs: f32,
    end_secs: f32,
) -> Result<GeneratedVideoSummary, String> {
    let source = state
        .orchestrator()
        .required_video_record(video_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    access::verify_project_editor_string(state, user_id, source.project_id).await?;
    let plan = state
        .orchestrator()
        .prepare_video_trim_validated(source, start_secs, end_secs)
        .map_err(crate::actions::app_error_message)?;
    let source = &plan.source;
    let start_secs = plan.start_secs;
    let end_secs = plan.end_secs;
    let source_path = source
        .file_path
        .as_deref()
        .ok_or_else(|| "源视频还没有可裁剪的本地文件".to_string())?;
    let source_path = canonicalize_data_file_path(
        state.data_dir(),
        source_path,
        &format!("源视频 #{}", source.id),
    )?;
    let output = trimmed_video_path(state.data_dir(), source)?;
    let (size, output_path_str) = persist_trimmed_video_output(state.data_dir(), &output, || {
        run_ffmpeg_trim(&source_path, &output, start_secs, end_secs)
    })?;

    let source_id = source.id;
    let source_project_id = source.project_id;
    let source_chapter_id = source.chapter_id;
    let record = match state
        .orchestrator()
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
        .orchestrator()
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
            let can_adopt = a4n_orchestrator::video::can_adopt_authoritative_video_completion(&err);
            let primary_error = format!("裁剪文件已生成但记录完成状态失败：{err}");
            let authoritative = match state.orchestrator().generated_video_by_id(record.id).await {
                Ok(authoritative) => authoritative,
                Err(lookup_error) => {
                    return Err(append_secondary_error(
                        primary_error,
                        Some(format!(
                            "无法确认视频完成事务是否已提交，已保留文件等待重试：{}",
                            crate::actions::app_error_message(lookup_error)
                        )),
                    ));
                }
            };
            if let Some(authoritative) = authoritative.as_ref()
                && authoritative.status == generated_video::STATUS_COMPLETED
            {
                let cleanup_error = enqueue_unadopted_video_file_cleanup(
                    state,
                    &format!("video-trim-duplicate:{}", record.id),
                    &output_path_str,
                    thumbnail_path.as_deref(),
                    Some(authoritative),
                )
                .await;
                if can_adopt {
                    if let Some(cleanup_error) = cleanup_error {
                        tracing::error!(
                            video_id = record.id,
                            error = %cleanup_error,
                            "清理未采用的裁剪视频文件失败"
                        );
                    }
                    return Ok(video_to_summary(authoritative));
                }
                return Err(append_secondary_error(primary_error, cleanup_error));
            }
            let cleanup_error = crate::actions::video_generation::cleanup_saved_video_outputs(
                state,
                record.id,
                &output,
                thumbnail_path.as_deref(),
            )
            .await;
            let failure_message = append_secondary_error(primary_error, cleanup_error);
            if authoritative.as_ref().is_some_and(|video| {
                matches!(
                    video.status.as_str(),
                    generated_video::STATUS_FAILED
                        | generated_video::STATUS_INTERRUPTED
                        | generated_video::STATUS_CANCELLED
                )
            }) {
                return Err(failure_message);
            }
            let mark_failed_error = state
                .orchestrator()
                .mark_generated_video_error_status_and_prompt(
                    record.id,
                    None,
                    generated_video::STATUS_FAILED,
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
        let (relative_path, size) =
            data_file_record_facts(data_dir, output, "裁剪视频", "裁剪视频文件")?;
        Ok((size, relative_path))
    })
}
