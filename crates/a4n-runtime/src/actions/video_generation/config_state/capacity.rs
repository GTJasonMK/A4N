use super::super::media::prompt_id_from_video;
use crate::AppState;
use a4n_core::models::ai::generated_video;
use a4n_orchestrator::video::validate_video_max_concurrent;
use a4n_orchestrator::workflow::{
    WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_INTERRUPTED,
    WORKFLOW_STATUS_SUPERSEDED,
};

pub(crate) async fn ensure_video_capacity(
    state: &AppState,
    config_id: i64,
    max_concurrent: i32,
) -> Result<(), String> {
    validate_video_max_concurrent(max_concurrent)?;
    let limit = max_concurrent as usize;
    let used = active_video_ids_for_config(state, config_id).await?.len();
    if used >= limit {
        Err(format!(
            "当前视频配置已有 {used} 个进行中的任务，已达到并发上限 {limit}"
        ))
    } else {
        Ok(())
    }
}

pub(crate) async fn ensure_created_video_capacity(
    state: &AppState,
    config_id: i64,
    max_concurrent: i32,
    video_id: i64,
) -> Result<(), String> {
    validate_video_max_concurrent(max_concurrent)?;
    let limit = max_concurrent as usize;
    let active = active_video_ids_for_config(state, config_id).await?;
    if active.len() <= limit || active.iter().take(limit).any(|id| *id == video_id) {
        Ok(())
    } else {
        let existing_active_count = active.iter().filter(|id| **id != video_id).count();
        Err(format!(
            "当前视频配置已有 {} 个进行中的任务，已达到并发上限 {limit}",
            existing_active_count
        ))
    }
}

async fn active_video_ids_for_config(state: &AppState, config_id: i64) -> Result<Vec<i64>, String> {
    let active_ids = state
        .orchestrator
        .active_generated_video_ids_for_config(config_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    let mut retained = Vec::with_capacity(active_ids.len());
    for video_id in active_ids {
        if reconcile_interrupted_provider_video(state, video_id).await? {
            continue;
        }
        retained.push(video_id);
    }
    Ok(retained)
}

async fn reconcile_interrupted_provider_video(
    state: &AppState,
    video_id: i64,
) -> Result<bool, String> {
    let Some(video) = state
        .orchestrator
        .generated_video_by_id(video_id)
        .await
        .map_err(crate::actions::app_error_message)?
    else {
        return Ok(true);
    };
    if video
        .provider_job_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        return Ok(false);
    }
    if reconcile_terminal_canvas_workflow_video(state, &video).await? {
        return Ok(true);
    }
    let Some(run) = state
        .orchestrator
        .latest_workflow_run_by_business_key(&format!("video_provider_job:{video_id}"))
        .await
        .map_err(crate::actions::app_error_message)?
    else {
        return Ok(false);
    };
    if run.status != WORKFLOW_STATUS_INTERRUPTED {
        return Ok(false);
    }
    let message = run
        .error_message
        .as_deref()
        .unwrap_or("视频 provider 轮询已中断，可稍后恢复轮询");
    state
        .orchestrator
        .mark_generated_video_error_status_and_prompt(
            video.id,
            prompt_id_from_video(&video),
            "interrupted",
            message,
        )
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(true)
}

async fn reconcile_terminal_canvas_workflow_video(
    state: &AppState,
    video: &generated_video::Model,
) -> Result<bool, String> {
    let Some(workflow_run_id) = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("canvas"))
        .and_then(|canvas| canvas.get("workflow_run_id"))
        .and_then(serde_json::Value::as_i64)
    else {
        return Ok(false);
    };
    let Some(run) = state
        .orchestrator
        .workflow_run_model_optional_by_id(workflow_run_id)
        .await
        .map_err(crate::actions::app_error_message)?
    else {
        return Ok(false);
    };
    if !matches!(
        run.status.as_str(),
        WORKFLOW_STATUS_CANCELLED | WORKFLOW_STATUS_FAILED | WORKFLOW_STATUS_SUPERSEDED
    ) {
        return Ok(false);
    }
    let message = format!(
        "所属画布运行 #{} 已{}",
        run.id,
        terminal_workflow_status_label(&run.status)
    );
    state
        .orchestrator
        .mark_generated_video_error_status_and_prompt(
            video.id,
            prompt_id_from_video(video),
            "interrupted",
            &message,
        )
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(true)
}

fn terminal_workflow_status_label(status: &str) -> &'static str {
    match status {
        WORKFLOW_STATUS_CANCELLED => "取消",
        WORKFLOW_STATUS_FAILED => "失败",
        WORKFLOW_STATUS_SUPERSEDED => "被取代",
        _ => "结束",
    }
}
