use super::super::contracts::{
    ASSEMBLY_POSTPROCESS_EDIT_VIDEO_TYPE, ASSEMBLY_PUSH_STATUS_REJECTED, ASSEMBLY_PUSH_VIDEO_TYPE,
    ApplyAssemblyPostprocessEditInput,
};
use super::super::postprocess::{
    MediaPostprocessEditInput, latest_postprocess_version, postprocess_edit_range,
    postprocess_edit_source, run_media_postprocess_edit, validate_postprocess_operation,
};
use super::super::push::{assembly_push_rows, assembly_push_status, latest_non_rejected_push};
use super::persistence::persist_assembly_record_output;
use crate::AppState;
use crate::actions::access;
use crate::actions::app_error_message;
use crate::actions::video_generation::media::best_effort_video_thumbnail_path;
use crate::actions::video_generation::{GeneratedVideoSummary, video_to_summary};
use crate::actions::video_records::imports::{
    import_video_output_path, mark_imported_video_completed,
};
use crate::file_utils::canonicalize_data_file_path;
use a4n_orchestrator::video::CreateGeneratedVideoRecordDraftInput as CreateVideoRecord;
use chrono::Utc;
use serde_json::json;

pub async fn apply_assembly_postprocess_edit(
    state: &AppState,
    user_id: i64,
    push_video_id: i64,
    input: ApplyAssemblyPostprocessEditInput,
) -> Result<GeneratedVideoSummary, String> {
    let push = state
        .orchestrator
        .generated_video_by_id(push_video_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| "视频不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, push.project_id).await?;
    if push.video_type != ASSEMBLY_PUSH_VIDEO_TYPE {
        return Err(format!("视频 #{} 不是后处理推送批次", push.id));
    }
    if assembly_push_status(&push)? == ASSEMBLY_PUSH_STATUS_REJECTED {
        return Err("已拒绝的推送批次不能继续编辑".to_string());
    }
    let chapter_id = push
        .chapter_id
        .ok_or_else(|| format!("推送批次 #{} 缺少章节绑定", push.id))?;
    let workflow_mode = push
        .workflow_mode
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("推送批次 #{} 缺少 workflow_mode", push.id))?;
    let rows = state
        .orchestrator
        .generated_videos_for_chapter(push.project_id, chapter_id)
        .await
        .map_err(app_error_message)?;
    let pushes = assembly_push_rows(&rows, workflow_mode)?;
    let latest = latest_non_rejected_push(&pushes)
        .ok_or_else(|| "当前章节没有可编辑的推送批次".to_string())?;
    if latest.video.id != push.id {
        return Err("只能编辑当前最新推送批次，不能跨推送操作".to_string());
    }
    let base = latest_postprocess_version(state, &push).await?;
    let base_path = base
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| format!("后处理基底视频 #{} 没有本地文件", base.id))?;
    let base_path = canonicalize_data_file_path(
        &state.config.data_dir,
        base_path,
        &format!("后处理基底视频 #{}", base.id),
    )?;
    let operation = validate_postprocess_operation(&input.operation)?;
    let source =
        postprocess_edit_source(state, user_id, push.project_id, &input, operation).await?;
    let (start_secs, end_secs, source_duration) =
        postprocess_edit_range(&base, &input, operation, source.as_ref())?;
    let output_duration = match operation {
        "insert" => base.duration_secs.max(0.0) + source_duration.unwrap_or(0.0),
        "delete" => (base.duration_secs - (end_secs - start_secs)).max(0.1),
        "replace" => {
            (base.duration_secs - (end_secs - start_secs) + source_duration.unwrap_or(0.0)).max(0.1)
        }
        _ => base.duration_secs,
    };
    let metadata = json!({
        "postprocess_edit": {
            "push_video_id": push.id,
            "base_video_id": base.id,
            "operation": operation,
            "at_secs": input.at_secs,
            "start_secs": input.start_secs,
            "end_secs": input.end_secs,
            "source_video_id": source.as_ref().and_then(|source| source.source_video_id),
            "source_file_name": source.as_ref().and_then(|source| source.file_name.clone()),
            "edited_at": Utc::now().to_rfc3339(),
        }
    });
    let record = state
        .orchestrator
        .create_video_record(CreateVideoRecord {
            project_id: push.project_id,
            chapter_id: Some(chapter_id),
            video_type: ASSEMBLY_POSTPROCESS_EDIT_VIDEO_TYPE.to_string(),
            workflow_mode: push.workflow_mode.clone(),
            prompt: format!("后处理编辑：推送批次 #{} {}", push.id, operation),
            negative_prompt: None,
            model: "a4n-postprocess-edit".to_string(),
            width: base.width,
            height: base.height,
            duration_secs: output_duration,
            fps: base.fps,
            source_video_id: Some(base.id),
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(metadata),
        })
        .await
        .map_err(crate::actions::app_error_message)?;
    let (output, output_path_str, size) = persist_assembly_record_output(
        state,
        record.id,
        || {
            import_video_output_path(
                state,
                push.project_id,
                chapter_id,
                record.id,
                "postprocess_edit",
                "mp4",
            )
        },
        |output| {
            run_media_postprocess_edit(MediaPostprocessEditInput {
                base_path: &base_path,
                source_path: source.as_ref().map(|source| source.path.as_path()),
                output_path: output,
                operation,
                start_secs,
                end_secs,
                base_duration_secs: base.duration_secs,
                width: base.width,
                height: base.height,
                fps: base.fps,
            })
        },
        "后处理编辑视频文件",
        "后处理编辑视频",
        "后处理编辑视频文件",
    )
    .await?;
    let thumbnail_path = best_effort_video_thumbnail_path(
        &state.config.data_dir,
        push.project_id,
        chapter_id,
        record.id,
        &output,
    );
    let updated = mark_imported_video_completed(
        state,
        record.id,
        &output,
        &output_path_str,
        thumbnail_path.as_deref(),
        size,
        None,
        None,
        None,
        "后处理编辑视频记录完成失败",
        "后处理编辑视频文件",
        "后处理编辑视频缩略图",
    )
    .await?;
    Ok(video_to_summary(&updated))
}
