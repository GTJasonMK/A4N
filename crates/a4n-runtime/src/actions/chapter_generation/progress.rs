use super::contracts::ChapterGenerationProgress;
use a4n_core::error::AppError;
use tokio_util::sync::CancellationToken;

pub(super) fn report_progress(
    progress: &mut Option<ChapterGenerationProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress.as_deref_mut() {
        progress(current, total, message).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(super) fn check_optional_cancelled(
    cancel_token: Option<&CancellationToken>,
) -> Result<(), String> {
    if let Some(cancel_token) = cancel_token {
        check_cancelled(cancel_token)?;
    }
    Ok(())
}

pub(super) fn check_cancelled(cancel_token: &CancellationToken) -> Result<(), String> {
    if cancel_token.is_cancelled() {
        return Err(AppError::Cancelled.to_string());
    }
    Ok(())
}
