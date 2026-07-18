use a4n_core::error::AppError;

use super::super::ensure_chapter_response_not_truncated;

#[test]
fn ensure_chapter_response_not_truncated_rejects_length_finish_reason() {
    let err = ensure_chapter_response_not_truncated(Some("length"), "第 1 章正文")
        .expect_err("length finish reason must fail explicitly");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("输出上限"));
    assert!(err.to_string().contains("JSON 被截断"));
}

#[test]
fn ensure_chapter_response_not_truncated_rejects_max_tokens_finish_reason() {
    let err = ensure_chapter_response_not_truncated(Some("max_tokens"), "第 1 章正文")
        .expect_err("max_tokens finish reason must fail explicitly");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("输出上限"));
    assert!(err.to_string().contains("JSON 被截断"));
}
