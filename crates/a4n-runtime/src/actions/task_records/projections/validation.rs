pub(in crate::actions::task_records) fn validate_task_history_limit(
    limit: u64,
) -> Result<(), String> {
    if limit == 0 {
        Err("limit 必须大于 0".to_string())
    } else {
        Ok(())
    }
}

pub(in crate::actions::task_records) fn validate_optional_project_id(
    project_id: Option<i64>,
) -> Result<(), String> {
    match project_id {
        Some(project_id) if project_id <= 0 => Err("project_id 必须大于 0".to_string()),
        _ => Ok(()),
    }
}
