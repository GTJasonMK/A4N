//! Workflow / 任务查询与统一恢复的纯规则（无 DB、无运行时依赖）。

use super::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED,
    WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
};

/// 统一恢复入口可分发的目标（不含 Host 访问与领域执行）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnifiedResumeDispatch {
    /// 视频 provider 任务：按 generated_video 续轮询。
    VideoProviderJob { video_id: i64 },
    /// 画布图工作流续跑。
    VideoCanvasGraph,
    /// 章节分析工作流续跑。
    ChapterAnalysis,
}

/// 工作流终态结果（纯规则）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowRunFinalizeOutcome {
    /// 成功完成，不可再 resume。
    Completed,
    /// 失败，可 resume。
    Failed,
    /// 取消，可 resume。
    Cancelled,
}

/// 按成败解析 completed/failed 与 can_resume。
pub fn workflow_run_finalize_status_from_success(success: bool) -> (&'static str, bool) {
    if success {
        (WORKFLOW_STATUS_COMPLETED, false)
    } else {
        (WORKFLOW_STATUS_FAILED, true)
    }
}

/// 按终态枚举解析 status 与 can_resume。
pub fn workflow_run_finalize_status(outcome: WorkflowRunFinalizeOutcome) -> (&'static str, bool) {
    match outcome {
        WorkflowRunFinalizeOutcome::Completed => (WORKFLOW_STATUS_COMPLETED, false),
        WorkflowRunFinalizeOutcome::Failed => (WORKFLOW_STATUS_FAILED, true),
        WorkflowRunFinalizeOutcome::Cancelled => (WORKFLOW_STATUS_CANCELLED, true),
    }
}

/// 开始工作流 run 时的输入门禁（无 DB）。
pub fn validate_begin_workflow_run_input(input: &UpsertWorkflowRunInput) -> Result<(), String> {
    if input.business_key.trim().is_empty() {
        return Err("business_key 不能为空".to_string());
    }
    if input.workflow_kind.trim().is_empty() {
        return Err("workflow_kind 不能为空".to_string());
    }
    if let Some(project_id) = input.project_id {
        if project_id <= 0 {
            return Err("project_id 必须大于 0".to_string());
        }
    }
    if let Some(scope_id) = input.scope_id {
        if scope_id <= 0 {
            return Err("scope_id 必须大于 0".to_string());
        }
    }
    if let Some(chapter_id) = input.chapter_id {
        if chapter_id <= 0 {
            return Err("chapter_id 必须大于 0".to_string());
        }
    }
    let status = input.status.trim();
    if status != WORKFLOW_STATUS_RUNNING {
        return Err(format!(
            "begin 工作流 status 必须为 {WORKFLOW_STATUS_RUNNING}，实际为 {status}"
        ));
    }
    Ok(())
}

/// 校验 workflow_run_id 为正。
pub fn require_positive_workflow_run_id(workflow_run_id: i64) -> Result<(), String> {
    if workflow_run_id <= 0 {
        Err("workflow_run_id 必须大于 0".to_string())
    } else {
        Ok(())
    }
}

/// 任务历史 limit 必须为正。
pub fn validate_task_history_limit(limit: u64) -> Result<(), String> {
    if limit == 0 {
        Err("limit 必须大于 0".to_string())
    } else {
        Ok(())
    }
}

/// 可选 project_id 若存在则必须为正。
pub fn validate_optional_project_id(project_id: Option<i64>) -> Result<(), String> {
    match project_id {
        Some(project_id) if project_id <= 0 => Err("project_id 必须大于 0".to_string()),
        _ => Ok(()),
    }
}

/// 取消工作流必须提供 task_id 或 workflow_run_id 之一。
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

/// 取消工作流时是否无需再把视频标为 interrupted。
///
/// 含 `interrupted`：已中断则跳过。与 video job lifecycle 的「provider 任务是否仍可轮询」
/// 谓词不同（后者通常不含 interrupted）。
pub fn is_video_status_final_for_cancel_interrupt(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "cancelled" | "interrupted")
}

/// 工作流是否允许恢复；不允许时返回已存原因或默认文案。
pub fn require_workflow_can_resume(
    can_resume: bool,
    resume_block_reason: Option<&str>,
) -> Result<(), String> {
    if can_resume {
        Ok(())
    } else {
        Err(resume_block_reason
            .filter(|reason| !reason.trim().is_empty())
            .unwrap_or("该工作流当前不可恢复")
            .to_string())
    }
}

/// 解析统一恢复入口的分发目标（权限校验与领域执行由调用方完成）。
pub fn resolve_unified_resume_dispatch(
    workflow_kind: &str,
    scope_table: Option<&str>,
    scope_id: Option<i64>,
) -> Result<UnifiedResumeDispatch, String> {
    match workflow_kind {
        "video_provider_job" => Ok(UnifiedResumeDispatch::VideoProviderJob {
            video_id: video_provider_job_target_id(scope_table, scope_id)?,
        }),
        "video_canvas_graph" => Ok(UnifiedResumeDispatch::VideoCanvasGraph),
        "chapter_analysis" => Ok(UnifiedResumeDispatch::ChapterAnalysis),
        "manga_pipeline" => Err("漫画流水线请在漫画章节页使用“续传流水线”入口恢复".to_string()),
        other => Err(format!("工作流类型 {other} 暂未接入统一恢复入口")),
    }
}

/// 可恢复校验 + kind 分发的单一规划入口（Host 访问校验仍在调用方）。
pub fn plan_unified_resume(
    can_resume: bool,
    resume_block_reason: Option<&str>,
    workflow_kind: &str,
    scope_table: Option<&str>,
    scope_id: Option<i64>,
) -> Result<UnifiedResumeDispatch, String> {
    require_workflow_can_resume(can_resume, resume_block_reason)?;
    resolve_unified_resume_dispatch(workflow_kind, scope_table, scope_id)
}

