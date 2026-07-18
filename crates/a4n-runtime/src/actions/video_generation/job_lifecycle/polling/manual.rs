use a4n_orchestrator::workflow::WORKFLOW_STATUS_FAILED;
use a4n_queue::priority::TaskPriority;
use a4n_video::VideoJobState;
use tokio_util::sync::CancellationToken;

use super::super::completion::{
    mark_canvas_video_workflow_from_video, mark_video_failed_and_update_prompt,
    save_completed_provider_result_for_video,
};
use super::super::context::{active_provider_job_id, provider_poll_config_id};
use super::super::workflow::record_video_provider_progress;
use super::provider::poll_provider_job;
use super::status::mark_video_job_waiting;
use crate::AppState;
use crate::actions::video_generation::config_state::resolve_video_config;
use crate::actions::video_generation::media::prompt_id_from_video;
use crate::actions::video_generation::{
    GeneratedVideoSummary, video_sidecar_root_project_id, video_to_summary,
};

pub(in crate::actions::video_generation) async fn poll_video_job(
    state: &AppState,
    video_id: i64,
) -> Result<GeneratedVideoSummary, String> {
    let video = state
        .orchestrator
        .generated_video_by_id(video_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .ok_or_else(|| "视频不存在".to_string())?;
    let Some(job_id) = active_provider_job_id(video.provider_job_id.as_deref(), &video.status)
    else {
        return Ok(video_to_summary(&video));
    };
    let job_id = job_id.to_string();
    let cfg_id = provider_poll_config_id(&video)?;
    let cfg = resolve_video_config(state, cfg_id).await?;
    let root_project_id = video_sidecar_root_project_id(state, video.project_id).await?;
    let workflow_run_id =
        record_video_provider_progress(state, &video, 1, 2, "正在恢复视频 provider 轮询").await?;
    match poll_provider_job(
        state,
        &cfg,
        video.project_id,
        root_project_id,
        &job_id,
        TaskPriority::Low,
        Some(workflow_run_id),
        CancellationToken::new(),
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
