use a4n_error::AppError;

pub(in crate::chapter_analysis::owner) fn analysis_schema_error(
    message: impl Into<String>,
) -> AppError {
    AppError::InvalidArgument(message.into())
}
