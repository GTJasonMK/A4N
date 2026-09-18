//! Workflow / 任务查询与统一恢复的纯规则（无 DB、无运行时依赖）。

use super::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED,
    WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
};

pub use a4n_domain::workflow::{
    is_video_status_final_for_cancel_interrupt, require_positive_workflow_run_id,
    validate_optional_project_id,
};

#[cfg(test)]
use a4n_domain::workflow::UnifiedResumeDispatch;

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
    if let Some(project_id) = input.project_id
        && project_id <= 0
    {
        return Err("project_id 必须大于 0".to_string());
    }
    if let Some(scope_id) = input.scope_id
        && scope_id <= 0
    {
        return Err("scope_id 必须大于 0".to_string());
    }
    if let Some(chapter_id) = input.chapter_id
        && chapter_id <= 0
    {
        return Err("chapter_id 必须大于 0".to_string());
    }
    let status = input.status.trim();
    if status != WORKFLOW_STATUS_RUNNING {
        return Err(format!(
            "begin 工作流 status 必须为 {WORKFLOW_STATUS_RUNNING}，实际为 {status}"
        ));
    }
    Ok(())
}

/// 工作流是否允许恢复；状态本身可判定时直接拒绝，其余不可恢复状态必须有持久化原因。
pub(crate) fn require_workflow_can_resume(
    status: &str,
    can_resume: bool,
    resume_block_reason: Option<&str>,
) -> Result<(), String> {
    a4n_domain::workflow::require_workflow_can_resume(status, can_resume, resume_block_reason)
}

/// 可恢复校验 + kind 分发的单一规划入口（Host 访问校验仍在调用方）。
#[cfg(test)]
fn plan_unified_resume(
    status: &str,
    can_resume: bool,
    resume_block_reason: Option<&str>,
    workflow_kind: &str,
    scope_table: Option<&str>,
    scope_id: Option<i64>,
) -> Result<UnifiedResumeDispatch, String> {
    a4n_domain::workflow::plan_unified_resume(
        status,
        can_resume,
        resume_block_reason,
        workflow_kind,
        scope_table,
        scope_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_workflow_can_resume_is_status_aware() {
        assert!(require_workflow_can_resume(WORKFLOW_STATUS_FAILED, true, None).is_ok());
        assert_eq!(
            require_workflow_can_resume(WORKFLOW_STATUS_COMPLETED, false, None).unwrap_err(),
            "工作流已完成，不能恢复"
        );
        assert_eq!(
            require_workflow_can_resume(WORKFLOW_STATUS_RUNNING, true, None).unwrap_err(),
            "工作流正在运行，不能重复恢复"
        );
        assert_eq!(
            require_workflow_can_resume(WORKFLOW_STATUS_FAILED, false, Some("断点已失效"))
                .unwrap_err(),
            "断点已失效"
        );
        assert_eq!(
            require_workflow_can_resume(WORKFLOW_STATUS_FAILED, false, None).unwrap_err(),
            "工作流状态 failed 缺少不可恢复原因"
        );
    }

    #[test]
    fn plan_unified_resume_combines_gate_and_dispatch() {
        assert_eq!(
            plan_unified_resume(
                WORKFLOW_STATUS_FAILED,
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
            plan_unified_resume(
                WORKFLOW_STATUS_COMPLETED,
                false,
                None,
                "chapter_analysis",
                None,
                None,
            )
            .unwrap_err(),
            "工作流已完成，不能恢复"
        );
        assert_eq!(
            plan_unified_resume(
                WORKFLOW_STATUS_FAILED,
                true,
                None,
                "video_canvas_graph",
                None,
                None,
            )
            .unwrap(),
            UnifiedResumeDispatch::VideoCanvasGraph
        );
        assert_eq!(
            plan_unified_resume(
                WORKFLOW_STATUS_FAILED,
                true,
                None,
                "chapter_analysis",
                None,
                None,
            )
            .unwrap(),
            UnifiedResumeDispatch::ChapterAnalysis
        );
        assert_eq!(
            plan_unified_resume(
                WORKFLOW_STATUS_FAILED,
                true,
                None,
                "chapter_content_sync",
                Some("chapter"),
                Some(7),
            )
            .unwrap(),
            UnifiedResumeDispatch::ChapterContentSync { chapter_id: 7 }
        );
        assert_eq!(
            plan_unified_resume(
                WORKFLOW_STATUS_FAILED,
                true,
                None,
                "blueprint_generation",
                Some("project"),
                Some(9),
            )
            .unwrap(),
            UnifiedResumeDispatch::BlueprintGeneration
        );
        assert_eq!(
            plan_unified_resume(
                WORKFLOW_STATUS_CANCELLED,
                true,
                None,
                "blueprint_refine",
                Some("novel_blueprint"),
                Some(11),
            )
            .unwrap(),
            UnifiedResumeDispatch::BlueprintRefinement
        );
        assert!(
            plan_unified_resume(
                WORKFLOW_STATUS_FAILED,
                true,
                None,
                "manga_pipeline",
                None,
                None,
            )
            .unwrap_err()
            .contains("漫画流水线")
        );
        assert!(
            plan_unified_resume(
                WORKFLOW_STATUS_FAILED,
                true,
                None,
                "unknown_kind",
                None,
                None,
            )
            .unwrap_err()
            .contains("暂未接入")
        );
        assert_eq!(
            plan_unified_resume(
                WORKFLOW_STATUS_FAILED,
                true,
                None,
                "video_provider_job",
                Some("other"),
                Some(1),
            )
            .unwrap_err(),
            "视频 provider 工作流缺少 generated_video 目标"
        );
        assert_eq!(
            plan_unified_resume(
                WORKFLOW_STATUS_FAILED,
                true,
                None,
                "chapter_content_sync",
                Some("other"),
                Some(7),
            )
            .unwrap_err(),
            "章节资料同步工作流缺少 chapter 目标"
        );
        assert_eq!(
            plan_unified_resume(
                WORKFLOW_STATUS_FAILED,
                true,
                None,
                "blueprint_generation",
                Some("novel_blueprint"),
                Some(1),
            )
            .unwrap_err(),
            "蓝图生成工作流缺少 project 目标"
        );
    }

    #[test]
    fn finalize_status_comes_from_outcome() {
        assert_eq!(
            workflow_run_finalize_status(WorkflowRunFinalizeOutcome::Completed),
            (WORKFLOW_STATUS_COMPLETED, false)
        );
        assert_eq!(
            workflow_run_finalize_status(WorkflowRunFinalizeOutcome::Failed),
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
    }
}
