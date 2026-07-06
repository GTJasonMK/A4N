use super::config_state::resolve_video_config;
use super::{
    GeneratedVideoSummary, VIDEO_GENERATION_PROGRESS_TOTAL, VIDEO_JOB_MAX_CONSECUTIVE_POLL_ERRORS,
    VIDEO_JOB_MAX_WAIT_SECS, VIDEO_JOB_POLL_INTERVAL_SECS, VideoGenerationProgress,
    ensure_provider_generation_run_updated, fail_video_generation_runs,
    save_completed_canvas_video_result, video_to_summary,
};
use crate::AppState;
use crate::actions::image_generation::run_queued_request_with_cancel_and_workflow;
use crate::file_utils::data_dir_relative_file_path;
use a4n_core::models::ai::{generated_video, video_gen_config};
use a4n_core::models::generation::workflow_run;
use a4n_engine::queue::priority::{TaskPriority, TaskType as QueueTaskType};
use a4n_engine::video::{VideoGenerationResult, VideoJobState};
use a4n_services::generation::AttachGenerationRunsToWorkflowInput;
use a4n_services::source::source_content_sha256;
use a4n_services::video::pick_provider_for_config;
use a4n_services::workflow::{
    UpsertWorkflowCheckpointInput, UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED,
    WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_INTERRUPTED,
    WORKFLOW_STATUS_RUNNING, WorkflowProgressEventInput,
};
use sea_orm::EntityTrait;
use std::path::Path;
use tokio::time::{Duration, Instant};

use super::media::{
    append_secondary_error, best_effort_video_thumbnail_path, capture_video_tail_frame_asset,
    combine_optional_errors, delete_video_tail_frame_assets, prompt_id_from_video,
    remove_partial_video_file, save_video_bytes,
};
use serde_json::{Value, json};

pub(super) async fn wait_for_video_job_completion(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    video: generated_video::Model,
    cfg: &video_gen_config::Model,
    job_id: &str,
) -> Result<generated_video::Model, String> {
    wait_for_video_job_completion_inner(state, progress, video, cfg, job_id, true).await
}

pub(super) async fn wait_for_canvas_graph_video_job_completion(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    video: generated_video::Model,
    cfg: &video_gen_config::Model,
    job_id: &str,
) -> Result<generated_video::Model, String> {
    wait_for_video_job_completion_inner(state, progress, video, cfg, job_id, false).await
}

async fn wait_for_video_job_completion_inner(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    mut video: generated_video::Model,
    cfg: &video_gen_config::Model,
    job_id: &str,
    finalize_canvas_workflow: bool,
) -> Result<generated_video::Model, String> {
    let started = Instant::now();
    let timeout = Duration::from_secs(VIDEO_JOB_MAX_WAIT_SECS);
    let interval = Duration::from_secs(VIDEO_JOB_POLL_INTERVAL_SECS);
    let mut poll_count = 0_u32;
    let mut consecutive_errors = 0_u32;

    loop {
        if let Err(msg) =
            progress.check_cancelled("视频生成已停止，供应商任务仍在处理中，可稍后恢复轮询")
        {
            interrupt_video_provider_polling(state, &video, &msg).await?;
            return Err(msg);
        }
        if canvas_parent_workflow_is_cancelled(state, &video).await? {
            let msg = "画布运行已取消，视频 provider 轮询已停止".to_string();
            interrupt_video_provider_polling(state, &video, &msg).await?;
            return Err(msg);
        }
        if started.elapsed() >= timeout {
            let msg = format!(
                "视频任务 #{} 等待超过 {} 分钟仍未完成，已保留 provider_job_id={}，可稍后恢复轮询",
                video.id,
                VIDEO_JOB_MAX_WAIT_SECS / 60,
                job_id
            );
            interrupt_video_provider_polling(state, &video, &msg).await?;
            return Err(msg);
        }

        poll_count = poll_count.saturating_add(1);
        let poll_message = format!("视频模型处理中，正在第 {poll_count} 次查询结果");
        progress.progress(5, VIDEO_GENERATION_PROGRESS_TOTAL, &poll_message)?;
        let workflow_run_id = record_video_provider_progress(
            state,
            &video,
            5,
            VIDEO_GENERATION_PROGRESS_TOTAL,
            &poll_message,
        )
        .await?;

        match poll_provider_job(
            state,
            cfg,
            video.project_id,
            job_id,
            TaskPriority::Low,
            Some(workflow_run_id),
            progress.cancel_token(),
        )
        .await
        {
            Ok(VideoJobState::Completed(result)) => {
                if canvas_parent_workflow_is_cancelled(state, &video).await? {
                    let msg = "画布运行已取消，视频 provider 结果不再写入画布运行".to_string();
                    interrupt_video_provider_polling(state, &video, &msg).await?;
                    return Err(msg);
                }
                progress.progress(
                    6,
                    VIDEO_GENERATION_PROGRESS_TOTAL,
                    "视频模型已完成，正在保存文件",
                )?;
                let updated = save_completed_provider_result_for_video_with_canvas_workflow_mode(
                    state,
                    &video,
                    result,
                    finalize_canvas_workflow,
                )
                .await?;
                progress.progress(7, VIDEO_GENERATION_PROGRESS_TOTAL, "视频文件已保存")?;
                return Ok(updated);
            }
            Ok(VideoJobState::Failed(msg)) => {
                let updated = mark_video_failed_and_update_prompt(
                    state,
                    video.id,
                    prompt_id_from_video(&video),
                    &msg,
                )
                .await?;
                if finalize_canvas_workflow {
                    mark_canvas_video_workflow_from_video(
                        state,
                        &updated,
                        WORKFLOW_STATUS_FAILED,
                        Some(&msg),
                    )
                    .await?;
                }
                return Ok(updated);
            }
            Ok(VideoJobState::Pending) => {
                consecutive_errors = 0;
                video = mark_video_job_waiting(
                    state,
                    video.id,
                    prompt_id_from_video(&video),
                    "pending",
                )
                .await?;
            }
            Ok(VideoJobState::Running) => {
                consecutive_errors = 0;
                video = mark_video_job_waiting(
                    state,
                    video.id,
                    prompt_id_from_video(&video),
                    "running",
                )
                .await?;
            }
            Err(err) => {
                consecutive_errors = consecutive_errors.saturating_add(1);
                if consecutive_errors >= VIDEO_JOB_MAX_CONSECUTIVE_POLL_ERRORS {
                    let msg = format!(
                        "视频任务 #{} 连续 {} 次查询失败，已保留 provider_job_id={}，可稍后恢复轮询。最后一次错误：{}",
                        video.id, consecutive_errors, job_id, err
                    );
                    interrupt_video_provider_polling(state, &video, &msg).await?;
                    return Err(msg);
                }
                progress.progress(
                    5,
                    VIDEO_GENERATION_PROGRESS_TOTAL,
                    &format!(
                        "视频任务查询失败，将继续重试（{}/{}）：{}",
                        consecutive_errors, VIDEO_JOB_MAX_CONSECUTIVE_POLL_ERRORS, err
                    ),
                )?;
            }
        }

        let remaining = timeout.saturating_sub(started.elapsed());
        let sleep_for = remaining.min(interval);
        let cancel_token = progress.cancel_token();
        tokio::select! {
            _ = cancel_token.cancelled() => {
                let msg = "视频生成已停止，供应商任务仍在处理中，可稍后恢复轮询".to_string();
                interrupt_video_provider_polling(state, &video, &msg).await?;
                return Err(msg);
            }
            _ = tokio::time::sleep(sleep_for) => {}
        }
    }
}

