use super::contracts::VideoProviderResultHandling;
use super::failure::mark_video_generation_failed;
use crate::AppState;
use crate::actions::video_generation::common::VIDEO_GENERATION_PROGRESS_TOTAL;
use crate::actions::video_generation::contracts::{
    GeneratedVideoSummary, VideoGenerationProgress, video_to_summary,
};
use crate::actions::video_generation::job_lifecycle::{
    normalized_provider_job_id, save_completed_video_result, wait_for_video_job_completion,
};
use a4n_core::models::ai::{generated_video, video_gen_config};
use a4n_video::VideoGenerationResult;

pub(in crate::actions::video_generation) async fn handle_video_provider_result(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    video: generated_video::Model,
    cfg: &video_gen_config::Model,
    result: Result<VideoGenerationResult, String>,
    handling: VideoProviderResultHandling<'_>,
) -> Result<GeneratedVideoSummary, String> {
    match result {
        Ok(result) if !result.bytes.is_empty() => {
            if let Some(message) = handling.completed_cancel_message
                && let Err(msg) = progress.check_cancelled(message)
            {
                return mark_video_generation_failed(state, video.id, handling.prompt_id, &msg)
                    .await;
            }
            progress.progress(
                6,
                VIDEO_GENERATION_PROGRESS_TOTAL,
                "视频模型已返回，正在保存文件",
            )?;
            let updated =
                save_completed_video_result(state, &video, handling.prompt_id, result).await?;
            progress.progress(7, VIDEO_GENERATION_PROGRESS_TOTAL, "视频文件已保存")?;
            Ok(video_to_summary(&updated))
        }
        Ok(result) => {
            if let Some(job_id) =
                normalized_provider_job_id(result.provider_job_id.as_deref()).map(str::to_string)
            {
                progress.progress(
                    4,
                    VIDEO_GENERATION_PROGRESS_TOTAL,
                    handling.async_job_progress_message,
                )?;
                let updated = state
                    .orchestrator
                    .mark_generated_video_job_and_prompt(
                        video.id,
                        &job_id,
                        Some(result.metadata),
                        handling.prompt_id,
                    )
                    .await
                    .map_err(crate::actions::app_error_message)?;
                let final_video =
                    wait_for_video_job_completion(state, progress, updated, cfg, &job_id).await?;
                Ok(video_to_summary(&final_video))
            } else {
                mark_video_generation_failed(
                    state,
                    video.id,
                    handling.prompt_id,
                    handling.missing_result_message,
                )
                .await
            }
        }
        Err(err) => mark_video_generation_failed(state, video.id, handling.prompt_id, &err).await,
    }
}
