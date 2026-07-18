use super::failure::{ensure_provider_generation_run_updated, mark_video_generation_failed};
use crate::AppState;
use crate::actions::video_generation::common::VIDEO_GENERATION_PROGRESS_TOTAL;
use crate::actions::video_generation::contracts::{
    GeneratedVideoSummary, VideoGenerationProgress, video_to_summary,
};
use crate::actions::video_generation::job_lifecycle::{
    normalized_provider_job_id, wait_for_canvas_graph_video_job_completion,
};
use crate::actions::video_generation::media::required_video_file_extension;
use crate::file_utils::{data_dir_relative_file_path, remove_created_file_on_error};
use a4n_core::files::paths::ProjectPaths;
use a4n_core::models::ai::{generated_video, video_gen_config};
use a4n_video::VideoGenerationResult;
use serde_json::json;
use std::path::PathBuf;

pub(in crate::actions::video_generation) async fn handle_canvas_video_provider_result(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    video: generated_video::Model,
    cfg: &video_gen_config::Model,
    result: Result<VideoGenerationResult, String>,
) -> Result<GeneratedVideoSummary, String> {
    match result {
        Ok(result) if !result.bytes.is_empty() => {
            if let Err(msg) = progress.check_cancelled("画布视频生成已停止") {
                return mark_video_generation_failed(state, video.id, None, &msg).await;
            }
            progress.progress(
                6,
                VIDEO_GENERATION_PROGRESS_TOTAL,
                "视频模型已返回，正在保存画布视频文件",
            )?;
            let updated = save_completed_canvas_video_result(state, &video, result).await?;
            progress.progress(7, VIDEO_GENERATION_PROGRESS_TOTAL, "画布视频文件已保存")?;
            Ok(video_to_summary(&updated))
        }
        Ok(result) => {
            if let Some(job_id) =
                normalized_provider_job_id(result.provider_job_id.as_deref()).map(str::to_string)
            {
                progress.progress(
                    4,
                    VIDEO_GENERATION_PROGRESS_TOTAL,
                    "视频模型已提交异步任务，画布视频记录已标记为运行中",
                )?;
                let updated = state
                    .orchestrator
                    .mark_generated_video_job_and_prompt(
                        video.id,
                        &job_id,
                        Some(result.metadata),
                        None,
                    )
                    .await
                    .map_err(crate::actions::app_error_message)?;
                let final_video = wait_for_canvas_graph_video_job_completion(
                    state, progress, updated, cfg, &job_id,
                )
                .await?;
                Ok(video_to_summary(&final_video))
            } else {
                mark_video_generation_failed(
                    state,
                    video.id,
                    None,
                    "画布视频 provider 未返回文件或任务 ID",
                )
                .await
            }
        }
        Err(err) => mark_video_generation_failed(state, video.id, None, &err).await,
    }
}

pub(in crate::actions::video_generation) async fn save_completed_canvas_video_result(
    state: &AppState,
    video: &generated_video::Model,
    result: VideoGenerationResult,
) -> Result<generated_video::Model, String> {
    if result.bytes.is_empty() {
        return state
            .orchestrator
            .mark_generated_video_error_status_and_prompt(
                video.id,
                None,
                "failed",
                "画布视频 provider 返回了空文件，无法保存生成结果",
            )
            .await
            .map_err(crate::actions::app_error_message);
    }
    let path = match save_canvas_video_bytes(
        state,
        video.project_id,
        video.id,
        &result.bytes,
        &result.format,
    ) {
        Ok(path) => path,
        Err(msg) => {
            return state
                .orchestrator
                .mark_generated_video_error_status_and_prompt(video.id, None, "failed", &msg)
                .await
                .map_err(crate::actions::app_error_message);
        }
    };
    let persisted_path = path.clone();
    let updated = remove_created_file_on_error(&path, "已写出的画布视频文件", async move {
        let video_path_str =
            data_dir_relative_file_path(&state.config.data_dir, &persisted_path, "画布视频文件")?;
        state
            .orchestrator
            .mark_generated_video_completed_and_link_prompt(
                video.id,
                &video_path_str,
                None,
                result.bytes.len() as i64,
                Some(json!({
                    "provider": result.metadata,
                    "canvas_video": true,
                })),
                None,
            )
            .await
            .map_err(|err| format!("画布视频已生成但记录完成状态失败：{err}"))
    })
    .await?;
    state
        .orchestrator
        .complete_generation_runs_for_target(
            "generated_video",
            video.id,
            Some(json!({
                "generated_video_id": video.id,
                "file_path": updated.file_path,
                "thumbnail_path": updated.thumbnail_path,
                "file_size": updated.file_size,
                "status": updated.status,
                "provider_result": updated.metadata_json,
            })),
        )
        .await
        .map_err(crate::actions::app_error_message)
        .and_then(|runs| {
            ensure_provider_generation_run_updated(video.id, runs.len(), "完成 provider 视频生成")
        })?;
    Ok(updated)
}

fn save_canvas_video_bytes(
    state: &AppState,
    project_id: i64,
    video_id: i64,
    bytes: &[u8],
    format: &str,
) -> Result<PathBuf, String> {
    let extension = required_video_file_extension(format)?;
    let dir = ProjectPaths::new(&state.config.data_dir, project_id).videos_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("无法创建画布视频目录 {}：{err}", dir.display()))?;
    let path = dir.join(format!("canvas_video_{video_id}.{extension}"));
    std::fs::write(&path, bytes).map_err(|err| format!("无法写入画布视频：{err}"))?;
    Ok(path)
}
