use a4n_core::error::AppError;
use tokio_util::sync::CancellationToken;

pub(in crate::blueprint::generation) fn ensure_not_cancelled(
    cancel_token: &CancellationToken,
) -> Result<(), AppError> {
    if cancel_token.is_cancelled() {
        return Err(AppError::Cancelled);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_gate_rejects_cancelled_token() {
        let token = CancellationToken::new();
        token.cancel();

        assert!(matches!(
            ensure_not_cancelled(&token),
            Err(AppError::Cancelled)
        ));
        assert!(ensure_not_cancelled(&CancellationToken::new()).is_ok());
    }
}
