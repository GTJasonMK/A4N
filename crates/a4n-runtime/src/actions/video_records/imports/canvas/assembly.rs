use std::collections::HashSet;

use super::super::prelude::{
    AppState, AssemblyClip, CreateVideoRecord, GeneratedVideoSummary, Path, PathBuf, Utc, access,
    append_secondary_error, canonicalize_data_file_path, json, persist_created_media_file,
    run_ffmpeg_assemble_videos, video_to_summary,
};
use super::super::processing::canvas_assembly_manifest;
use super::super::source::{import_project_video_output_path, mark_imported_video_completed};

pub async fn assemble_canvas_videos(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    video_ids: Vec<i64>,
    title: Option<String>,
) -> Result<GeneratedVideoSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    if video_ids.is_empty() {
        return Err("AssembleNode 至少需要一个视频片段".to_string());
    }
    let mut seen = HashSet::new();
    let mut clips = Vec::with_capacity(video_ids.len());
    for (index, video_id) in video_ids.iter().copied().enumerate() {
        if !seen.insert(video_id) {
            return Err(format!("AssembleNode 视频片段重复: {video_id}"));
        }
        let video = state
            .orchestrator
            .generated_video_by_id(video_id)
            .await
            .map_err(crate::actions::app_error_message)?
            .ok_or_else(|| "视频不存在".to_string())?;
        if video.project_id != project_id {
            return Err(format!("视频 #{video_id} 不属于项目 {project_id}"));
        }
        if video.chapter_id.is_some() {
            return Err(format!("AssembleNode 只支持项目级画布视频: {video_id}"));
        }
        if video.status != "completed" {
            return Err(format!("视频 #{video_id} 尚未完成，不能成片"));
        }
        let file_path = video
            .file_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("视频 #{video_id} 缺少本地文件"))?;
        let path = canonicalize_data_file_path(
            &state.config.data_dir,
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
    let first = clips
        .first()
        .ok_or_else(|| "AssembleNode 至少需要一个视频片段".to_string())?;
    let width = first.width.max(1);
    let height = first.height.max(1);
    let fps = first.fps.max(1);
    let duration_secs = clips
        .iter()
        .map(|clip| clip.duration_secs.max(0.0))
        .sum::<f32>();
    let title = title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("画布成片")
        .to_string();
    let metadata = json!({
        "canvas_assembly": {
            "source_video_ids": video_ids,
            "clip_count": clips.len(),
            "assembled_at": Utc::now().to_rfc3339(),
        },
        "source": "video_canvas",
    });
    let record = state
        .orchestrator
        .create_video_record(CreateVideoRecord {
            project_id,
            chapter_id: None,
            video_type: "canvas_assembly".to_string(),
            workflow_mode: Some("canvas_assembly".to_string()),
            prompt: title,
            negative_prompt: None,
            model: "ffmpeg".to_string(),
            width,
            height,
            duration_secs,
            fps,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(metadata),
        })
        .await
        .map_err(crate::actions::app_error_message)?;
    let manifest = canvas_assembly_manifest(project_id, &clips, width, height, fps)?;
    let (output, output_path_str, file_size) = persist_canvas_assembly_output(
        state,
        record.id,
        || import_project_video_output_path(state, project_id, record.id, "canvas_assembly", "mp4"),
        |output| run_ffmpeg_assemble_videos(&clips, output, width, height, fps, &manifest),
    )
    .await?;
    let updated = mark_imported_video_completed(
        state,
        record.id,
        &output,
        &output_path_str,
        None,
        file_size,
        None,
        None,
        None,
        "更新画布成片记录失败",
        "画布成片文件",
        "画布成片缩略图",
    )
    .await?;
    Ok(video_to_summary(&updated))
}

async fn persist_canvas_assembly_output<B, R>(
    state: &AppState,
    record_id: i64,
    build_output: B,
    render: R,
) -> Result<(PathBuf, String, i64), String>
where
    B: FnOnce() -> Result<PathBuf, String>,
    R: FnOnce(&Path) -> Result<(), String>,
{
    let result = build_output().and_then(|output| {
        persist_created_media_file(
            &state.config.data_dir,
            &output,
            render,
            "画布成片文件",
            "画布成片",
            "画布成片文件",
        )
        .map(|(relative_path, file_size)| (output, relative_path, file_size))
    });
    match result {
        Ok(result) => Ok(result),
        Err(error) => {
            let mark_failed_error = state
                .orchestrator
                .mark_generated_video_error_status_and_prompt(record_id, None, "failed", &error)
                .await
                .map_err(crate::actions::app_error_message)
                .err();
            Err(append_secondary_error(error, mark_failed_error))
        }
    }
}
