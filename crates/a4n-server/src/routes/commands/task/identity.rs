use super::super::ApiError;

pub(in crate::routes::commands) fn sanitize_task_id(task_id: String) -> Result<String, ApiError> {
    let candidate = task_id.trim().to_string();
    if candidate.is_empty() {
        return Err(ApiError::bad_request("task_id 不能为空"));
    }
    if candidate.len() > 160 {
        return Err(ApiError::bad_request("task_id 长度不能超过 160 个字符"));
    }
    if !candidate
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'))
    {
        return Err(ApiError::bad_request(
            "task_id 只能包含 ASCII 字母、数字、-、_、:、.",
        ));
    }
    Ok(candidate)
}

pub(in crate::routes::commands) fn sanitize_optional_task_id(
    task_id: Option<String>,
) -> Result<Option<String>, ApiError> {
    task_id.map(sanitize_task_id).transpose()
}