pub(super) async fn poll_video_job(
    state: &AppState,
    video_id: i64,
) -> Result<GeneratedVideoSummary, String> {
    let video = state
        .video
        .get_video(video_id)
        .await
        .map_err(|e| e.to_string())?;
    let Some(job_id) = active_provider_job_id(video.provider_job_id.as_deref(), &video.status)
    else {
        return Ok(video_to_summary(&video));
    };
    let job_id = job_id.to_string();
    let cfg_id = provider_poll_config_id(&video)?;
    let cfg = resolve_video_config(state, cfg_id).await?;
    let workflow_run_id =
        record_video_provider_progress(state, &video, 1, 2, "正在恢复视频 provider 轮询").await?;
    match poll_provider_job(
        state,
        &cfg,
        video.project_id,
        &job_id,
        TaskPriority::Low,
        Some(workflow_run_id),
        tokio_util::sync::CancellationToken::new(),
    )
    .await?
    {
        VideoJobState::Completed(result) => {
            let updated = save_completed_provider_result_for_video(state, &video, result).await?;
            Ok(video_to_summary(&updated))
        }
        VideoJobState::Failed(msg) => {
            let updated = mark_video_failed_and_update_prompt(
                state,
                video.id,
                prompt_id_from_video(&video),
                &msg,
            )
            .await?;
            mark_canvas_video_workflow_from_video(
                state,
                &updated,
                WORKFLOW_STATUS_FAILED,
                Some(&msg),
            )
            .await?;
            Ok(video_to_summary(&updated))
        }
        VideoJobState::Pending => {
            let updated =
                mark_video_job_waiting(state, video.id, prompt_id_from_video(&video), "pending")
                    .await?;
            Ok(video_to_summary(&updated))
        }
        VideoJobState::Running => {
            let updated =
                mark_video_job_waiting(state, video.id, prompt_id_from_video(&video), "running")
                    .await?;
            Ok(video_to_summary(&updated))
        }
    }
}

async fn save_completed_provider_result_for_video(
    state: &AppState,
    video: &generated_video::Model,
    result: VideoGenerationResult,
) -> Result<generated_video::Model, String> {
    save_completed_provider_result_for_video_with_canvas_workflow_mode(state, video, result, true)
        .await
}

async fn save_completed_provider_result_for_video_with_canvas_workflow_mode(
    state: &AppState,
    video: &generated_video::Model,
    result: VideoGenerationResult,
    finalize_canvas_workflow: bool,
) -> Result<generated_video::Model, String> {
    if is_canvas_video_record(video) {
        let updated = save_completed_canvas_video_result(state, video, result).await?;
        mark_video_provider_workflow(state, &updated, WORKFLOW_STATUS_COMPLETED, false, None)
            .await?;
        if finalize_canvas_workflow {
            mark_canvas_video_workflow_from_video(state, &updated, WORKFLOW_STATUS_COMPLETED, None)
                .await?;
        }
        Ok(updated)
    } else {
        save_completed_video_result(state, video, prompt_id_from_video(video), result).await
    }
}

