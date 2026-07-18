use a4n_core::error::AppError;

pub(in crate::blueprint::json) fn validate_enum(
    value: &str,
    key: &str,
    context: &str,
    allowed: &[&str],
) -> Result<(), AppError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "{context}.{key} 必须是以下值之一：{}",
            allowed.join(", ")
        )))
    }
}
