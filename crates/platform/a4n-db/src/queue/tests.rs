use super::QueueDbError;
use super::concurrency::positive_concurrency;
use super::status::{TaskStatusTimestampUpdate, is_terminal_status, task_status_timestamp_update};

#[test]
fn positive_concurrency_accepts_positive_values() {
    assert_eq!(positive_concurrency(1, "默认 LLM 配置").unwrap(), 1);
    assert_eq!(positive_concurrency(8, "默认图片配置").unwrap(), 8);
}

#[test]
fn positive_concurrency_rejects_zero_and_negative_values() {
    let zero = positive_concurrency(0, "默认视频配置").unwrap_err();
    assert!(matches!(zero, QueueDbError::InvalidConfig(_)));
    assert_eq!(zero.to_string(), "默认视频配置 max_concurrent 必须大于 0");

    let negative = positive_concurrency(-1, "默认 LLM 配置").unwrap_err();
    assert!(matches!(negative, QueueDbError::InvalidConfig(_)));
    assert_eq!(
        negative.to_string(),
        "默认 LLM 配置 max_concurrent 必须大于 0"
    );
}

#[test]
fn task_status_terminal_detection_matches_persisted_terminal_states() {
    for status in ["completed", "failed", "cancelled", "interrupted"] {
        assert!(is_terminal_status(status), "{status} should be terminal");
    }
    for status in ["pending", "running", "queued", "unknown"] {
        assert!(
            !is_terminal_status(status),
            "{status} should not be terminal"
        );
    }
}

#[test]
fn task_status_timestamp_rules_match_queue_lifecycle() {
    assert_eq!(
        task_status_timestamp_update("running"),
        TaskStatusTimestampUpdate::Started
    );
    for status in ["completed", "failed", "cancelled", "interrupted"] {
        assert_eq!(
            task_status_timestamp_update(status),
            TaskStatusTimestampUpdate::Completed,
            "{status} should update completed_at"
        );
    }
    for status in ["pending", "queued", "unknown"] {
        assert_eq!(
            task_status_timestamp_update(status),
            TaskStatusTimestampUpdate::None,
            "{status} should not update lifecycle timestamps"
        );
    }
}
