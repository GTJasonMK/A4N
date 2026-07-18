use a4n_core::error::AppError;

pub fn inspiration_stream_task_id(
    project_id: i64,
    provided: Option<String>,
) -> Result<String, AppError> {
    let Some(candidate) = provided.map(|id| id.trim().to_string()) else {
        return Ok(format!(
            "inspiration-{project_id}-{}",
            chrono::Utc::now().timestamp_millis()
        ));
    };
    if candidate.is_empty() {
        return Err(AppError::InvalidArgument("task_id 不能为空".to_string()));
    }
    if candidate.len() > 160 {
        return Err(AppError::InvalidArgument(
            "task_id 长度不能超过 160 个字符".to_string(),
        ));
    }
    if !candidate
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'))
    {
        return Err(AppError::InvalidArgument(
            "task_id 只能包含 ASCII 字母、数字、-、_、:、.".to_string(),
        ));
    }
    Ok(candidate)
}

pub(super) fn validate_history_limit(limit: u64) -> Result<(), AppError> {
    if limit == 0 {
        Err(AppError::InvalidArgument("limit 必须大于 0".to_string()))
    } else {
        Ok(())
    }
}
