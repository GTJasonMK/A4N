use crate::{AppState, actions::app_error_message};
use a4n_queue::priority::TaskQueue;

use super::super::{
    CancelWorkflowInput,
    access::{verify_task_access, verify_workflow_run_summary_access},
};

pub async fn cancel_workflow(
    state: &AppState,
    user_id: i64,
    input: CancelWorkflowInput,
) -> Result<(), String> {
    let has_task_id = input
        .task_id
        .as_deref()
        .is_some_and(|task_id| !task_id.trim().is_empty());
    if !has_task_id && input.workflow_run_id.is_none() {
        return Err("必须提供 task_id 或 workflow_run_id".to_string());
    }
    if let Some(task_id) = input
        .task_id
        .as_deref()
        .map(str::trim)
        .filter(|task_id| !task_id.is_empty())
    {
        verify_task_access(state, user_id, task_id).await?;
        if state.task_manager.cancel(task_id) {
            return Ok(());
        }
        state
            .task_queue
            .cancel(task_id)
            .await
            .map_err(|err| err.to_string())?;
    }
    if let Some(workflow_run_id) = input.workflow_run_id {
        cancel_workflow_run_for_user(state, user_id, workflow_run_id).await?;
    }
    Ok(())
}

async fn cancel_workflow_run_for_user(
    state: &AppState,
    user_id: i64,
    workflow_run_id: i64,
) -> Result<(), String> {
    let run = state
        .orchestrator
        .workflow_run_summary(workflow_run_id)
        .await
        .map_err(app_error_message)?;
    verify_workflow_run_summary_access(state, user_id, &run).await?;
    if let Some(task_id) = run.active_task_id.as_deref() {
        let _ = state.task_manager.cancel(task_id);
        state
            .task_queue
            .cancel(task_id)
            .await
            .map_err(|error| error.to_string())?;
    }
    let cancellation = state
        .orchestrator
        .cancel_workflow_run_persistence(workflow_run_id, "用户取消画布运行")
        .await
        .map_err(app_error_message)?;
    interrupt_canvas_generated_videos_for_cancelled_workflow(
        state,
        &cancellation.interrupted_generated_video_ids,
    )
    .await
}

async fn interrupt_canvas_generated_videos_for_cancelled_workflow(
    state: &AppState,
    video_ids: &[i64],
) -> Result<(), String> {
    for video_id in video_ids {
        let video = match state.orchestrator.generated_video_by_id(*video_id).await {
            Ok(Some(video)) => video,
            Ok(None) => {
                tracing::warn!(video_id, "取消画布运行时读取关联视频失败: 视频不存在");
                continue;
            }
            Err(err) => {
                tracing::warn!(video_id, error = %err, "取消画布运行时读取关联视频失败");
                continue;
            }
        };
        if matches!(
            video.status.as_str(),
            "completed" | "failed" | "cancelled" | "interrupted"
        ) {
            continue;
        }
        state
            .orchestrator
            .mark_generated_video_error_status_and_prompt(
                video.id,
                None,
                "interrupted",
                "用户取消画布运行",
            )
            .await
            .map_err(app_error_message)?;
    }
    Ok(())
}
