#[test]
fn portrait_spec_plan_accepts_expression_and_pose() {
    let spec = portrait_expression_spec(
        10,
        Some(42),
        json!({
            "canonical_key": "galgame:10:portrait:42:expression:smile:pose:standing",
            "character_profile_id": 42,
            "expression": "smile",
            "pose": "standing",
            "position_samples": ["left", "right"],
            "source_line_ids": [1000, 1001]
        }),
    );

    let plan = GalgamePortraitSpecPlan::from_spec(&spec).expect("portrait spec plan");

    assert_eq!(plan.profile_id, 42);
    assert_eq!(plan.expression, "smile");
    assert_eq!(plan.pose.as_deref(), Some("standing"));
    assert_eq!(plan.position_samples, vec!["left", "right"]);
    assert_eq!(plan.source_line_ids, vec![1000, 1001]);
}

#[test]
fn portrait_spec_plan_rejects_missing_expression() {
    let spec = portrait_expression_spec(10, Some(42), json!({"character_profile_id": 42}));

    let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("missing variant fields");

    assert!(err.contains("缺少 expression"));
}

#[test]
fn portrait_spec_plan_rejects_invalid_expression() {
    let spec = portrait_expression_spec(
        10,
        Some(42),
        json!({
            "character_profile_id": 42,
            "expression": "ecstatic"
        }),
    );

    let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("invalid expression");

    assert!(err.contains("expression 无效"));
    assert!(err.contains("ecstatic"));
}

#[test]
fn portrait_spec_plan_rejects_pose_without_expression() {
    let spec = portrait_expression_spec(
        10,
        Some(42),
        json!({
            "character_profile_id": 42,
            "pose": "standing"
        }),
    );

    let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("missing expression");

    assert!(err.contains("缺少 expression"));
}

#[test]
fn portrait_spec_plan_rejects_profile_id_mismatch() {
    let spec = portrait_expression_spec(
        10,
        Some(42),
        json!({
            "character_profile_id": 99,
            "expression": "smile"
        }),
    );

    let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("profile mismatch");

    assert!(err.contains("character_profile_id 与 target_entity_id 不一致"));
}

#[test]
fn portrait_spec_plan_rejects_non_portrait_target() {
    let mut spec = portrait_expression_spec(10, Some(42), json!({"expression": "smile"}));
    spec.target_kind = GALGAME_TARGET_BACKGROUND.to_string();
    spec.target_entity_kind = GALGAME_ENTITY_SCENE.to_string();

    let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("invalid target");

    assert!(err.contains("只支持 portrait_expression/character_profile spec"));
}

#[test]
fn portrait_spec_plan_rejects_canonical_key_mismatch() {
    let spec = portrait_expression_spec(
        10,
        Some(42),
        json!({
            "canonical_key": "galgame:10:portrait:99:expression:smile",
            "character_profile_id": 42,
            "expression": "smile"
        }),
    );

    let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("canonical key mismatch");

    assert!(err.contains("canonical_key 与目标不一致"));
    assert!(err.contains("galgame:10:portrait:99:expression:smile"));
    assert!(err.contains("galgame:10:portrait:42:expression:smile"));
}

#[test]
fn portrait_spec_plan_rejects_missing_source_line_ids() {
    let spec = portrait_expression_spec(
        10,
        Some(42),
        json!({
            "canonical_key": "galgame:10:portrait:42:expression:smile",
            "character_profile_id": 42,
            "expression": "smile",
            "position_samples": []
        }),
    );

    let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("missing source lines");

    assert!(err.contains("source_line_ids 缺失"));
}

#[test]
fn portrait_spec_plan_rejects_empty_source_line_ids() {
    let spec = portrait_expression_spec(
        10,
        Some(42),
        json!({
            "canonical_key": "galgame:10:portrait:42:expression:smile",
            "character_profile_id": 42,
            "expression": "smile",
            "position_samples": [],
            "source_line_ids": []
        }),
    );

    let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("empty source lines");

    assert!(err.contains("source_line_ids 必须是非空数组"));
}

#[test]
fn portrait_spec_plan_rejects_invalid_position_samples() {
    let spec = portrait_expression_spec(
        10,
        Some(42),
        json!({
            "canonical_key": "galgame:10:portrait:42:expression:smile",
            "character_profile_id": 42,
            "expression": "smile",
            "position_samples": ["left", ""],
            "source_line_ids": [1000]
        }),
    );

    let err = GalgamePortraitSpecPlan::from_spec(&spec).expect_err("invalid position sample");

    assert!(err.contains("position_samples 必须是非空字符串数组"));
}

#[test]
fn portrait_position_samples_must_match_source_positions() {
    let samples = vec!["left".to_string()];
    let source_positions = BTreeSet::from(["right".to_string()]);

    let err = validate_galgame_portrait_position_samples(1, &samples, &source_positions)
        .expect_err("portrait position samples must match source line positions");

    assert!(err.contains("position_samples 与 source_line_ids 行站位不一致"));
}

#[test]
fn portrait_position_samples_reject_invalid_position() {
    let samples = vec!["foreground".to_string()];
    let source_positions = BTreeSet::new();

    let err = validate_galgame_portrait_position_samples(1, &samples, &source_positions)
        .expect_err("portrait position samples must use controlled positions");

    assert!(err.contains("position_samples 包含无效站位"));
    assert!(err.contains("foreground"));
}

#[test]
fn portrait_spec_family_context_accepts_matching_project_context() {
    let spec = portrait_expression_spec(
        10,
        Some(42),
        json!({
            "canonical_key": "galgame:10:portrait:42:expression:smile",
            "character_profile_id": 42,
            "expression": "smile"
        }),
    );

    validate_galgame_portrait_spec_family_context(&spec, 1, 20)
        .expect("matching family context should pass");
}

#[test]
fn portrait_spec_family_context_rejects_source_snapshot_mismatch() {
    let spec = portrait_expression_spec(
        10,
        Some(42),
        json!({
            "canonical_key": "galgame:10:portrait:42:expression:smile",
            "character_profile_id": 42,
            "expression": "smile"
        }),
    );

    let err = validate_galgame_portrait_spec_family_context(&spec, 1, 21)
        .expect_err("source snapshot mismatch should fail");

    assert!(err.contains("项目族上下文与项目不一致"));
    assert!(err.contains("spec_source_snapshot_id=Some(20)"));
    assert!(err.contains("project_source_snapshot_id=21"));
}
