use super::*;

#[test]
fn validate_existing_chapter_outline_for_request_accepts_matching_part() {
    let outline = sample_chapter_outline(12, Some(2));

    validate_existing_chapter_outline_for_request(&outline, 12, Some(2))
        .expect("matching existing chapter outline should be reusable");
}

#[test]
fn validate_existing_chapter_outline_for_request_rejects_part_mismatch() {
    let outline = sample_chapter_outline(12, Some(2));

    let err = validate_existing_chapter_outline_for_request(&outline, 12, Some(3))
        .expect_err("part mismatch must fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("关联分部"));
}

#[test]
fn ensure_response_not_truncated_rejects_length_finish_reason() {
    let err = ensure_response_not_truncated(Some("length"), "第 1 章章纲")
        .expect_err("length finish reason must fail explicitly");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("输出上限"));
    assert!(err.to_string().contains("JSON 被截断"));
}

#[test]
fn ensure_response_not_truncated_rejects_max_tokens_finish_reason() {
    let err = ensure_response_not_truncated(Some("max_tokens"), "第 1 章章纲")
        .expect_err("max_tokens finish reason must fail explicitly");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("输出上限"));
    assert!(err.to_string().contains("JSON 被截断"));
}

#[test]
fn validate_chapter_outline_generation_request_rejects_non_positive_and_overflow() {
    assert!(matches!(
        validate_chapter_outline_generation_request(0, 1),
        Err(AppError::InvalidArgument(_))
    ));
    assert!(matches!(
        validate_chapter_outline_generation_request(1, 0),
        Err(AppError::InvalidArgument(_))
    ));
    assert!(matches!(
        validate_chapter_outline_generation_request(i32::MAX, 2),
        Err(AppError::InvalidArgument(_))
    ));
}

#[test]
fn validate_requested_part_context_rejects_uncovered_chapter() {
    let part = sample_part(Some("星火初燃"));
    let err =
        validate_requested_part_context(&[part], 9, 11).expect_err("uncovered chapter must fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("不在任何分部范围内"));
}

#[test]
fn required_part_summary_rejects_missing_summary() {
    let err = required_part_summary(&sample_part(None)).expect_err("missing summary must fail");
    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn required_part_summary_rejects_blank_summary() {
    let err =
        required_part_summary(&sample_part(Some(" \n\t "))).expect_err("blank summary must fail");
    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn required_part_summary_accepts_trimmed_summary() {
    let part = sample_part(Some("  星火初燃  "));
    assert_eq!(required_part_summary(&part).unwrap(), "星火初燃");
}

#[test]
fn validate_chapter_generation_range_rejects_implicit_or_overflowing_ranges() {
    let part = sample_part(Some("星火初燃"));

    let zero = validate_chapter_generation_range(&part, 11, 0).expect_err("zero count must fail");
    assert!(matches!(zero, AppError::InvalidArgument(_)));

    let before_part = validate_chapter_generation_range(&part, 10, 1)
        .expect_err("start before part range must fail");
    assert!(matches!(before_part, AppError::InvalidArgument(_)));

    let beyond_part = validate_chapter_generation_range(&part, 18, 4)
        .expect_err("end beyond part range must fail");
    assert!(matches!(beyond_part, AppError::InvalidArgument(_)));
}

#[test]
fn validate_chapter_generation_range_accepts_explicit_range() {
    let part = sample_part(Some("星火初燃"));

    let range = validate_chapter_generation_range(&part, 12, 3)
        .expect("explicit in-part range should pass");

    assert_eq!(range, (12, 14));
}
