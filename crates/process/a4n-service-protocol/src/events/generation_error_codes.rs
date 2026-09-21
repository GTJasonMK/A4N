/// 规范生成错误码唯一注册表。
const REGISTERED_GENERATION_ERROR_CODES: &[&str] = &[
    "artifact_download_failed",
    "artifact_persist_failed",
    "artifact_validation_failed",
    "generation_attempt_conflict",
    "generation_attempt_mismatch",
    "generation_cancel_too_late",
    "generation_checkpoint_corrupt",
    "generation_checkpoint_unavailable",
    "generation_cursor_invalid",
    "generation_destructive_drain_timeout",
    "generation_event_invalid",
    "generation_interrupted",
    "generation_offset_mismatch",
    "generation_owner_lifecycle_blocked",
    "generation_parse_failed",
    "generation_persist_failed",
    "generation_protocol_unsupported",
    "generation_request_invalid",
    "generation_sequence_gap",
    "generation_session_capacity_exceeded",
    "generation_session_owner_lost",
    "generation_snapshot_expired",
    "generation_snapshot_required",
    "generation_snapshot_too_large",
    "generation_start_request_capacity_exceeded",
    "generation_start_request_conflict",
    "generation_subscription_capacity_exceeded",
    "generation_task_forbidden",
    "generation_task_not_found",
    "generation_task_unauthenticated",
    "generation_terminal_already_set",
    "generation_terminal_persistence_unavailable",
    "generation_validation_failed",
    "llm_output_truncated",
    "llm_stream_connect_timeout",
    "llm_stream_first_content_timeout",
    "llm_stream_idle_timeout",
    "llm_stream_missing_finish_reason",
    "llm_stream_total_timeout",
    "package_unavailable",
    "provider_cancel_unsupported",
    "provider_job_failed",
];

pub(super) fn is_registered_generation_error_code(value: &str) -> bool {
    REGISTERED_GENERATION_ERROR_CODES
        .binary_search(&value)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_sorted_unique_and_well_formed() {
        assert!(
            REGISTERED_GENERATION_ERROR_CODES
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        assert!(REGISTERED_GENERATION_ERROR_CODES.iter().all(|code| {
            let mut bytes = code.bytes();
            matches!(bytes.next(), Some(b'a'..=b'z'))
                && bytes
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        }));
    }
}
