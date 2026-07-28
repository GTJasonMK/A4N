//! 视频 provider 轮询工作流指纹与 business key 纯规则（无 DB）。

use a4n_core::models::ai::generated_video;
use serde_json::{Value, json};

use crate::workflow::UpsertWorkflowRunInput;
use crate::workflow::{
    WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_INTERRUPTED,
    WORKFLOW_STATUS_RUNNING,
};

/// 视频 provider job exclusive business key。
pub fn video_provider_workflow_business_key(video_id: i64) -> Result<String, String> {
    if video_id <= 0 {
        return Err("generated_video_id 必须大于 0".to_string());
    }
    Ok(format!("video_provider_job:{video_id}"))
}

/// 从 metadata 提取 prompt_id（与 Host 历史约定一致）。
pub fn prompt_id_from_generated_video(video: &generated_video::Model) -> Option<i64> {
    video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("prompt_id"))
        .and_then(Value::as_i64)
}

/// 视频 provider 工作流输入指纹。
pub fn video_provider_workflow_fingerprint(video: &generated_video::Model, job_id: &str) -> Value {
    json!({
        "generated_video_id": video.id,
        "provider_job_id": job_id,
        "prompt_id": prompt_id_from_generated_video(video),
        "config_id": video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("config_id"))
            .and_then(Value::as_i64),
        "workflow_mode": video.workflow_mode,
        "source_prompt_type": video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("source_prompt_type")),
        "video_execution_identity": video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("video_execution_identity")),
    })
}

/// 构造视频 provider 工作流 upsert 输入（状态由调用方指定）。
pub fn video_provider_workflow_run_input(
    video: &generated_video::Model,
    job_id: &str,
    status: &str,
    can_resume: bool,
    error_message: Option<&str>,
) -> Result<UpsertWorkflowRunInput, String> {
    if video.project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    if job_id.trim().is_empty() {
        return Err("provider_job_id 不能为空".to_string());
    }
    let status = status.trim();
    if status.is_empty() {
        return Err("provider workflow status 不能为空".to_string());
    }
    let fingerprint = video_provider_workflow_fingerprint(video, job_id);
    Ok(UpsertWorkflowRunInput {
        business_key: video_provider_workflow_business_key(video.id)?,
        workflow_kind: "video_provider_job".to_string(),
        project_id: Some(video.project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("generated_video".to_string()),
        scope_id: Some(video.id),
        chapter_id: video.chapter_id,
        mode: video.workflow_mode.clone(),
        input_fingerprint_json: Some(fingerprint),
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
}

/// 构造 RUNNING 态 provider 进度 begin 输入。
pub fn video_provider_progress_workflow_run_input(
    video: &generated_video::Model,
    job_id: &str,
) -> Result<UpsertWorkflowRunInput, String> {
    video_provider_workflow_run_input(video, job_id, WORKFLOW_STATUS_RUNNING, true, None)
}

/// 允许的终态/中断状态（用于调用方校验）。
pub fn is_video_provider_terminal_or_interrupted_status(status: &str) -> bool {
    matches!(
        status,
        WORKFLOW_STATUS_COMPLETED
            | WORKFLOW_STATUS_FAILED
            | WORKFLOW_STATUS_INTERRUPTED
            | "cancelled"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_key_requires_positive_video_id() {
        assert!(video_provider_workflow_business_key(0).is_err());
        assert_eq!(
            video_provider_workflow_business_key(7).unwrap(),
            "video_provider_job:7"
        );
    }
}
