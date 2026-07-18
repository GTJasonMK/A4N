use crate::AppState;
use crate::actions::video_generation::contracts::{GeneratedVideoSummary, video_to_summary};

pub(in crate::actions::video_generation) async fn mark_video_generation_failed(
    state: &AppState,
    video_id: i64,
    prompt_id: Option<i64>,
    message: &str,
) -> Result<GeneratedVideoSummary, String> {
    let updated = state
        .orchestrator
        .mark_generated_video_error_status_and_prompt(video_id, prompt_id, "failed", message)
        .await
        .map_err(crate::actions::app_error_message)?;
    fail_video_generation_runs(state, video_id, message).await?;
    Ok(video_to_summary(&updated))
}

pub(in crate::actions::video_generation) async fn fail_video_generation_runs(
    state: &AppState,
    video_id: i64,
    message: &str,
) -> Result<(), String> {
    state
        .orchestrator
        .fail_generation_runs_for_target("generated_video", video_id, message.to_string(), None)
        .await
        .map_err(crate::actions::app_error_message)
        .and_then(|runs| {
            ensure_provider_generation_run_updated(
                video_id,
                runs.len(),
                "标记 provider 视频生成失败",
            )
        })?;
    Ok(())
}

pub(in crate::actions::video_generation) fn ensure_provider_generation_run_updated(
    video_id: i64,
    updated_run_count: usize,
    action: &str,
) -> Result<(), String> {
    if updated_run_count == 0 {
        return Err(format!(
            "{action}失败：视频 #{video_id} 缺少可更新的 generation_run"
        ));
    }
    Ok(())
}
