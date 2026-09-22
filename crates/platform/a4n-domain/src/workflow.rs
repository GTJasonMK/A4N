//! 无状态工作流输入门禁。

pub const WORKFLOW_STATUS_RUNNING: &str = "running";
pub const WORKFLOW_STATUS_INTERRUPTED: &str = "interrupted";
pub const WORKFLOW_STATUS_FAILED: &str = "failed";
pub const WORKFLOW_STATUS_CANCELLED: &str = "cancelled";
pub const WORKFLOW_STATUS_COMPLETED: &str = "completed";
pub const WORKFLOW_STATUS_SUPERSEDED: &str = "superseded";
pub const WORKFLOW_MODE_DERIVED_REPLAY: &str = "derived_replay";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnifiedResumeDispatch {
    VideoProviderJob { video_id: i64 },
    VideoCanvasGraph,
    ChapterAnalysis,
    ChapterContentSync { chapter_id: i64 },
    BlueprintGeneration,
    BlueprintRefinement,
}

pub fn require_positive_workflow_run_id(workflow_run_id: i64) -> Result<(), String> {
    if workflow_run_id <= 0 {
        Err("workflow_run_id 必须大于 0".to_string())
    } else {
        Ok(())
    }
}

pub fn validate_task_history_limit(limit: u64) -> Result<(), String> {
    if limit == 0 {
        Err("limit 必须大于 0".to_string())
    } else {
        Ok(())
    }
}

pub fn validate_optional_project_id(project_id: Option<i64>) -> Result<(), String> {
    match project_id {
        Some(project_id) if project_id <= 0 => Err("project_id 必须大于 0".to_string()),
        _ => Ok(()),
    }
}

pub fn cancel_workflow_requires_target(
    has_task_id: bool,
    has_workflow_run_id: bool,
) -> Result<(), String> {
    if !has_task_id && !has_workflow_run_id {
        Err("必须提供 task_id 或 workflow_run_id".to_string())
    } else {
        Ok(())
    }
}

pub fn is_video_status_final_for_cancel_interrupt(status: &str) -> bool {
    matches!(
        status,
        WORKFLOW_STATUS_COMPLETED
            | WORKFLOW_STATUS_FAILED
            | WORKFLOW_STATUS_CANCELLED
            | WORKFLOW_STATUS_INTERRUPTED
    )
}

pub fn require_workflow_can_resume(
    status: &str,
    can_resume: bool,
    resume_block_reason: Option<&str>,
) -> Result<(), String> {
    match status {
        WORKFLOW_STATUS_RUNNING => return Err("工作流正在运行，不能重复恢复".to_string()),
        WORKFLOW_STATUS_COMPLETED => return Err("工作流已完成，不能恢复".to_string()),
        _ => {}
    }
    if can_resume {
        return if matches!(
            status,
            WORKFLOW_STATUS_FAILED | WORKFLOW_STATUS_CANCELLED | WORKFLOW_STATUS_INTERRUPTED
        ) {
            Ok(())
        } else {
            Err(format!("工作流状态 {status} 不支持恢复"))
        };
    }
    let reason = resume_block_reason
        .filter(|reason| !reason.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("工作流状态 {status} 缺少不可恢复原因"))?;
    Err(reason)
}

pub fn resolve_unified_resume_dispatch(
    workflow_kind: &str,
    scope_table: Option<&str>,
    scope_id: Option<i64>,
) -> Result<UnifiedResumeDispatch, String> {
    match workflow_kind {
        "video_provider_job" => Ok(UnifiedResumeDispatch::VideoProviderJob {
            video_id: scope_id
                .filter(|_| scope_table == Some("generated_video"))
                .ok_or_else(|| "视频 provider 工作流缺少 generated_video 目标".to_string())?,
        }),
        "video_canvas_graph" => Ok(UnifiedResumeDispatch::VideoCanvasGraph),
        "chapter_analysis" => Ok(UnifiedResumeDispatch::ChapterAnalysis),
        "chapter_content_sync" => Ok(UnifiedResumeDispatch::ChapterContentSync {
            chapter_id: scope_id
                .filter(|id| *id > 0 && scope_table == Some("chapter"))
                .ok_or_else(|| "章节资料同步工作流缺少 chapter 目标".to_string())?,
        }),
        "blueprint_generation" => {
            if scope_id
                .filter(|id| *id > 0 && scope_table == Some("project"))
                .is_none()
            {
                return Err("蓝图生成工作流缺少 project 目标".to_string());
            }
            Ok(UnifiedResumeDispatch::BlueprintGeneration)
        }
        "blueprint_refine" => {
            if scope_id
                .filter(|id| *id > 0 && scope_table == Some("novel_blueprint"))
                .is_none()
            {
                return Err("蓝图优化工作流缺少 novel_blueprint 目标".to_string());
            }
            Ok(UnifiedResumeDispatch::BlueprintRefinement)
        }
        "manga_pipeline" => Err("漫画流水线请在漫画章节页使用“续传流水线”入口恢复".to_string()),
        other => Err(format!("工作流类型 {other} 暂未接入统一恢复入口")),
    }
}

pub fn plan_unified_resume(
    status: &str,
    can_resume: bool,
    resume_block_reason: Option<&str>,
    workflow_kind: &str,
    scope_table: Option<&str>,
    scope_id: Option<i64>,
) -> Result<UnifiedResumeDispatch, String> {
    require_workflow_can_resume(status, can_resume, resume_block_reason)?;
    resolve_unified_resume_dispatch(workflow_kind, scope_table, scope_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_input_guards_are_explicit() {
        assert!(require_positive_workflow_run_id(1).is_ok());
        assert!(require_positive_workflow_run_id(0).is_err());
        assert!(validate_task_history_limit(1).is_ok());
        assert_eq!(
            validate_task_history_limit(0).unwrap_err(),
            "limit 必须大于 0"
        );
        assert!(validate_optional_project_id(None).is_ok());
        assert!(validate_optional_project_id(Some(1)).is_ok());
        assert!(validate_optional_project_id(Some(0)).is_err());
    }

    #[test]
    fn cancellation_guards_cover_target_and_terminal_statuses() {
        assert!(cancel_workflow_requires_target(true, false).is_ok());
        assert!(cancel_workflow_requires_target(false, true).is_ok());
        assert!(cancel_workflow_requires_target(false, false).is_err());
        assert!(is_video_status_final_for_cancel_interrupt("completed"));
        assert!(is_video_status_final_for_cancel_interrupt("failed"));
        assert!(is_video_status_final_for_cancel_interrupt("cancelled"));
        assert!(is_video_status_final_for_cancel_interrupt("interrupted"));
        assert!(!is_video_status_final_for_cancel_interrupt("running"));
        assert!(!is_video_status_final_for_cancel_interrupt("pending"));
    }
}
