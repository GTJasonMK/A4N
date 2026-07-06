use super::super::*;

#[test]
fn ensure_character_sync_response_not_truncated_rejects_length_finish_reason() {
    let err = ensure_character_sync_response_not_truncated(Some("length"), "批量角色同步")
        .expect_err("length finish reason must fail clearly");

    assert!(err.contains("达到当前角色同步输出上限"));
    assert!(err.contains("角色同步"));
}

#[test]
fn ensure_character_sync_response_not_truncated_rejects_max_tokens_finish_reason() {
    let err = ensure_character_sync_response_not_truncated(Some("max_tokens"), "批量角色同步")
        .expect_err("max_tokens finish reason must fail clearly");

    assert!(err.contains("达到当前角色同步输出上限"));
    assert!(err.contains("角色同步"));
}