async fn mark_canvas_video_workflow_from_video(
    state: &AppState,
    video: &generated_video::Model,
    status: &str,
    error_message: Option<&str>,
) -> Result<(), String> {
    let Some(canvas) = canvas_video_workflow_context(video)? else {
        return Ok(());
    };
    let workflow_run_id = canvas.workflow_run_id;
    let graph_id = canvas.graph_id;
    let node_id = canvas.node_id;
    let step_key = format!("canvas_node:{node_id}");
    state
        .workflow_run
        .upsert_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id,
            step_key,
            step_order: 0,
            status: status.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: Some(json!({
                "generated_video_id": video.id,
                "provider_job_id": video.provider_job_id,
                "graph_id": graph_id,
                "node_id": node_id,
            })),
            checkpoint_json: None,
            artifacts_json: Some(json!({
                "generated_video_id": video.id,
                "status": video.status,
                "file_path": video.file_path,
                "thumbnail_path": video.thumbnail_path,
                "provider_job_id": video.provider_job_id,
            })),
            generation_run_ids_json: None,
            error_message: error_message.map(str::to_string),
            completed: matches!(status, WORKFLOW_STATUS_COMPLETED | WORKFLOW_STATUS_FAILED),
        })
        .await
        .map_err(|err| err.to_string())?;
    state
        .workflow_run
        .mark_status(
            workflow_run_id,
            status,
            status == WORKFLOW_STATUS_FAILED,
            error_message,
        )
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn is_canvas_video_record(video: &generated_video::Model) -> bool {
    video.chapter_id.is_none()
        && matches!(
            video.workflow_mode.as_deref(),
            Some("canvas_video" | "canvas_reference_media" | "canvas_trim" | "canvas_assembly")
        )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CanvasVideoWorkflowContext {
    workflow_run_id: i64,
    graph_id: i64,
    node_id: i64,
}

fn provider_poll_config_id(video: &generated_video::Model) -> Result<Option<i64>, String> {
    let raw_config_id = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("config_id"));
    if !is_canvas_video_record(video) {
        return raw_config_id
            .map(|value| positive_provider_poll_config_id(video, value))
            .transpose();
    }
    let config_id = raw_config_id.ok_or_else(|| {
        format!(
            "画布视频 #{} 缺少 metadata_json.config_id，不能恢复 provider 轮询",
            video.id
        )
    })?;
    positive_provider_poll_config_id(video, config_id).map(Some)
}

fn positive_provider_poll_config_id(
    video: &generated_video::Model,
    value: &serde_json::Value,
) -> Result<i64, String> {
    let config_id = value.as_i64().ok_or_else(|| {
        format!(
            "视频 #{} 的 metadata_json.config_id 必须是正整数，不能恢复 provider 轮询",
            video.id
        )
    })?;
    if config_id <= 0 {
        return Err(format!(
            "视频 #{} 的 metadata_json.config_id 必须是正整数，不能恢复 provider 轮询",
            video.id
        ));
    }
    Ok(config_id)
}

fn canvas_video_workflow_context(
    video: &generated_video::Model,
) -> Result<Option<CanvasVideoWorkflowContext>, String> {
    if !is_canvas_video_record(video) {
        return Ok(None);
    }
    let metadata = video.metadata_json.as_ref().ok_or_else(|| {
        format!(
            "画布视频 #{} 缺少 metadata_json，不能同步 canvas workflow",
            video.id
        )
    })?;
    let canvas = metadata.get("canvas").ok_or_else(|| {
        format!(
            "画布视频 #{} 缺少 metadata_json.canvas，不能同步 canvas workflow",
            video.id
        )
    })?;
    let context = CanvasVideoWorkflowContext {
        workflow_run_id: required_canvas_video_context_id(video.id, canvas, "workflow_run_id")?,
        graph_id: required_canvas_video_context_id(video.id, canvas, "graph_id")?,
        node_id: required_canvas_video_context_id(video.id, canvas, "node_id")?,
    };
    Ok(Some(context))
}

fn required_canvas_video_context_id(
    video_id: i64,
    canvas: &serde_json::Value,
    field: &str,
) -> Result<i64, String> {
    let value = canvas
        .get(field)
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| {
            format!(
                "画布视频 #{video_id} 缺少 metadata_json.canvas.{field}，不能同步 canvas workflow"
            )
        })?;
    if value <= 0 {
        return Err(format!(
            "画布视频 #{video_id} 的 metadata_json.canvas.{field} 必须是正整数"
        ));
    }
    Ok(value)
}

async fn poll_provider_job(
    state: &AppState,
    cfg: &video_gen_config::Model,
    project_id: i64,
    job_id: &str,
    priority: TaskPriority,
    workflow_run_id: Option<i64>,
    cancel_token: tokio_util::sync::CancellationToken,
) -> Result<VideoJobState, String> {
    let provider = pick_provider_for_config(cfg)?;
    run_queued_request_with_cancel_and_workflow(
        state,
        QueueTaskType::VideoPoll,
        priority,
        Some(project_id),
        workflow_run_id,
        cancel_token,
        || async { provider.poll(job_id).await.map_err(|e| e.to_string()) },
    )
    .await
}