/// video_provider_job 必须绑定 generated_video 作用域。
fn video_provider_job_target_id(
    scope_table: Option<&str>,
    scope_id: Option<i64>,
) -> Result<i64, String> {
    scope_id
        .filter(|_| scope_table == Some("generated_video"))
        .ok_or_else(|| "视频 provider 工作流缺少 generated_video 目标".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_history_limit_must_be_positive() {
        assert!(validate_task_history_limit(1).is_ok());
        assert_eq!(
            validate_task_history_limit(0).unwrap_err(),
            "limit 必须大于 0"
        );
    }

    #[test]
    fn optional_project_id_must_be_positive_when_present() {
        assert!(validate_optional_project_id(None).is_ok());
        assert!(validate_optional_project_id(Some(1)).is_ok());
        assert_eq!(
            validate_optional_project_id(Some(0)).unwrap_err(),
            "project_id 必须大于 0"
        );
        assert_eq!(
            validate_optional_project_id(Some(-1)).unwrap_err(),
            "project_id 必须大于 0"
        );
    }

    #[test]
    fn cancel_requires_at_least_one_target() {
        assert!(cancel_workflow_requires_target(true, false).is_ok());
        assert!(cancel_workflow_requires_target(false, true).is_ok());
        assert!(cancel_workflow_requires_target(true, true).is_ok());
        assert_eq!(
            cancel_workflow_requires_target(false, false).unwrap_err(),
            "必须提供 task_id 或 workflow_run_id"
        );
    }

    #[test]
    fn cancel_interrupt_skip_status_includes_interrupted() {
        assert!(is_video_status_final_for_cancel_interrupt("completed"));
        assert!(is_video_status_final_for_cancel_interrupt("failed"));
        assert!(is_video_status_final_for_cancel_interrupt("cancelled"));
        assert!(is_video_status_final_for_cancel_interrupt("interrupted"));
        assert!(!is_video_status_final_for_cancel_interrupt("running"));
        assert!(!is_video_status_final_for_cancel_interrupt("pending"));
    }

    #[test]
    fn require_workflow_can_resume_uses_block_reason() {
        assert!(require_workflow_can_resume(true, None).is_ok());
        assert_eq!(
            require_workflow_can_resume(false, Some("已完成")).unwrap_err(),
            "已完成"
        );
        assert_eq!(
            require_workflow_can_resume(false, None).unwrap_err(),
            "该工作流当前不可恢复"
        );
        assert_eq!(
            require_workflow_can_resume(false, Some("  ")).unwrap_err(),
            "该工作流当前不可恢复"
        );
    }

    #[test]
    fn plan_unified_resume_combines_gate_and_dispatch() {
        assert_eq!(
            plan_unified_resume(
                true,
                None,
                "video_provider_job",
                Some("generated_video"),
                Some(42)
            )
            .unwrap(),
            UnifiedResumeDispatch::VideoProviderJob { video_id: 42 }
        );
        assert_eq!(
            plan_unified_resume(false, Some("已完成"), "chapter_analysis", None, None).unwrap_err(),
            "已完成"
        );
        assert_eq!(
            plan_unified_resume(true, None, "video_canvas_graph", None, None).unwrap(),
            UnifiedResumeDispatch::VideoCanvasGraph
        );
        assert_eq!(
            plan_unified_resume(true, None, "chapter_analysis", None, None).unwrap(),
            UnifiedResumeDispatch::ChapterAnalysis
        );
        assert!(
            plan_unified_resume(true, None, "manga_pipeline", None, None)
                .unwrap_err()
                .contains("漫画流水线")
        );
        assert!(
            plan_unified_resume(true, None, "unknown_kind", None, None)
                .unwrap_err()
                .contains("暂未接入")
        );
        assert_eq!(
            plan_unified_resume(true, None, "video_provider_job", Some("other"), Some(1))
                .unwrap_err(),
            "视频 provider 工作流缺少 generated_video 目标"
        );
    }

    #[test]
    fn finalize_status_from_success_and_outcome() {
        assert_eq!(
            workflow_run_finalize_status_from_success(true),
            (WORKFLOW_STATUS_COMPLETED, false)
        );
        assert_eq!(
            workflow_run_finalize_status_from_success(false),
            (WORKFLOW_STATUS_FAILED, true)
        );
        assert_eq!(
            workflow_run_finalize_status(WorkflowRunFinalizeOutcome::Cancelled),
            (WORKFLOW_STATUS_CANCELLED, true)
        );
    }

    #[test]
    fn begin_input_requires_running_and_nonempty_keys() {
        let mut input = UpsertWorkflowRunInput {
            business_key: "k".into(),
            workflow_kind: "kind".into(),
            project_id: Some(1),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("t".into()),
            scope_id: Some(1),
            chapter_id: None,
            mode: None,
            input_fingerprint_json: None,
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: None,
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        };
        assert!(validate_begin_workflow_run_input(&input).is_ok());
        input.status = WORKFLOW_STATUS_COMPLETED.to_string();
        assert!(validate_begin_workflow_run_input(&input).is_err());
        input.status = WORKFLOW_STATUS_RUNNING.to_string();
        input.business_key = "  ".into();
        assert!(validate_begin_workflow_run_input(&input).is_err());
        assert!(require_positive_workflow_run_id(0).is_err());
        assert!(require_positive_workflow_run_id(1).is_ok());
    }
}
