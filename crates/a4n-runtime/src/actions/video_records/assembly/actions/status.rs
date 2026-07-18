use super::super::contracts::{
    ASSEMBLY_PUSH_STATUS_ACCEPTED, ASSEMBLY_PUSH_STATUS_PENDING, ASSEMBLY_PUSH_STATUS_REJECTED,
    ASSEMBLY_PUSH_VIDEO_TYPE, UpdateAssemblyPushStatusInput,
};
use super::super::push::{
    assembly_push_metadata_with_status, assembly_push_rows, latest_non_rejected_push,
};
use crate::AppState;
use crate::actions::access;
use crate::actions::app_error_message;
use crate::actions::video_generation::{GeneratedVideoSummary, video_to_summary};
use a4n_orchestrator::video::UpdateGeneratedVideoRecordInput;

pub async fn update_assembly_push_status(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    input: UpdateAssemblyPushStatusInput,
) -> Result<GeneratedVideoSummary, String> {
    let existing = state
        .orchestrator
        .generated_video_by_id(video_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| "视频不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, existing.project_id).await?;
    if existing.video_type != ASSEMBLY_PUSH_VIDEO_TYPE {
        return Err(format!("视频 #{} 不是成片后处理推送批次", existing.id));
    }
    let chapter_id = existing
        .chapter_id
        .ok_or_else(|| format!("推送批次 #{} 缺少章节绑定", existing.id))?;
    let workflow_mode = existing
        .workflow_mode
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("推送批次 #{} 缺少 workflow_mode", existing.id))?;
    let rows = state
        .orchestrator
        .generated_videos_for_chapter(existing.project_id, chapter_id)
        .await
        .map_err(app_error_message)?;
    let pushes = assembly_push_rows(&rows, workflow_mode)?;
    let latest = latest_non_rejected_push(&pushes)
        .ok_or_else(|| "当前章节没有可管理的推送批次".to_string())?;
    if latest.video.id != existing.id {
        return Err("只能管理当前最新推送批次，不能跨推送操作".to_string());
    }
    let action = input.action.trim();
    let next_status = match (action, latest.status.as_str()) {
        ("accept", ASSEMBLY_PUSH_STATUS_PENDING) => ASSEMBLY_PUSH_STATUS_ACCEPTED,
        ("reject", ASSEMBLY_PUSH_STATUS_PENDING) => ASSEMBLY_PUSH_STATUS_REJECTED,
        ("undo", ASSEMBLY_PUSH_STATUS_ACCEPTED) => ASSEMBLY_PUSH_STATUS_PENDING,
        ("accept", _) => return Err("只有待处理推送可以同意".to_string()),
        ("reject", _) => return Err("只有待处理推送可以拒绝".to_string()),
        ("undo", _) => return Err("只有已同意的最新推送可以撤销".to_string()),
        _ => return Err(format!("未知推送管理操作：{action}")),
    };
    let metadata = assembly_push_metadata_with_status(existing.metadata_json.clone(), next_status);
    let row = state
        .orchestrator
        .update_generated_video_record(
            existing.id,
            UpdateGeneratedVideoRecordInput {
                prompt: None,
                negative_prompt: None,
                clear_negative_prompt: false,
                metadata_json: Some(metadata),
                clear_metadata_json: false,
                is_favorite: None,
            },
        )
        .await
        .map_err(app_error_message)?;
    Ok(video_to_summary(&row))
}
