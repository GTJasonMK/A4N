use super::super::cache::{
    assembly_dimensions, assembly_fps, build_assembly_cache_manifest, run_ffmpeg_assemble_videos,
};
use super::super::contracts::{
    ASSEMBLY_PUSH_STATUS_PENDING, ASSEMBLY_PUSH_VIDEO_TYPE, AssemblyPushResult,
    PushAssembledChapterVideoInput,
};
use super::super::push::{
    active_assembly_push_source_ids, assembly_push_rows, assembly_request_context,
    ensure_active_pushes_match_clip_prefix, selected_assembly_push_source_clips,
};
use super::persistence::persist_assembly_record_output;
use crate::AppState;
use crate::actions::app_error_message;
use crate::actions::video_generation::media::best_effort_video_thumbnail_path;
use crate::actions::video_generation::video_to_summary;
use crate::actions::video_records::imports::{
    import_video_output_path, mark_imported_video_completed,
};
use a4n_orchestrator::video::CreateGeneratedVideoRecordDraftInput as CreateVideoRecord;
use chrono::Utc;
use serde_json::json;

pub async fn push_assembled_chapter_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: String,
    input: PushAssembledChapterVideoInput,
) -> Result<AssemblyPushResult, String> {
    let context = assembly_request_context(
        state,
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
        "推送成片片段",
    )
    .await?;
    if context.clips.is_empty() {
        return Err("当前视频生成流程还没有可推送的成片片段".to_string());
    }
    let rows = state
        .orchestrator
        .generated_videos_for_chapter(project_id, chapter_id)
        .await
        .map_err(app_error_message)?;
    let pushes = assembly_push_rows(&rows, context.manifest.workflow_mode.as_str())?;
    if pushes
        .iter()
        .any(|push| push.status == ASSEMBLY_PUSH_STATUS_PENDING)
    {
        return Err("当前最新推送尚未同意或拒绝，不能继续推送后续片段".to_string());
    }

    let active_source_ids = active_assembly_push_source_ids(&pushes);
    ensure_active_pushes_match_clip_prefix(&context.clips, &active_source_ids)?;
    let pending_clips = &context.clips[active_source_ids.len()..];
    if pending_clips.is_empty() {
        return Err("没有新的后续成片片段可推送".to_string());
    }
    let source_clips = selected_assembly_push_source_clips(pending_clips, input.end_video_id)?;
    let push_index = pushes
        .iter()
        .map(|push| push.push_index)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| "成片推送批次序号超过 i64 上限".to_string())?;
    let duration_secs = source_clips
        .iter()
        .map(|clip| clip.duration_secs.max(0.0))
        .sum::<f32>();
    let source_video_ids = source_clips
        .iter()
        .map(|clip| clip.video_id)
        .collect::<Vec<_>>();
    let source_sort_orders = source_clips
        .iter()
        .map(|clip| clip.sort_order)
        .collect::<Vec<_>>();
    let (width, height) = assembly_dimensions(&source_clips);
    let fps = assembly_fps(&source_clips);
    let manifest = build_assembly_cache_manifest(
        project_id,
        chapter_id,
        &context.manifest.workflow_mode,
        width,
        height,
        fps,
        &source_clips,
    )?;
    let metadata = json!({
        "assembly_push": {
            "status": ASSEMBLY_PUSH_STATUS_PENDING,
            "push_index": push_index,
            "workflow_mode": context.manifest.workflow_mode,
            "source_video_ids": source_video_ids,
            "source_sort_orders": source_sort_orders,
            "source_clip_count": source_clips.len(),
            "pushed_at": Utc::now().to_rfc3339(),
        }
    });
    let record = state
        .orchestrator
        .create_video_record(CreateVideoRecord {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: ASSEMBLY_PUSH_VIDEO_TYPE.to_string(),
            workflow_mode: Some(context.manifest.workflow_mode.clone()),
            prompt: format!("成片后处理推送批次 {push_index}"),
            negative_prompt: None,
            model: "a4n-assembly-push".to_string(),
            width,
            height,
            duration_secs,
            fps,
            source_video_id: source_video_ids.first().copied(),
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
                project_id,
                chapter_id,
                record.id,
                "assembly_push",
                "mp4",
            )
        },
        |output| run_ffmpeg_assemble_videos(&source_clips, output, width, height, fps, &manifest),
        "推送成片文件",
        "推送成片",
        "推送成片文件",
    )
    .await?;
    let thumbnail_path = best_effort_video_thumbnail_path(
        &state.config.data_dir,
        project_id,
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
        "推送成片记录完成失败",
        "推送成片文件",
        "推送成片缩略图",
    )
    .await?;
    Ok(AssemblyPushResult {
        video: video_to_summary(&updated),
        source_video_ids,
        push_index,
        status: ASSEMBLY_PUSH_STATUS_PENDING.to_string(),
    })
}
