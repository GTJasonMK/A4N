//! 视频生成 job 状态推进纯规则（poll / 失败 / submit 落库门禁）。

use a4n_core::{
    error::AppError, models::ai::generated_video, video_workflow::CANVAS_WORKFLOW_MODES,
};

pub fn can_adopt_authoritative_video_completion(error: &AppError) -> bool {
    matches!(error, AppError::Database(_))
        || matches!(
            error,
            AppError::InvalidState { current, .. }
                if matches!(
                    current.as_str(),
                    generated_video::STATUS_COMPLETED
                        | generated_video::STATUS_FAILED
                        | generated_video::STATUS_INTERRUPTED
                        | generated_video::STATUS_CANCELLED
                )
        )
}

/// 校验「等待中」状态仅允许 pending / running。
pub fn validate_generated_video_waiting_status(status: &str) -> Result<(), String> {
    match status {
        generated_video::STATUS_PENDING | generated_video::STATUS_RUNNING => Ok(()),
        _ => Err(format!("不支持的视频等待状态: {status}")),
    }
}

/// 校验中断状态字面量（仅 interrupted）。
pub fn validate_generated_video_interrupted_status(status: &str) -> Result<(), String> {
    if status == generated_video::STATUS_INTERRUPTED {
        Ok(())
    } else {
        Err(format!("不支持的视频中断状态: {status}"))
    }
}

/// 规范化 provider job id：trim 后非空才有效。
pub fn normalized_provider_job_id(provider_job_id: Option<&str>) -> Option<&str> {
    provider_job_id
        .map(str::trim)
        .filter(|job_id| !job_id.is_empty())
}

/// 提交 job 时要求非空 job id（trim 后）。
pub fn required_provider_job_id(provider_job_id: &str) -> Result<&str, String> {
    normalized_provider_job_id(Some(provider_job_id))
        .ok_or_else(|| "provider job id 不能为空".to_string())
}

/// 活跃 provider job：非终态且 job id 有效。
pub fn active_provider_job_id<'a>(
    provider_job_id: Option<&'a str>,
    status: &str,
) -> Option<&'a str> {
    if generated_video::is_terminal_status(status) {
        return None;
    }
    normalized_provider_job_id(provider_job_id)
}

/// 是否存在可恢复/可轮询的 provider job。
pub fn has_active_provider_job(provider_job_id: Option<&str>, status: &str) -> bool {
    active_provider_job_id(provider_job_id, status).is_some()
}

/// 旧画布/章节 provider job 恢复轮询时使用的配置 ID。
pub fn provider_poll_config_id(video: &generated_video::Model) -> Result<Option<i64>, String> {
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
    let config_id = value.as_i64().filter(|id| *id > 0).ok_or_else(|| {
        format!(
            "视频 #{} 的 metadata_json.config_id 必须是正整数，不能恢复 provider 轮询",
            video.id
        )
    })?;
    Ok(config_id)
}

pub fn is_canvas_video_record(video: &generated_video::Model) -> bool {
    video.chapter_id.is_none()
        && video
            .workflow_mode
            .as_deref()
            .is_some_and(|mode| CANVAS_WORKFLOW_MODES.contains(&mode))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasVideoWorkflowContext {
    pub workflow_run_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
}

pub fn canvas_video_workflow_context(
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
    Ok(Some(CanvasVideoWorkflowContext {
        workflow_run_id: required_canvas_video_context_id(video.id, canvas, "workflow_run_id")?,
        graph_id: required_canvas_video_context_id(video.id, canvas, "graph_id")?,
        node_id: required_canvas_video_context_id(video.id, canvas, "node_id")?,
    }))
}

fn required_canvas_video_context_id(
    video_id: i64,
    canvas: &serde_json::Value,
    field: &str,
) -> Result<i64, String> {
    canvas
        .get(field)
        .and_then(serde_json::Value::as_i64)
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("画布视频 #{video_id} 的 metadata_json.canvas.{field} 必须是正整数"))
}

/// provider 关联 generation_run 必须至少更新一条。
pub fn ensure_provider_generation_run_updated(
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

/// 失败原因非空。
pub fn required_generated_video_failure_message(message: &str) -> Result<&str, String> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Err("失败原因不能为空".to_string());
    }
    Ok(trimmed)
}

/// 视频 ID 必须为正。
pub fn ensure_positive_generated_video_id(video_id: i64) -> Result<(), String> {
    if video_id <= 0 {
        return Err("视频 ID 必须大于 0".to_string());
    }
    Ok(())
}

/// 项目 ID 必须为正。
pub fn ensure_positive_project_id_for_video(project_id: i64) -> Result<(), String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    Ok(())
}

