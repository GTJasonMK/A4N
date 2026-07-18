use a4n_core::models::ai::{generated_video, video_gen_config};
use a4n_orchestrator::workflow::WORKFLOW_STATUS_FAILED;
use a4n_queue::priority::TaskPriority;
use a4n_video::VideoJobState;
use tokio::time::{Duration, Instant};

use super::super::super::completion::{
    mark_canvas_video_workflow_from_video, mark_video_failed_and_update_prompt,
    save_completed_provider_result_for_video_with_canvas_workflow_mode,
};
use super::super::super::workflow::{
    canvas_parent_workflow_is_cancelled, interrupt_video_provider_polling,
    record_video_provider_progress,
};
use super::super::provider::poll_provider_job;
use super::super::status::mark_video_job_waiting;
use crate::AppState;
use crate::actions::video_generation::media::prompt_id_from_video;
use crate::actions::video_generation::{
    VIDEO_GENERATION_PROGRESS_TOTAL, VIDEO_JOB_MAX_CONSECUTIVE_POLL_ERRORS,
    VIDEO_JOB_MAX_WAIT_SECS, VIDEO_JOB_POLL_INTERVAL_SECS, VideoGenerationProgress,
    video_sidecar_root_project_id,
};

pub(super) async fn wait_for_video_job_completion_inner(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    mut video: generated_video::Model,
    cfg: &video_gen_config::Model,
    job_id: &str,
    finalize_canvas_workflow: bool,
) -> Result<generated_video::Model, String> {
    let root_project_id = video_sidecar_root_project_id(state, video.project_id).await?;
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

        poll_count = poll_count
            .checked_add(1)
            .ok_or_else(|| "视频 provider 轮询次数超过 u32 上限".to_string())?;
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
            root_project_id,
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
                consecutive_errors = consecutive_errors
                    .checked_add(1)
                    .ok_or_else(|| "视频 provider 连续轮询错误次数超过 u32 上限".to_string())?;
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
