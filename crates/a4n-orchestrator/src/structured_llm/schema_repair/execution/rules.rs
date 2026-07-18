use a4n_core::error::AppError;

use super::contracts::StructuredRepairProgress;

pub(super) fn report_progress(
    progress: Option<StructuredRepairProgress<'_>>,
    message: &str,
) -> Result<(), AppError> {
    if let Some(progress) = progress {
        progress(message)?;
    }
    Ok(())
}

pub(super) fn is_transport_or_cancel_error(error: &AppError) -> bool {
    matches!(error, AppError::Cancelled | AppError::LlmError(_))
}
