use a4n_core::error::AppError;

use super::{sanitize_optional_task_id, string_action_result};

#[test]
fn optional_task_id_rejects_blank_oversized_and_unsafe_values() {
    assert!(sanitize_optional_task_id(Some("  ".to_string())).is_err());
    assert!(sanitize_optional_task_id(Some("a".repeat(161))).is_err());
    assert!(sanitize_optional_task_id(Some("task/path".to_string())).is_err());
    assert!(sanitize_optional_task_id(Some("任务".to_string())).is_err());
}

#[test]
fn optional_task_id_preserves_none_and_validates_present_values() {
    assert_eq!(sanitize_optional_task_id(None).unwrap(), None);
    assert_eq!(
        sanitize_optional_task_id(Some(" task-2 ".to_string())).unwrap(),
        Some("task-2".to_string())
    );
    assert!(sanitize_optional_task_id(Some("task/path".to_string())).is_err());
}

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
