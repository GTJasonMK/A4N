use a4n_error::AppError;

pub(super) fn required_positive_id(value: i64, field: &str) -> Result<i64, AppError> {
    if value <= 0 {
        return Err(AppError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(value)
}
