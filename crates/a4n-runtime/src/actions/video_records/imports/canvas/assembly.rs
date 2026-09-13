use crate::AppState;
use crate::actions::access;
use crate::actions::video_generation::GeneratedVideoSummary;
use crate::actions::video_generation::media::append_secondary_error;
use crate::actions::video_records::assembly::{
    AssemblyClip, build_assembly_cache_manifest, run_ffmpeg_assemble_videos,
};
use crate::actions::video_records::{complete_video_record, video_record_output_path};
use a4n_storage::{canonicalize_data_file_path, persist_created_media_file};
use std::path::{Path, PathBuf};

pub(crate) async fn assemble_canvas_videos(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    video_ids: Vec<i64>,
    title: Option<String>,
) -> Result<GeneratedVideoSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let plan = state
        .video_canvas_media_application()
        .prepare_canvas_assembly(project_id, video_ids, title.as_deref())
        .await
        .map_err(crate::actions::app_error_message)?;
    let mut clips = Vec::with_capacity(plan.sources.len());
    for (index, video) in plan.sources.iter().enumerate() {
        let video_id = video.id;
        let file_path = video
            .file_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("视频 #{video_id} 缺少本地文件"))?;
        let path = canonicalize_data_file_path(
            state.data_dir(),
            file_path,
            &format!("画布成片片段视频 #{video_id}"),
        )?;
        clips.push(AssemblyClip {
            video_id,
            video_type: video.video_type.clone(),
            shot_number: i32::try_from(index + 1)
                .map_err(|_| "画布成片片段数量超出 i32 范围".to_string())?,
            sort_order: index as f64,
            insert_after_video_id: None,
            path,
            file_path: file_path.to_string(),
            duration_secs: video.duration_secs,
            width: video.width,
            height: video.height,
            fps: video.fps,
        });
    }
    let (width, height, fps) = (plan.width, plan.height, plan.fps);
    let record = state
        .video_canvas_media_application()
        .create_canvas_assembly_record(project_id, plan)
        .await
        .map_err(crate::actions::app_error_message)?;
    let manifest = build_assembly_cache_manifest(
        project_id,
        0,
        "canvas_assembly",
        width,
        height,
        fps,
        &clips,
    )?;
    let (output_path_str, file_size) = persist_canvas_assembly_output(
        state,
        record.id,
        || {
            video_record_output_path(
                state.data_dir(),
                project_id,
                None,
                record.id,
                "canvas_assembly",
                "mp4",
            )
        },
        |output| run_ffmpeg_assemble_videos(&clips, output, width, height, fps, &manifest),
    )
    .await?;
    let updated = complete_video_record(
        state,
        record.id,
        &output_path_str,
        None,
        file_size,
        None,
        None,
        "更新画布成片记录失败",
    )
    .await?;
    Ok(updated)
}

async fn persist_canvas_assembly_output<B, R>(
    state: &AppState,
    record_id: i64,
    build_output: B,
    render: R,
) -> Result<(String, i64), String>
where
    B: FnOnce() -> Result<PathBuf, String>,
    R: FnOnce(&Path) -> Result<(), String>,
{
    let result = build_output().and_then(|output| {
        persist_created_media_file(
            state.data_dir(),
            &output,
            render,
            "画布成片文件",
            "画布成片",
            "画布成片文件",
        )
    });
    match result {
        Ok(result) => Ok(result),
        Err(error) => {
            let mark_failed_error = state
                .video_record_failure_application()
                .mark_error_status_and_prompt(record_id, None, "failed", &error)
                .await
                .map_err(crate::actions::app_error_message)
                .err();
            Err(append_secondary_error(error, mark_failed_error))
        }
    }
}