/// 关联章节视频提示词 ID 必须为正。
pub fn ensure_positive_video_prompt_id(prompt_id: i64) -> Result<(), String> {
    if prompt_id <= 0 {
        return Err("视频提示词 ID 必须大于 0".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;

    fn video(video_type: &str, workflow_mode: Option<&str>) -> generated_video::Model {
        let now = Utc::now().naive_utc();
        generated_video::Model {
            id: 42,
            project_id: 1,
            chapter_id: Some(1),
            video_type: video_type.to_string(),
            workflow_mode: workflow_mode.map(str::to_string),
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
            status: "running".to_string(),
            provider_job_id: Some("provider-job".to_string()),
            error_message: None,
            timeline_item_id: None,
            metadata_json: None,
            is_favorite: false,
            created_at: now,
            updated_at: now,
        }
    }

    fn canvas_video(metadata_json: Option<serde_json::Value>) -> generated_video::Model {
        let mut video = video(
            generated_video::VIDEO_TYPE_CANVAS,
            Some(a4n_core::video_workflow::WORKFLOW_MODE_CANVAS_VIDEO),
        );
        video.chapter_id = None;
        video.metadata_json = metadata_json;
        video
    }

    #[test]
    fn waiting_status_only_allows_pending_or_running() {
        assert!(validate_generated_video_waiting_status("pending").is_ok());
        assert!(validate_generated_video_waiting_status("running").is_ok());
        assert!(validate_generated_video_waiting_status("failed").is_err());
        assert!(validate_generated_video_waiting_status("completed").is_err());
    }

    #[test]
    fn interrupted_status_literal_only() {
        assert!(validate_generated_video_interrupted_status("interrupted").is_ok());
        assert!(validate_generated_video_interrupted_status("failed").is_err());
    }

    #[test]
    fn ensure_provider_run_requires_at_least_one() {
        assert!(ensure_provider_generation_run_updated(1, 1, "完成").is_ok());
        let err =
            ensure_provider_generation_run_updated(9, 0, "标记 provider 视频生成失败").unwrap_err();
        assert!(err.contains("#9"));
        assert!(err.contains("generation_run"));
    }

    #[test]
    fn failure_message_and_video_id_gates() {
        assert_eq!(
            required_generated_video_failure_message("  boom  ").unwrap(),
            "boom"
        );
        assert!(required_generated_video_failure_message("   ").is_err());
        assert!(ensure_positive_generated_video_id(1).is_ok());
        assert!(ensure_positive_generated_video_id(0).is_err());
    }

    #[test]
    fn provider_job_id_normalize_and_active_semantics() {
        assert_eq!(normalized_provider_job_id(Some(" job-1 ")), Some("job-1"));
        assert_eq!(normalized_provider_job_id(Some(" \n\t ")), None);
        assert_eq!(required_provider_job_id("  j  ").unwrap(), "j");
        assert!(required_provider_job_id("  ").is_err());

        assert!(generated_video::is_terminal_status("completed"));
        assert!(generated_video::is_terminal_status("failed"));
        assert!(generated_video::is_terminal_status("cancelled"));
        assert!(!generated_video::is_terminal_status("interrupted"));
        assert!(!generated_video::is_terminal_status("running"));

        assert_eq!(
            active_provider_job_id(Some(" job-1 "), "running"),
            Some("job-1")
        );
        assert_eq!(
            active_provider_job_id(Some("job-1"), "interrupted"),
            Some("job-1")
        );
        assert_eq!(active_provider_job_id(Some("job-1"), "completed"), None);
        assert!(has_active_provider_job(Some("job-1"), "pending"));
        assert!(!has_active_provider_job(Some("job-1"), "failed"));
    }

    #[test]
    fn authoritative_completion_only_recovers_ambiguous_or_terminal_conflicts() {
        assert!(can_adopt_authoritative_video_completion(
            &AppError::InvalidState {
                current: "completed".to_string(),
                operation: "重复完成".to_string(),
            }
        ));
        assert!(!can_adopt_authoritative_video_completion(
            &AppError::InvalidArgument("提示词不匹配".to_string())
        ));
    }

    #[test]
    fn project_and_prompt_id_gates() {
        assert!(ensure_positive_project_id_for_video(1).is_ok());
        assert!(ensure_positive_project_id_for_video(0).is_err());
        assert!(ensure_positive_video_prompt_id(3).is_ok());
        assert!(ensure_positive_video_prompt_id(-1).is_err());
    }

    #[test]
    fn provider_poll_config_requires_valid_canvas_config() {
        assert_eq!(
            provider_poll_config_id(&video(
                generated_video::VIDEO_TYPE_CHAPTER,
                Some("boundary_frames")
            ))
            .unwrap(),
            None
        );
        assert!(
            provider_poll_config_id(&canvas_video(Some(json!({
                "canvas": {"workflow_run_id": 1, "graph_id": 2, "node_id": 3}
            }))))
            .unwrap_err()
            .contains("缺少 metadata_json.config_id")
        );
        assert!(
            provider_poll_config_id(&canvas_video(Some(json!({"config_id": 0}))))
                .unwrap_err()
                .contains("必须是正整数")
        );
        assert_eq!(
            provider_poll_config_id(&canvas_video(Some(json!({"config_id": 9})))).unwrap(),
            Some(9)
        );
    }

    #[test]
    fn canvas_workflow_context_requires_positive_ids() {
        assert_eq!(
            canvas_video_workflow_context(&video(
                generated_video::VIDEO_TYPE_CHAPTER,
                Some("boundary_frames")
            ))
            .unwrap(),
            None
        );
        assert!(canvas_video_workflow_context(&canvas_video(None)).is_err());
        assert!(
            canvas_video_workflow_context(&canvas_video(Some(json!({
                "canvas": {"workflow_run_id": 1, "graph_id": 2}
            }))))
            .unwrap_err()
            .contains("node_id")
        );
        assert_eq!(
            canvas_video_workflow_context(&canvas_video(Some(json!({
                "canvas": {"workflow_run_id": 11, "graph_id": 22, "node_id": 33}
            }))))
            .unwrap(),
            Some(CanvasVideoWorkflowContext {
                workflow_run_id: 11,
                graph_id: 22,
                node_id: 33,
            })
        );
    }
}