async fn mark_video_job_waiting(
    state: &AppState,
    video_id: i64,
    prompt_id: Option<i64>,
    status: &str,
) -> Result<generated_video::Model, String> {
    let updated = state
        .video
        .update_waiting_status_and_prompt(video_id, prompt_id, status)
        .await
        .map_err(|e| e.to_string())?;
    mark_video_provider_workflow(state, &updated, WORKFLOW_STATUS_RUNNING, true, None).await?;
    Ok(updated)
}

pub(super) async fn save_completed_video_result(
    state: &AppState,
    video: &generated_video::Model,
    prompt_id: Option<i64>,
    result: VideoGenerationResult,
) -> Result<generated_video::Model, String> {
    let prompt_id = prompt_id.or_else(|| prompt_id_from_video(video));
    if result.bytes.is_empty() {
        let updated = mark_video_failed_and_update_prompt(
            state,
            video.id,
            prompt_id,
            "视频 provider 返回了空文件，无法保存生成结果",
        )
        .await?;
        return Ok(updated);
    }

    let chapter_id = required_video_chapter_id(video.chapter_id, video.id)?;
    let path = match save_video_bytes(
        state,
        video.project_id,
        chapter_id,
        video.id,
        &result.bytes,
        &result.format,
    ) {
        Ok(path) => path,
        Err(msg) => {
            let updated =
                mark_video_failed_and_update_prompt(state, video.id, prompt_id, &msg).await?;
            return Ok(updated);
        }
    };
    let video_path_str = data_dir_relative_file_path(&state.config.data_dir, &path, "视频文件")?;
    let thumbnail_path = best_effort_video_thumbnail_path(
        &state.config.data_dir,
        video.project_id,
        chapter_id,
        video.id,
        &path,
    );
    let mut tail_frame_capture_error = None;
    let tail_frame_metadata = if should_capture_video_tail_frame(video) {
        match capture_video_tail_frame_asset(state, video, chapter_id, prompt_id, &path).await {
            Ok(metadata) => Some(metadata),
            Err(msg) => {
                if is_first_frame_chain_video(video) {
                    let msg = format!("视频已生成但首帧串联尾帧采集失败：{msg}");
                    let cleanup_error = cleanup_saved_video_outputs(
                        state,
                        video.id,
                        &path,
                        thumbnail_path.as_deref(),
                    )
                    .await;
                    let updated = mark_video_failed_and_update_prompt(
                        state,
                        video.id,
                        prompt_id,
                        &append_secondary_error(msg.clone(), cleanup_error.clone()),
                    )
                    .await?;
                    if let Some(cleanup_error) = cleanup_error {
                        return Err(append_secondary_error(msg, Some(cleanup_error)));
                    }
                    return Ok(updated);
                }
                tail_frame_capture_error = Some(msg);
                None
            }
        }
    } else {
        None
    };
    let completed_metadata = completed_provider_metadata(
        result.metadata,
        tail_frame_metadata,
        tail_frame_capture_error,
    );
    let updated = match state
        .video
        .mark_completed_and_link_prompt(
            video.id,
            &video_path_str,
            thumbnail_path.as_deref(),
            result.bytes.len() as i64,
            completed_metadata,
            prompt_id,
        )
        .await
    {
        Ok(updated) => updated,
        Err(err) => {
            let msg = format!("视频已生成但记录完成状态失败：{err}");
            let cleanup_error =
                cleanup_saved_video_outputs(state, video.id, &path, thumbnail_path.as_deref())
                    .await;
            let updated = mark_video_failed_and_update_prompt(
                state,
                video.id,
                prompt_id,
                &append_secondary_error(msg.clone(), cleanup_error.clone()),
            )
            .await?;
            if let Some(cleanup_error) = cleanup_error {
                return Err(append_secondary_error(msg, Some(cleanup_error)));
            }
            return Ok(updated);
        }
    };
    complete_video_generation_runs(state, &updated).await?;
    mark_video_provider_workflow(state, &updated, WORKFLOW_STATUS_COMPLETED, false, None).await?;
    Ok(updated)
}

async fn cleanup_saved_video_outputs(
    state: &AppState,
    video_id: i64,
    video_path: &Path,
    thumbnail_path: Option<&str>,
) -> Option<String> {
    combine_optional_errors(
        cleanup_saved_video_files(&state.config.data_dir, video_path, thumbnail_path),
        delete_video_tail_frame_assets(state, video_id).await.err(),
    )
}

fn cleanup_saved_video_files(
    data_dir: &Path,
    video_path: &Path,
    thumbnail_path: Option<&str>,
) -> Option<String> {
    combine_optional_errors(
        remove_partial_video_file(video_path, "已生成视频文件").err(),
        thumbnail_path
            .map(|path| data_dir.join(path))
            .and_then(|path| remove_partial_video_file(&path, "视频缩略图").err()),
    )
}

async fn mark_video_failed_and_update_prompt(
    state: &AppState,
    video_id: i64,
    prompt_id: Option<i64>,
    message: &str,
) -> Result<generated_video::Model, String> {
    let updated = state
        .video
        .mark_failed_and_update_prompt(video_id, prompt_id, message)
        .await
        .map_err(|e| e.to_string())?;
    fail_video_generation_runs(state, video_id, message).await?;
    mark_video_provider_workflow(
        state,
        &updated,
        WORKFLOW_STATUS_FAILED,
        false,
        Some(message),
    )
    .await?;
    Ok(updated)
}

