use a4n_core::error::AppError;

pub(super) fn is_cancelled_optimize_error(error: &AppError) -> bool {
    matches!(error, AppError::Cancelled)
}
