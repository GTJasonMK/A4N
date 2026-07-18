use super::support::*;
use super::*;

#[test]
fn validate_package_manifest_rejects_unknown_source_sync_decision_actor() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut decision = source_sync_decision_record("keep_local");
    decision.decided_by = "robot".to_string();
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("source sync decision actor must use service-generated values");

    assert!(err.contains("源同步决策人无效"));
    assert!(err.contains("source_sync_decision_id=821"));
    assert!(err.contains("decided_by=robot"));
}

#[test]
fn validate_package_manifest_accepts_known_source_sync_decision_actors() {
    for actor in DECISION_ACTORS {
        let mut decision = source_sync_decision_record("keep_local");
        decision.decided_by = (*actor).to_string();
        let mut package =
            package_with_source_sync_conflict(handled_source_sync_conflict_for_decision(
                CONFLICT_STATUS_RESOLVED,
                "keep_local",
                decision.decision_payload_json.clone(),
            ));
        package.source_sync_decisions = vec![decision];

        validate_package_manifest(&package)
            .expect("known source sync decision actor should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_unknown_source_sync_decision_strategy() {
    let mut package = package_with_part_outlines(Vec::new());
    package.source_sync_decisions = vec![source_sync_decision_record("archive")];

    let err = validate_package_manifest(&package)
        .expect_err("source sync decision strategy must use known values");

    assert!(err.contains("源同步决策策略无效"));
    assert!(err.contains("source_sync_decision_id=821"));
    assert!(err.contains("strategy=archive"));
}

#[test]
fn validate_package_manifest_accepts_known_source_sync_decision_strategies() {
    for strategy in RESOLUTION_STRATEGIES {
        let decision_payload_json = if *strategy == "regenerate_scene" {
            regenerate_scene_decision_payload()
        } else {
            json!({})
        };
        let mut decision = source_sync_decision_record(strategy);
        decision.decision_payload_json = decision_payload_json.clone();
        let mut package = if *strategy == "regenerate_scene" {
            package_with_regenerate_scene_decision(decision_payload_json)
        } else {
            package_with_source_sync_conflict(handled_source_sync_conflict_for_decision(
                CONFLICT_STATUS_RESOLVED,
                strategy,
                decision_payload_json,
            ))
        };
        package.source_sync_decisions = vec![decision];

        validate_package_manifest(&package)
            .expect("known source sync decision strategy should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_non_object_source_sync_decision_payload() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut decision = source_sync_decision_record("keep_local");
    decision.decision_payload_json = json!(["not", "an", "object"]);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("source sync decision payload must keep service object invariant");

    assert!(err.contains("源同步决策载荷必须是 JSON 对象"));
    assert!(err.contains("source_sync_decision_id=821"));
}

#[test]
fn validate_package_manifest_rejects_source_sync_decision_for_pending_conflict() {
    let mut package = package_with_source_sync_conflict(source_sync_conflict_record());
    package.source_sync_decisions = vec![source_sync_decision_record("keep_local")];

    let err = validate_package_manifest(&package)
        .expect_err("source sync decisions require the referenced conflict to be handled");

    assert!(err.contains("源同步决策引用了待处理源同步冲突"));
    assert!(err.contains("source_sync_decision_id=821"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_source_sync_decision_strategy_mismatch() {
    let decision = source_sync_decision_record("manual");
    let mut package = package_with_source_sync_conflict(handled_source_sync_conflict_for_decision(
        CONFLICT_STATUS_RESOLVED,
        "keep_local",
        decision.decision_payload_json.clone(),
    ));
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("source sync decision strategy must match conflict resolution_strategy");

    assert!(err.contains("源同步决策策略与冲突解决策略不一致"));
    assert!(err.contains("source_sync_decision_id=821"));
    assert!(err.contains("source_sync_conflict_id=811"));
    assert!(err.contains("decision_strategy=manual"));
}

#[test]
fn validate_package_manifest_rejects_source_sync_decision_payload_mismatch() {
    let decision = source_sync_decision_record("keep_local");
    let mut package = package_with_source_sync_conflict(handled_source_sync_conflict_for_decision(
        CONFLICT_STATUS_RESOLVED,
        "keep_local",
        json!({"reason": "different"}),
    ));
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("source sync decision payload must match conflict decision_json");

    assert!(err.contains("源同步决策载荷与冲突决策载荷不一致"));
    assert!(err.contains("source_sync_decision_id=821"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_accepts_source_sync_decision_matching_conflict() {
    let decision = source_sync_decision_record("keep_local");
    let mut package = package_with_source_sync_conflict(handled_source_sync_conflict_for_decision(
        CONFLICT_STATUS_RESOLVED,
        "keep_local",
        decision.decision_payload_json.clone(),
    ));
    package.source_sync_decisions = vec![decision];

    validate_package_manifest(&package)
        .expect("source sync decision matching its handled conflict should import");
}

#[test]
fn validate_package_manifest_rejects_duplicate_source_sync_decision_for_conflict() {
    let decision = source_sync_decision_record("keep_local");
    let mut duplicate = source_sync_decision_record("keep_local");
    duplicate.id = 822;
    let mut package = package_with_source_sync_conflict(handled_source_sync_conflict_for_decision(
        CONFLICT_STATUS_RESOLVED,
        "keep_local",
        decision.decision_payload_json.clone(),
    ));
    package.source_sync_decisions = vec![decision, duplicate];

    let err = validate_package_manifest(&package)
        .expect_err("a handled source sync conflict can only have one decision record");

    assert!(err.contains("重复 source_sync_decisions.conflict_id"));
    assert!(err.contains("811"));
}
