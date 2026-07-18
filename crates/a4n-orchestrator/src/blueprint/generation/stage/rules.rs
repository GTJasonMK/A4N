use a4n_core::blueprint::BLUEPRINT_GENERATE_PROGRESS_TOTAL;
use a4n_core::error::AppError;
use tokio_util::sync::CancellationToken;

use crate::blueprint::BlueprintProgressReporter;

pub(in crate::blueprint::generation) fn report(
    progress: BlueprintProgressReporter<'_>,
    current: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(report) = progress {
        report(current, BLUEPRINT_GENERATE_PROGRESS_TOTAL, message)?;
    }
    Ok(())
}

pub(in crate::blueprint::generation) fn ensure_not_cancelled(
    cancel_token: Option<&CancellationToken>,
) -> Result<(), AppError> {
    if cancel_token.is_some_and(CancellationToken::is_cancelled) {
        return Err(AppError::Cancelled);
    }
    Ok(())
}

pub(super) fn blueprint_stage_call_order(progress_current: u32) -> Result<i32, AppError> {
    i32::try_from(progress_current).map_err(|_| {
        AppError::InvalidArgument(format!(
            "蓝图生成阶段 call_order 超出 i32 范围：{progress_current}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_gate_rejects_cancelled_token() {
        let token = CancellationToken::new();
        token.cancel();

        assert!(matches!(
            ensure_not_cancelled(Some(&token)),
            Err(AppError::Cancelled)
        ));
        assert!(ensure_not_cancelled(None).is_ok());
    }

    #[test]
    fn stage_call_order_rejects_u32_overflow() {
        assert_eq!(blueprint_stage_call_order(4).unwrap(), 4);
        let error = blueprint_stage_call_order(i32::MAX as u32 + 1).unwrap_err();

        match error {
            AppError::InvalidArgument(message) => assert_eq!(
                message,
                format!(
                    "蓝图生成阶段 call_order 超出 i32 范围：{}",
                    i32::MAX as u32 + 1
                )
            ),
            other => panic!("unexpected error: {other}"),
        }
    }
}
