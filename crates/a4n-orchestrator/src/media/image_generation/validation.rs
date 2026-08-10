use a4n_core::error::AppError;

pub(crate) fn require_positive(value: i64, label: &str) -> Result<(), AppError> {
    if value <= 0 {
        Err(AppError::InvalidArgument(format!("{label} 必须大于 0")))
    } else {
        Ok(())
    }
}

pub(crate) fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{label} 不能为空")))
    } else {
        Ok(value)
    }
}
