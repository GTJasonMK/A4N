//! 无状态工作流输入门禁。

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
    matches!(status, "completed" | "failed" | "cancelled" | "interrupted")
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
        assert!(is_video_status_final_for_cancel_interrupt("interrupted"));
        assert!(!is_video_status_final_for_cancel_interrupt("running"));
    }
}
