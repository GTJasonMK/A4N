use a4n_core::error::AppError;

use super::{safe_workflow_task_id, sanitize_optional_task_id, string_action_result};

#[test]
fn task_id_uses_default_only_when_missing() {
    assert_eq!(
        safe_workflow_task_id(None, || "generated-task".to_string()).unwrap(),
        "generated-task"
    );
    assert_eq!(
        safe_workflow_task_id(Some("  task:1.test_id  ".to_string()), String::new).unwrap(),
        "task:1.test_id"
    );
}

#[test]
fn task_id_rejects_blank_oversized_and_non_ascii_safe_values() {
    assert!(safe_workflow_task_id(Some("  ".to_string()), String::new).is_err());
    assert!(safe_workflow_task_id(Some("a".repeat(161)), String::new).is_err());
    assert!(safe_workflow_task_id(Some("task/path".to_string()), String::new).is_err());
    assert!(safe_workflow_task_id(Some("任务".to_string()), String::new).is_err());
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
