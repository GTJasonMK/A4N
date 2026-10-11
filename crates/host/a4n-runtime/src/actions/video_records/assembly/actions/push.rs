use super::super::cache::{build_assembly_cache_manifest, run_ffmpeg_assemble_videos};
use super::super::contracts::{AssemblyPushResult, PushAssembledChapterVideoInput};
use super::super::push::assembly_request_context;
use super::persistence::persist_assembly_record_output;
use crate::AppState;
use crate::actions::app_error_message;
use crate::actions::video_generation::media::best_effort_video_thumbnail_path;
use crate::actions::video_records::{
    VideoRecordCompletionAttempt, complete_video_record, video_record_output_path,
};
use a4n_api_contract::video::AssemblyPushClipInput;
use a4n_domain::video_workflow::ASSEMBLY_PUSH_STATUS_PENDING;

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
    .await?
    .ok_or_else(|| "当前视频生成流程还没有可推送的成片片段".to_string())?;
    let prepared = state
        .video_assembly_push_application()
        .create_assembly_push_record(
            project_id,
            chapter_id,
            &context.manifest.workflow_mode,
            context
                .clips
                .iter()
                .map(|clip| AssemblyPushClipInput {
                    video_id: clip.video_id,
                    sort_order: clip.sort_order,
                    duration_secs: clip.duration_secs,
                    width: clip.width,
                    height: clip.height,
                    fps: clip.fps,
                })
                .collect(),
            input.end_video_id,
        )
        .await
        .map_err(app_error_message)?;
    // ponytail: 章节片段通常是几十条；数量上千时再改为 video_id -> clip 索引。
    let source_clips = prepared
        .source_video_ids
        .iter()
        .map(|video_id| {
            context
                .clips
                .iter()
                .find(|clip| clip.video_id == *video_id)
                .cloned()
                .ok_or_else(|| format!("推送源视频 #{video_id} 不在当前成片片段中"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let manifest = build_assembly_cache_manifest(
        state,
        project_id,
        chapter_id,
        &context.manifest.workflow_mode,
        prepared.video.width,
        prepared.video.height,
        prepared.video.fps,
        &source_clips,
    )
    .await?;
    let record = prepared.video;
    let (width, height, fps) = (record.width, record.height, record.fps);
    let (output, output_path_str, size) = persist_assembly_record_output(
        state,
        record.id,
        || {
            video_record_output_path(
                state.data_dir(),
                project_id,
                Some(chapter_id),
                record.id,
                "assembly_push",
                "mp4",
            )
        },
        |output| {
            let state = state.clone();
            let source_clips = source_clips.clone();
            let manifest = manifest.clone();
            async move {
                run_ffmpeg_assemble_videos(
                    &state,
                    &source_clips,
                    &output,
                    width,
                    height,
                    fps,
                    &manifest,
                )
                .await
            }
        },
        "推送成片文件",
        "推送成片",
        "推送成片文件",
    )
    .await?;
    let thumbnail_path =
        best_effort_video_thumbnail_path(state, project_id, chapter_id, record.id, &output).await;
    let updated = complete_video_record(
        state,
        VideoRecordCompletionAttempt {
            record_id: record.id,
            output_path: &output_path_str,
            thumbnail_path: thumbnail_path.as_deref(),
            file_size: size,
            prompt_id: None,
            metadata_json: None,
            failure_prefix: "推送成片记录完成失败",
        },
    )
    .await?;
    Ok(AssemblyPushResult {
        video: updated,
        source_video_ids: prepared.source_video_ids,
        push_index: prepared.push_index,
        status: ASSEMBLY_PUSH_STATUS_PENDING.to_string(),
    })
}