async fn interrupt_video_provider_polling(
    state: &AppState,
    video: &generated_video::Model,
    message: &str,
) -> Result<generated_video::Model, String> {
    let updated = state
        .video
        .mark_interrupted_and_update_prompt(video.id, prompt_id_from_video(video), message)
        .await
        .map_err(|e| e.to_string())?;
    mark_video_provider_workflow(
        state,
        &updated,
        WORKFLOW_STATUS_INTERRUPTED,
        true,
        Some(message),
    )
    .await?;
    Ok(updated)
}

async fn canvas_parent_workflow_is_cancelled(
    state: &AppState,
    video: &generated_video::Model,
) -> Result<bool, String> {
    let Some(workflow_run_id) = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("canvas"))
        .and_then(|canvas| canvas.get("workflow_run_id"))
        .and_then(Value::as_i64)
    else {
        return Ok(false);
    };
    let Some(run) = workflow_run::Entity::find_by_id(workflow_run_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
    else {
        return Ok(false);
    };
    Ok(run.status == WORKFLOW_STATUS_CANCELLED)
}

async fn complete_video_generation_runs(
    state: &AppState,
    video: &generated_video::Model,
) -> Result<(), String> {
    state
        .generation_run
        .complete_runs_for_target(
            "generated_video",
            video.id,
            Some(json!({
                "generated_video_id": video.id,
                "file_path": video.file_path,
                "thumbnail_path": video.thumbnail_path,
                "file_size": video.file_size,
                "status": video.status,
                "provider_result": video.metadata_json,
            })),
        )
        .await
        .map_err(|err| err.to_string())
        .and_then(|runs| {
            ensure_provider_generation_run_updated(video.id, runs.len(), "完成 provider 视频生成")
        })?;
    Ok(())
}

pub(super) async fn mark_video_provider_workflow(
    state: &AppState,
    video: &generated_video::Model,
    status: &str,
    can_resume: bool,
    error_message: Option<&str>,
) -> Result<(), String> {
    let Some(job_id) = normalized_provider_job_id(video.provider_job_id.as_deref()) else {
        return Ok(());
    };
    let fingerprint = video_provider_workflow_fingerprint(video, job_id);
    let fingerprint_sha = serde_json::to_string(&fingerprint)
        .map(|value| source_content_sha256(&value))
        .map_err(|err| err.to_string())?;
    let run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: video_provider_workflow_business_key(video.id),
            workflow_kind: "video_provider_job".to_string(),
            project_id: Some(video.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("generated_video".to_string()),
            scope_id: Some(video.id),
            chapter_id: video.chapter_id,
            mode: video.workflow_mode.clone(),
            input_fingerprint_json: Some(fingerprint.clone()),
            status: status.to_string(),
            current_step_key: Some("provider_poll".to_string()),
            active_task_id: None,
            can_resume,
            resume_block_reason: (!can_resume && status != WORKFLOW_STATUS_COMPLETED).then(|| {
                error_message
                    .unwrap_or("该 provider 任务已进入终态，不能继续轮询")
                    .to_string()
            }),
            error_message: error_message.map(str::to_string),
        })
        .await
        .map_err(|err| err.to_string())?;
    state
        .workflow_run
        .upsert_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id: run.id,
            step_key: "provider_job".to_string(),
            step_order: 1,
            status: status.to_string(),
            input_fingerprint_sha256: Some(fingerprint_sha.clone()),
            input_fingerprint_json: Some(fingerprint.clone()),
            checkpoint_json: Some(json!({
                "provider_job_id": job_id,
                "generated_video_id": video.id,
                "status": video.status,
            })),
            artifacts_json: Some(provider_job_artifacts_json(video, job_id)),
            generation_run_ids_json: None,
            error_message: error_message.map(str::to_string),
            completed: is_terminal_provider_workflow_checkpoint_status(status),
        })
        .await
        .map_err(|err| err.to_string())?;
    state
        .generation_run
        .attach_runs_to_workflow_for_target(AttachGenerationRunsToWorkflowInput {
            target_table: "generated_video".to_string(),
            target_id: video.id,
            workflow_run_id: run.id,
            call_key: Some("provider_job".to_string()),
            input_fingerprint_sha256: Some(fingerprint_sha),
            input_fingerprint_json: Some(fingerprint),
            artifact_refs_json: Some(provider_job_artifacts_json(video, job_id)),
        })
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn provider_job_artifacts_json(video: &generated_video::Model, job_id: &str) -> serde_json::Value {
    json!({
        "targets": [{
            "target_table": "generated_video",
            "target_id": video.id,
            "target_kind": "video_provider_job",
            "role": "provider_job",
            "provider_job_id": job_id,
        }],
    })
}

fn is_terminal_provider_workflow_checkpoint_status(status: &str) -> bool {
    matches!(status, WORKFLOW_STATUS_COMPLETED | WORKFLOW_STATUS_FAILED)
}

