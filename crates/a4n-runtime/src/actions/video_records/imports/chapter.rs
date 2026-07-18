use super::prelude::*;
use super::*;

pub async fn upload_chapter_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: UploadChapterVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let source = uploaded_video_source(
        state,
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
    resolve_project_chapter_video_source(state, project_id, chapter_id, "导入视频", true).await?;
    let prompt = import_video_prompt(state, project_id, chapter_id, prompt_id).await?;
    let properties = probe_video_properties(&source.path, "导入视频")?;
    let prompt_text = imported_video_prompt_text(&prompt);
    let metadata = imported_chapter_video_metadata(
        &prompt,
        &source.file_name,
        source.extension,
        Utc::now().to_rfc3339(),
    );

    let record = state
        .orchestrator
        .create_video_record(CreateVideoRecord {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: "chapter_video".into(),
            workflow_mode: Some(prompt.workflow_mode.clone()),
            prompt: prompt_text,
            negative_prompt: prompt.negative_prompt.clone(),
            model: "external-import".to_string(),
            width: properties.width,
            height: properties.height,
            duration_secs: properties.duration_secs,
            fps: properties.fps,
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
            import_video_output_path(
                state,
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
        &state.config.data_dir,
        project_id,
        chapter_id,
        record.id,
        &output,
    );
    let mut tail_frame_capture_error = None;
    let tail_frame_metadata =
        if should_capture_imported_chapter_video_tail_frame(&prompt.workflow_mode) {
            match capture_video_tail_frame_asset(
                state,
                &record,
                chapter_id,
                Some(prompt.id),
                &output,
            )
            .await
            {
                Ok(metadata) => Some(metadata),
                Err(err) => {
                    if prompt.workflow_mode == "first_frame_chain" {
                        let cleanup_error = combine_optional_errors(
                            remove_partial_video_file(&output, "导入视频文件").err(),
                            thumbnail_path.as_deref().and_then(|path| {
                                remove_partial_video_file(
                                    &state.config.data_dir.join(path),
                                    "视频缩略图",
                                )
                                .err()
                            }),
                        );
                        let cleanup_error = combine_optional_errors(
                            cleanup_error,
                            delete_video_tail_frame_assets(state, record.id).await.err(),
                        );
                        let delete_error = delete_video_record_error(state, record.id).await;
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
        &output,
        &output_path_str,
        thumbnail_path.as_deref(),
        source.file_size,
        Some(prompt.id),
        tail_frame_metadata.as_ref(),
        tail_frame_capture_error.as_deref(),
        "导入视频记录完成失败",
        "导入视频文件",
        "视频缩略图",
    )
    .await?;
    Ok(video_to_summary(&updated))
}

fn should_capture_imported_chapter_video_tail_frame(workflow_mode: &str) -> bool {
    matches!(
        workflow_mode,
        "boundary_frames" | "first_frame_chain" | "chat_video"
    )
}
