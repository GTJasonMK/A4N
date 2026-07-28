use super::prelude::*;
use super::*;
use a4n_orchestrator::video::{
    ImportedVideoProperties, completed_provider_metadata, is_first_frame_chain_video,
    should_capture_video_tail_frame,
};

pub async fn upload_chapter_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: UploadChapterVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let source = uploaded_video_source(
        state.temp_dir(),
        &input.file_name,
        &input.mime_type,
        &input.data_base64,
        "导入视频",
    )?;
    create_imported_chapter_video(
        state,
        user_id,
        project_id,
        chapter_id,
        input.prompt_id,
        source,
    )
    .await
}

async fn create_imported_chapter_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    prompt_id: i64,
    source: ImportedVideoSource,
) -> Result<GeneratedVideoSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let properties = probe_video_properties(&source.path, "导入视频")?;
    let imported_at = Utc::now().to_rfc3339();
    let prepared = state
        .video()
        .create_imported_chapter_video_record_validated(
            project_id,
            chapter_id,
            prompt_id,
            ImportedVideoProperties {
                width: properties.width,
                height: properties.height,
                duration_secs: properties.duration_secs,
                fps: properties.fps,
            },
            &source.file_name,
            source.extension,
            &imported_at,
        )
        .await
        .map_err(crate::actions::app_error_message)?;
    let record = prepared.video;
    let prompt = prepared.prompt;

    let (output, output_path_str) = persist_imported_video_file(
        &state.video(),
        state.data_dir(),
        record.id,
        &source.path,
        || {
            import_video_output_path(
                state.data_dir(),
                project_id,
                chapter_id,
                record.id,
                "imported_video",
                source.extension,
            )
        },
        "复制导入视频失败",
        "导入视频",
        "导入视频文件",
    )
    .await?;
    let thumbnail_path = best_effort_video_thumbnail_path(
        state.data_dir(),
        project_id,
        chapter_id,
        record.id,
        &output,
    );
    let mut tail_frame_capture_error = None;
    let tail_frame_metadata = if should_capture_video_tail_frame(&record) {
        match capture_video_tail_frame_asset(state, &record, chapter_id, Some(prompt.id), &output)
            .await
        {
            Ok(metadata) => Some(metadata),
            Err(err) => {
                if is_first_frame_chain_video(&record) {
                    let cleanup_error =
                        crate::actions::video_generation::cleanup_saved_video_outputs(
                            state,
                            record.id,
                            &output,
                            thumbnail_path.as_deref(),
                        )
                        .await;
                    let delete_error = delete_video_record_error(&state.video(), record.id).await;
                    return Err(append_secondary_error(
                        append_secondary_error(
                            format!("导入视频已复制但首帧串联尾帧采集失败：{err}"),
                            cleanup_error,
                        ),
                        delete_error,
                    ));
                }
                tail_frame_capture_error = Some(err);
                None
            }
        }
    } else {
        None
    };
    let updated = mark_imported_video_completed(
        state,
        record.id,
        &output_path_str,
        thumbnail_path.as_deref(),
        source.file_size,
        Some(prompt.id),
        completed_provider_metadata(Value::Null, tail_frame_metadata, tail_frame_capture_error),
        "导入视频记录完成失败",
    )
    .await?;
    Ok(video_to_summary(&updated))
}
