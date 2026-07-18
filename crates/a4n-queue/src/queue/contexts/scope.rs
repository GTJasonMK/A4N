use a4n_core::error::AppError;

pub(super) fn validate_task_scope(value: String) -> Result<String, AppError> {
    if matches!(
        value.as_str(),
        "text_analysis" | "source_sync" | "video_prompt" | "media_generation" | "assembly"
    ) {
        return Ok(value);
    }
    Err(AppError::InvalidArgument(format!(
        "task_scope 无效: {value}"
    )))
}