async fn record_video_provider_progress(
    state: &AppState,
    video: &generated_video::Model,
    current: u32,
    total: u32,
    message: &str,
) -> Result<i64, String> {
    if is_terminal_generated_video_status(&video.status) {
        return Err("视频已进入终态，不能继续记录 provider 进度".to_string());
    }
    let Some(job_id) = normalized_provider_job_id(video.provider_job_id.as_deref()) else {
        return Err("视频缺少 provider_job_id，不能记录 provider 进度".to_string());
    };
    let run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: video_provider_workflow_business_key(video.id),
            workflow_kind: "video_provider_job".to_string(),
            project_id: Some(video.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("generated_video".to_string()),
            scope_id: Some(video.id),
            chapter_id: video.chapter_id,
            mode: video.workflow_mode.clone(),
            input_fingerprint_json: Some(video_provider_workflow_fingerprint(video, job_id)),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("provider_poll".to_string()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    state
        .workflow_run
        .record_progress_event(WorkflowProgressEventInput {
            workflow_run_id: run.id,
            task_id: None,
            current,
            total,
            message,
            meta: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    record_canvas_video_provider_progress(state, video, current, total, message).await?;
    Ok(run.id)
}

async fn record_canvas_video_provider_progress(
    state: &AppState,
    video: &generated_video::Model,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), String> {
    let Some(canvas) = canvas_video_workflow_context(video)? else {
        return Ok(());
    };
    let step_key = format!("canvas_node:{}", canvas.node_id);
    state
        .workflow_run
        .mark_current_step(canvas.workflow_run_id, &step_key)
        .await
        .map_err(|err| err.to_string())?;
    state
        .workflow_run
        .upsert_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id: canvas.workflow_run_id,
            step_key,
            step_order: 0,
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: Some(json!({
                "generated_video_id": video.id,
                "provider_job_id": video.provider_job_id,
                "graph_id": canvas.graph_id,
                "node_id": canvas.node_id,
            })),
            checkpoint_json: None,
            artifacts_json: Some(json!({
                "data_type": "video",
                "generated_video_id": video.id,
                "status": video.status,
                "provider_job_id": video.provider_job_id,
            })),
            generation_run_ids_json: None,
            error_message: None,
            completed: false,
        })
        .await
        .map_err(|err| err.to_string())?;
    state
        .workflow_run
        .record_progress_event(WorkflowProgressEventInput {
            workflow_run_id: canvas.workflow_run_id,
            task_id: None,
            current,
            total,
            message,
            meta: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn video_provider_workflow_business_key(video_id: i64) -> String {
    format!("video_provider_job:{video_id}")
}

fn video_provider_workflow_fingerprint(
    video: &generated_video::Model,
    job_id: &str,
) -> serde_json::Value {
    json!({
        "generated_video_id": video.id,
        "provider_job_id": job_id,
        "prompt_id": prompt_id_from_video(video),
        "config_id": video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("config_id"))
            .and_then(serde_json::Value::as_i64),
        "workflow_mode": video.workflow_mode,
    })
}

fn is_first_frame_chain_video(video: &generated_video::Model) -> bool {
    video.workflow_mode.as_deref() == Some("first_frame_chain")
}

fn should_capture_video_tail_frame(video: &generated_video::Model) -> bool {
    video.video_type == "chapter_video"
        && matches!(
            video.workflow_mode.as_deref(),
            Some("boundary_frames" | "first_frame_chain" | "chat_video")
        )
}

fn completed_provider_metadata(
    provider_metadata: Value,
    tail_frame_metadata: Option<Value>,
    tail_frame_capture_error: Option<String>,
) -> Option<Value> {
    if tail_frame_metadata.is_none() && tail_frame_capture_error.is_none() {
        return (!provider_metadata.is_null()).then_some(provider_metadata);
    }

    let mut metadata = serde_json::Map::new();
    if !provider_metadata.is_null() {
        metadata.insert("provider".to_string(), provider_metadata);
    }
    if let Some(tail_frame_metadata) = tail_frame_metadata {
        metadata.insert("tail_frame".to_string(), tail_frame_metadata);
    }
    if let Some(error) = tail_frame_capture_error {
        metadata.insert("tail_frame_capture_error".to_string(), Value::String(error));
    }
    Some(Value::Object(metadata))
}

fn active_provider_job_id<'a>(provider_job_id: Option<&'a str>, status: &str) -> Option<&'a str> {
    if is_terminal_generated_video_status(status) {
        return None;
    }
    normalized_provider_job_id(provider_job_id)
}

pub(super) fn has_active_provider_job(provider_job_id: Option<&str>, status: &str) -> bool {
    active_provider_job_id(provider_job_id, status).is_some()
}

pub(super) fn normalized_provider_job_id(provider_job_id: Option<&str>) -> Option<&str> {
    provider_job_id
        .map(str::trim)
        .filter(|job_id| !job_id.is_empty())
}

fn is_terminal_generated_video_status(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "cancelled")
}

fn required_video_chapter_id(chapter_id: Option<i64>, video_id: i64) -> Result<i64, String> {
    chapter_id.ok_or_else(|| format!("视频 #{video_id} 缺少 chapter_id，无法保存生成结果"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn video_for_tail_capture(
        video_type: &str,
        workflow_mode: Option<&str>,
    ) -> generated_video::Model {
        let now = Utc::now().naive_utc();
        generated_video::Model {
            id: 42,
            project_id: 1,
            chapter_id: Some(1),
            video_type: video_type.to_string(),
            workflow_mode: workflow_mode.map(ToString::to_string),
            file_path: None,
            thumbnail_path: None,
            prompt: "prompt".to_string(),
            negative_prompt: None,
            model: "model".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 5.0,
            fps: 24,
            file_size: None,
            source_video_id: None,
            source_image_id: None,
            status: "completed".to_string(),
            provider_job_id: None,
            error_message: None,
            metadata_json: None,
            is_favorite: false,
            created_at: now,
            updated_at: now,
        }
    }

    fn canvas_video_with_metadata(
        metadata_json: Option<serde_json::Value>,
    ) -> generated_video::Model {
        let mut video = video_for_tail_capture("canvas_video", Some("canvas_video"));
        video.chapter_id = None;
        video.status = "running".to_string();
        video.provider_job_id = Some("provider-job".to_string());
        video.metadata_json = metadata_json;
        video
    }

    #[test]
    fn required_video_chapter_id_rejects_missing_id() {
        let err = required_video_chapter_id(None, 42).expect_err("chapter id should be required");

        assert!(err.contains("视频 #42 缺少 chapter_id"));
    }

    #[test]
    fn tail_frame_capture_applies_to_chapter_video_workflows() {
        for workflow in ["boundary_frames", "first_frame_chain", "chat_video"] {
            assert!(should_capture_video_tail_frame(&video_for_tail_capture(
                "chapter_video",
                Some(workflow),
            )));
        }
        assert!(!should_capture_video_tail_frame(&video_for_tail_capture(
            "reference_media_video",
            Some("reference_media"),
        )));
        assert!(!should_capture_video_tail_frame(&video_for_tail_capture(
            "chapter_video",
            Some("reference_media"),
        )));
    }

    #[test]
    fn completed_provider_metadata_omits_null_provider_metadata() {
        assert_eq!(completed_provider_metadata(Value::Null, None, None), None);
        assert_eq!(
            completed_provider_metadata(json!({"job_id": "provider-job"}), None, None),
            Some(json!({"job_id": "provider-job"}))
        );

        assert_eq!(
            completed_provider_metadata(Value::Null, Some(json!({"image_id": 9})), None),
            Some(json!({"tail_frame": {"image_id": 9}}))
        );
        assert_eq!(
            completed_provider_metadata(
                Value::Null,
                Some(json!({"image_id": 9})),
                Some("tail capture failed".to_string()),
            ),
            Some(json!({
                "tail_frame": {"image_id": 9},
                "tail_frame_capture_error": "tail capture failed",
            }))
        );
        assert_eq!(
            completed_provider_metadata(
                json!({"job_id": "provider-job"}),
                Some(json!({"image_id": 9})),
                Some("tail capture failed".to_string()),
            ),
            Some(json!({
                "provider": {"job_id": "provider-job"},
                "tail_frame": {"image_id": 9},
                "tail_frame_capture_error": "tail capture failed",
            }))
        );
    }

    #[test]
    fn cleanup_saved_video_files_removes_video_and_thumbnail() {
        let root = std::env::temp_dir().join(format!(
            "a4n-video-cleanup-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let thumbnail_rel = "projects/1/chapter-1/thumb.jpg";
        let thumbnail_path = root.join(thumbnail_rel);
        let video_path = root.join("projects/1/chapter-1/video.mp4");
        std::fs::create_dir_all(thumbnail_path.parent().expect("thumbnail parent"))
            .expect("create thumbnail dir");
        std::fs::write(&thumbnail_path, b"thumbnail").expect("write thumbnail");
        std::fs::write(&video_path, b"video").expect("write video");

        let cleanup_error = cleanup_saved_video_files(&root, &video_path, Some(thumbnail_rel));

        assert_eq!(cleanup_error, None);
        assert!(!video_path.exists());
        assert!(!thumbnail_path.exists());
        std::fs::remove_dir_all(&root).expect("remove temp cleanup dir");
    }

    #[test]
    fn canvas_provider_poll_requires_explicit_config_id() {
        let chapter_video = video_for_tail_capture("chapter_video", Some("boundary_frames"));
        assert_eq!(
            provider_poll_config_id(&chapter_video).expect("chapter videos may use default config"),
            None
        );

        let err = provider_poll_config_id(&canvas_video_with_metadata(Some(json!({
            "canvas": {"workflow_run_id": 1, "graph_id": 2, "node_id": 3}
        }))))
        .expect_err("canvas provider poll must not fall back to default config");
        assert!(err.contains("缺少 metadata_json.config_id"));

        let err = provider_poll_config_id(&canvas_video_with_metadata(Some(json!({
            "config_id": 0,
            "canvas": {"workflow_run_id": 1, "graph_id": 2, "node_id": 3}
        }))))
        .expect_err("canvas provider poll should reject invalid config id");
        assert!(err.contains("metadata_json.config_id 必须是正整数"));

        assert_eq!(
            provider_poll_config_id(&canvas_video_with_metadata(Some(json!({
                "config_id": 9,
                "canvas": {"workflow_run_id": 1, "graph_id": 2, "node_id": 3}
            }))))
            .expect("valid canvas provider poll config should parse"),
            Some(9)
        );
    }

    #[test]
    fn chapter_provider_poll_rejects_malformed_config_id() {
        let mut video = video_for_tail_capture("chapter_video", Some("boundary_frames"));

        video.metadata_json = Some(json!({ "config_id": "9" }));
        let err = provider_poll_config_id(&video)
            .expect_err("string config_id must not fall back to default config");
        assert!(err.contains("metadata_json.config_id 必须是正整数"));

        video.metadata_json = Some(json!({ "config_id": 0 }));
        let err = provider_poll_config_id(&video)
            .expect_err("non-positive config_id must not reach config lookup");
        assert!(err.contains("metadata_json.config_id 必须是正整数"));

        video.metadata_json = Some(json!({ "config_id": 9 }));
        assert_eq!(
            provider_poll_config_id(&video).expect("positive config_id should be reused"),
            Some(9)
        );
    }

    #[test]
    fn canvas_provider_workflow_context_requires_canvas_metadata() {
        let chapter_video = video_for_tail_capture("chapter_video", Some("boundary_frames"));
        assert_eq!(
            canvas_video_workflow_context(&chapter_video)
                .expect("non-canvas videos do not need canvas workflow context"),
            None
        );

        let err = canvas_video_workflow_context(&canvas_video_with_metadata(None))
            .expect_err("canvas video should require metadata_json");
        assert!(err.contains("缺少 metadata_json"));

        let err = canvas_video_workflow_context(&canvas_video_with_metadata(Some(json!({
            "config_id": 9
        }))))
        .expect_err("canvas video should require canvas metadata object");
        assert!(err.contains("缺少 metadata_json.canvas"));

        let err = canvas_video_workflow_context(&canvas_video_with_metadata(Some(json!({
            "config_id": 9,
            "canvas": {"workflow_run_id": 1, "graph_id": 2}
        }))))
        .expect_err("canvas video should require node id");
        assert!(err.contains("metadata_json.canvas.node_id"));

        assert_eq!(
            canvas_video_workflow_context(&canvas_video_with_metadata(Some(json!({
                "config_id": 9,
                "canvas": {"workflow_run_id": 11, "graph_id": 22, "node_id": 33}
            }))))
            .expect("valid canvas workflow context should parse"),
            Some(CanvasVideoWorkflowContext {
                workflow_run_id: 11,
                graph_id: 22,
                node_id: 33,
            })
        );
    }

    #[test]
    fn terminal_video_status_matches_completed_failed_cancelled() {
        assert!(is_terminal_generated_video_status("completed"));
        assert!(is_terminal_generated_video_status("failed"));
        assert!(is_terminal_generated_video_status("cancelled"));
        assert!(!is_terminal_generated_video_status("running"));
        assert!(!is_terminal_generated_video_status("pending"));
        assert!(!is_terminal_generated_video_status("interrupted"));
    }

    #[test]
    fn active_provider_job_requires_non_terminal_status_and_nonblank_id() {
        assert_eq!(
            active_provider_job_id(Some(" job-1 "), "running"),
            Some("job-1")
        );
        assert_eq!(
            active_provider_job_id(Some("job-1"), "pending"),
            Some("job-1")
        );
        assert_eq!(
            active_provider_job_id(Some("job-1"), "interrupted"),
            Some("job-1")
        );
        assert_eq!(active_provider_job_id(Some("job-1"), "completed"), None);
        assert_eq!(active_provider_job_id(Some("job-1"), "failed"), None);
        assert_eq!(active_provider_job_id(Some("job-1"), "cancelled"), None);
        assert_eq!(active_provider_job_id(Some(" \t "), "running"), None);
        assert_eq!(active_provider_job_id(None, "running"), None);
    }

    #[test]
    fn has_active_provider_job_matches_lifecycle_status_semantics() {
        assert!(has_active_provider_job(Some(" job-1 "), "running"));
        assert!(has_active_provider_job(Some("job-1"), "pending"));
        assert!(has_active_provider_job(Some("job-1"), "interrupted"));
        assert!(!has_active_provider_job(Some("job-1"), "completed"));
        assert!(!has_active_provider_job(Some("job-1"), "failed"));
        assert!(!has_active_provider_job(Some("job-1"), "cancelled"));
        assert!(!has_active_provider_job(Some(" \t "), "running"));
        assert!(!has_active_provider_job(None, "running"));
    }

    #[test]
    fn provider_workflow_checkpoint_completed_only_for_terminal_provider_statuses() {
        assert!(is_terminal_provider_workflow_checkpoint_status(
            WORKFLOW_STATUS_COMPLETED
        ));
        assert!(is_terminal_provider_workflow_checkpoint_status(
            WORKFLOW_STATUS_FAILED
        ));
        assert!(!is_terminal_provider_workflow_checkpoint_status(
            WORKFLOW_STATUS_RUNNING
        ));
        assert!(!is_terminal_provider_workflow_checkpoint_status(
            WORKFLOW_STATUS_INTERRUPTED
        ));
    }

    #[test]
    fn normalized_provider_job_id_trims_and_rejects_blank_values() {
        assert_eq!(normalized_provider_job_id(Some(" job-1 ")), Some("job-1"));
        assert_eq!(normalized_provider_job_id(Some(" \n\t ")), None);
        assert_eq!(normalized_provider_job_id(Some("")), None);
        assert_eq!(normalized_provider_job_id(None), None);
    }
}
