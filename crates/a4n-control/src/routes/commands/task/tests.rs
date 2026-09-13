use a4n_error::AppError;

use super::string_action_result;

#[test]
fn string_action_error_classification_prefers_cancellation_state() {
    let cancelled = string_action_result::<()>(Err("provider failed".to_string()), true);
    assert!(matches!(cancelled, Err(AppError::Cancelled)));

    let failed = string_action_result::<()>(Err("provider failed".to_string()), false);
    assert!(matches!(
        failed,
        Err(AppError::InvalidArgument(message)) if message == "provider failed"
    ));
}
