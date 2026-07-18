use super::super::prelude::{
    AppState, CreateVideoRecord, GeneratedVideoSummary, Path, Value, access,
    append_secondary_error, best_effort_video_thumbnail_path, canonicalize_data_file_path,
    combine_optional_errors, data_dir_relative_file_path, ensure_ffmpeg, generated_video, json,
    remove_created_file_on_error_sync, remove_partial_video_file, run_ffmpeg_trim,
    trimmed_video_path, video_to_summary,
};
use super::metadata::insert_source_context;

pub async fn trim_video(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    start_secs: f32,
    end_secs: f32,
) -> Result<GeneratedVideoSummary, String> {
    validate_video_trim_range(start_secs, end_secs)?;
    let source = state
        .orchestrator
        .generated_video_by_id(video_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .ok_or_else(|| "视频不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, source.project_id).await?;
    let source_workflow_mode = source.workflow_mode.clone().ok_or_else(|| {
        format!(
            "源视频 #{} 缺少 workflow_mode，无法按生成模式保存裁剪结果",
            source.id
        )
    })?;
    let target_workflow_mode =
        if source.chapter_id.is_none() || source_workflow_mode.starts_with("canvas_") {
            "canvas_trim".to_string()
        } else {
            source_workflow_mode
        };
    let source_path = source
        .file_path
        .clone()
        .ok_or_else(|| "源视频还没有可裁剪的本地文件".to_string())?;
    let source_path = canonicalize_data_file_path(
        &state.config.data_dir,
        &source_path,
        &format!("源视频 #{}", source.id),
    )?;
    if source.duration_secs > 0.0 && end_secs > source.duration_secs + 0.5 {
        return Err("裁剪结束时间超过视频时长".into());
    }
    ensure_ffmpeg()?;

    let output = trimmed_video_path(state, &source, start_secs, end_secs)?;
    let (size, output_path_str) =
        persist_trimmed_video_output(&state.config.data_dir, &output, || {
            run_ffmpeg_trim(&source_path, &output, start_secs, end_secs)
        })?;

    let record = match state
        .orchestrator
        .create_video_record(CreateVideoRecord {
            project_id: source.project_id,
            chapter_id: source.chapter_id,
            video_type: "trimmed_clip".into(),
            workflow_mode: Some(target_workflow_mode),
            prompt: source.prompt.clone(),
            negative_prompt: source.negative_prompt.clone(),
            model: source.model.clone(),
            width: source.width,
            height: source.height,
            duration_secs: end_secs - start_secs,
            fps: source.fps,
            source_video_id: Some(source.id),
            source_image_id: source.source_image_id,
            provider_job_id: None,
            metadata_json: Some(trimmed_video_metadata(&source, start_secs, end_secs)),
        })
        .await
    {
        Ok(record) => record,
        Err(err) => {
            return Err(append_secondary_error(
                err.to_string(),
                remove_partial_video_file(&output, "裁剪输出文件").err(),
            ));
        }
    };
    let thumbnail_path = source.chapter_id.and_then(|chapter_id| {
        best_effort_video_thumbnail_path(
            &state.config.data_dir,
            source.project_id,
            chapter_id,
            record.id,
            &output,
        )
    });
    let updated = match state
        .orchestrator
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
            let cleanup_error = combine_optional_errors(
                remove_partial_video_file(&output, "裁剪输出文件").err(),
                thumbnail_path.as_deref().and_then(|path| {
                    remove_partial_video_file(&state.config.data_dir.join(path), "视频缩略图").err()
                }),
            );
            let failure_message = append_secondary_error(primary_error, cleanup_error);
            let mark_failed_error = state
                .orchestrator
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

pub(in crate::actions::video_records) fn trimmed_video_metadata(
    source: &generated_video::Model,
    start_secs: f32,
    end_secs: f32,
) -> Value {
    let mut metadata = json!({
        "trim": {
            "start_secs": start_secs,
            "end_secs": end_secs,
        }
    });
    insert_source_context(&mut metadata, source.metadata_json.as_ref());
    metadata
}

pub(in crate::actions::video_records) fn validate_video_trim_range(
    start_secs: f32,
    end_secs: f32,
) -> Result<(), String> {
    if !start_secs.is_finite()
        || !end_secs.is_finite()
        || start_secs < 0.0
        || end_secs <= start_secs
    {
        return Err("裁剪时间范围无效".into());
    }
    Ok(())
}
