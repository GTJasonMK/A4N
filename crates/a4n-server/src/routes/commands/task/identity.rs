use super::super::ApiError;

pub(in crate::routes::commands) fn sanitize_task_id(task_id: String) -> Result<String, ApiError> {
    a4n_core::event::channel::normalize_task_id(&task_id).map_err(ApiError::from)
}

pub(in crate::routes::commands) fn sanitize_optional_task_id(
    task_id: Option<String>,
) -> Result<Option<String>, ApiError> {
    task_id.map(sanitize_task_id).transpose()
}
