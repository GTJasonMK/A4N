use super::*;

#[test]
fn ensure_continue_part_request_requires_next_part_and_chapter() {
    let part = sample_part(Some("星火初燃"));
    let wrong_number = ensure_continue_part_request(std::slice::from_ref(&part), 4, 21)
        .expect_err("part number must be next");
    assert!(wrong_number.to_string().contains("part_number 必须等于 3"));

    let wrong_start = ensure_continue_part_request(std::slice::from_ref(&part), 3, 22)
        .expect_err("start chapter must be next");
    assert!(
        wrong_start
            .to_string()
            .contains("start_chapter 必须等于 21")
    );

    ensure_continue_part_request(&[part], 3, 21).expect("next part should pass");
}

#[test]
fn validate_part_generation_inputs_rejects_non_positive_values() {
    assert!(matches!(
        validate_part_generation_inputs(0, 20),
        Err(AppError::InvalidArgument(_))
    ));
    assert!(matches!(
        validate_part_generation_inputs(60, 0),
        Err(AppError::InvalidArgument(_))
    ));
}

#[test]
fn validate_part_generation_inputs_accepts_part_count_and_part_size() {
    validate_part_generation_inputs(3, 20).expect("valid part plan");
    validate_part_generation_inputs(4, 20).expect("valid part plan");
}

#[test]
fn resolve_part_generation_plan_defaults_to_one_part() {
    let plan = resolve_part_generation_plan(None, Some(20), Some(25), Some(10)).unwrap();
    assert_eq!(
        plan,
        PartGenerationPlan {
            chapters_per_part: 20,
            requested_part_count: 1,
        }
    );
}

#[test]
fn resolve_part_generation_plan_uses_requested_part_count() {
    let plan = resolve_part_generation_plan(Some(3), None, Some(25), Some(10)).unwrap();
    assert_eq!(plan.requested_part_count, 3);
    assert_eq!(plan.chapters_per_part, 25);
}

#[test]
fn resolve_part_generation_plan_uses_blueprint_part_size_when_project_missing_it() {
    let plan = resolve_part_generation_plan(None, None, None, Some(12)).unwrap();
    assert_eq!(plan.requested_part_count, 1);
    assert_eq!(plan.chapters_per_part, 12);
}

#[test]
fn resolve_part_generation_plan_rejects_missing_part_size() {
    let err = resolve_part_generation_plan(None, None, None, None)
        .expect_err("missing chapters_per_part must fail explicitly");
    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn validate_existing_parts_for_plan_accepts_valid_partial_parts() {
    let parts = vec![sample_part_for_plan(2, 26, 50)];

    validate_existing_parts_for_plan(&parts)
        .expect("valid existing parts should be kept for resume");
}

#[test]
fn validate_existing_parts_for_plan_rejects_invalid_part_number() {
    let parts = vec![sample_part_for_plan(0, 101, 125)];

    let err = validate_existing_parts_for_plan(&parts)
        .expect_err("invalid existing part number must fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("分部编号无效"));
}

#[test]
fn validate_existing_parts_for_plan_rejects_invalid_range() {
    let parts = vec![sample_part_for_plan(2, 55, 30)];

    let err = validate_existing_parts_for_plan(&parts).expect_err("range mismatch must fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("章节范围"));
}

#[test]
fn validate_existing_parts_for_plan_rejects_overlapping_ranges() {
    let parts = vec![
        sample_part_for_plan(1, 1, 20),
        sample_part_for_plan(2, 20, 40),
    ];

    let err =
        validate_existing_parts_for_plan(&parts).expect_err("overlapping part ranges must fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("章节范围重叠"));
    assert!(err.to_string().contains("第 1 部 1..=20"));
    assert!(err.to_string().contains("第 2 部 20..=40"));
}
