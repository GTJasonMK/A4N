use super::media::prompt_id_from_video;
use crate::AppState;
use a4n_core::models::ai::{generated_video, video_gen_config};
use a4n_core::models::generation::workflow_run;
use a4n_services::config::active_generated_video_ids_for_config;
use a4n_services::video::validate_video_max_concurrent;
use a4n_services::workflow::{
    WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_INTERRUPTED,
    WORKFLOW_STATUS_SUPERSEDED,
};
use sea_orm::*;

pub(super) async fn resolve_video_config(
    state: &AppState,
    config_id: Option<i64>,
) -> Result<video_gen_config::Model, String> {
    if let Some(config) = crate::runtime_config::current_video_model_for_id(config_id) {
        return Ok(config);
    }
    if config_id.is_none() && crate::runtime_config::uses_system_default_video_template() {
        return resolve_system_default_video_template_with_user_key(state).await;
    }
    if let Some(id) = config_id
        && crate::runtime_config::current_video_snapshot_scope_present()
    {
        return Err(format!(
            "runtime_config_snapshot.video_configs 不包含视频配置 #{id}"
        ));
    }
    if crate::runtime_config::current_user_id().is_some() {
        return Err(match config_id {
            Some(id) => format!(
                "远程用户请求不能直接使用服务器视频配置 #{id}；请在用户配置页使用个人视频配置快照"
            ),
            None => "使用系统默认视频配置需要先在用户配置页填写并保存你的个人 API Key".to_string(),
        });
    }

    crate::global_ai_config::resolve_video_config(state.db.as_ref(), config_id)
        .await
        .map_err(|e| e.to_string())
}

async fn resolve_system_default_video_template_with_user_key(
    state: &AppState,
) -> Result<video_gen_config::Model, String> {
    let api_key = crate::runtime_config::current_system_video_api_key().ok_or_else(|| {
        "使用系统默认视频配置需要先在用户配置页填写并保存你的个人 API Key".to_string()
    })?;
    let mut model = crate::global_ai_config::resolve_video_config(state.db.as_ref(), None)
        .await
        .map_err(|e| e.to_string())?;
    model.api_key_encrypted = Some(api_key);
    Ok(model)
}

pub(super) async fn ensure_video_capacity(
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

pub(super) async fn ensure_created_video_capacity(
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
        Err(format!(
            "当前视频配置已有 {} 个进行中的任务，已达到并发上限 {limit}",
            active.len().saturating_sub(1)
        ))
    }
}

async fn active_video_ids_for_config(state: &AppState, config_id: i64) -> Result<Vec<i64>, String> {
    let active_ids = active_generated_video_ids_for_config(state.db.as_ref(), config_id)
        .await
        .map_err(|e| e.to_string())?;
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
    let Some(video) = generated_video::Entity::find_by_id(video_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
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
    let Some(run) = workflow_run::Entity::find()
        .filter(workflow_run::Column::BusinessKey.eq(format!("video_provider_job:{video_id}")))
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
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
        .video
        .mark_interrupted_and_update_prompt(video.id, prompt_id_from_video(&video), message)
        .await
        .map_err(|e| e.to_string())?;
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
    let Some(run) = workflow_run::Entity::find_by_id(workflow_run_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
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
        .video
        .mark_interrupted_and_update_prompt(video.id, prompt_id_from_video(video), &message)
        .await
        .map_err(|e| e.to_string())?;
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
