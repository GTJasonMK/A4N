use crate::AppState;
use crate::actions::video_generation::GeneratedVideoSummary;
use crate::actions::video_generation::media::{
    append_secondary_error, best_effort_video_thumbnail_path, run_ffmpeg_trim, trimmed_video_path,
};
use crate::actions::video_records::complete_video_record;
use crate::cleanup_outbox::enqueue_file_artifact_cleanup_by_project;
use a4n_storage::{
    canonicalize_data_file_path, data_file_record_facts, remove_created_file_on_error_sync,
};
use std::path::Path;

pub async fn trim_video(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    start_secs: f32,
    end_secs: f32,
) -> Result<GeneratedVideoSummary, String> {
    let plan = state
        .video_record_application()
        .prepare_trim(user_id, video_id, start_secs, end_secs)
        .await
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
    let output = trimmed_video_path(
        state.data_dir(),
        source.project_id,
        source.chapter_id,
        source.id,
    )?;
    let (size, output_path_str) = persist_trimmed_video_output(state.data_dir(), &output, || {
        run_ffmpeg_trim(&source_path, &output, start_secs, end_secs)
    })?;

    let source_id = source.id;
    let source_project_id = source.project_id;
    let source_chapter_id = source.chapter_id;
    let record = match state
        .video_record_application()
        .create_trimmed_record(plan)
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
    let updated = complete_video_record(
        state,
        record.id,
        &output_path_str,
        thumbnail_path.as_deref(),
        size,
        None,
        None,
        "裁剪文件已生成但记录完成状态失败",
    )
    .await?;
    Ok(updated)
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
