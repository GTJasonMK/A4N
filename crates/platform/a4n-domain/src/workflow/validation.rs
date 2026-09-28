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
