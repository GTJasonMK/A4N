use super::support::*;
use super::*;

#[test]
fn validate_package_manifest_accepts_regenerate_scene_stage_outputs() {
    let payload = regenerate_scene_decision_payload();
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    validate_package_manifest(&package)
        .expect("regenerate_scene stage output references should remain importable");
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_result() {
    let mut payload = regenerate_scene_decision_payload();
    payload
        .as_object_mut()
        .expect("fixture payload must be object")
        .remove("result");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("handled regenerate_scene decisions must keep runtime result");

    assert!(err.contains("regenerate_scene result 缺失"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_null_result() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"] = Value::Null;
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("handled regenerate_scene result must not be null");

    assert!(err.contains("regenerate_scene result 必须是 JSON 对象"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_result_scene_id() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]
        .as_object_mut()
        .expect("fixture result must be object")
        .remove("galgame_scene_id");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result must keep rebuilt scene id");

    assert!(err.contains("regenerate_scene result.galgame_scene_id 必须是整数"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_old_snapshot_chapter_id() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]
        .as_object_mut()
        .expect("fixture result must be object")
        .remove("old_source_snapshot_chapter_id");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result must keep old source snapshot chapter id");

    assert!(err.contains("regenerate_scene result.old_source_snapshot_chapter_id 必须是整数"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_new_snapshot_chapter_id() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]
        .as_object_mut()
        .expect("fixture result must be object")
        .remove("new_source_snapshot_chapter_id");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result must keep new source snapshot chapter id");

    assert!(err.contains("regenerate_scene result.new_source_snapshot_chapter_id 必须是整数"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_stage_outputs() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]
        .as_object_mut()
        .expect("fixture result must be object")
        .remove("stage_outputs");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result must keep runtime stage outputs");

    assert!(err.contains("regenerate_scene result.stage_outputs 缺失"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_null_stage_outputs() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"] = Value::Null;
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result stage_outputs must not be null");

    assert!(err.contains("regenerate_scene result.stage_outputs 必须是 JSON 对象"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_result_count() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]
        .as_object_mut()
        .expect("fixture result must be object")
        .remove("deleted_portrait_generation_spec_count");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result cleanup counters must be complete");

    assert!(err.contains("result.deleted_portrait_generation_spec_count 缺失"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_invalid_result_count() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["deleted_portrait_generation_spec_count"] = json!(-1);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result cleanup counters must be non-negative integers");

    assert!(err.contains("result.deleted_portrait_generation_spec_count 必须是非负整数"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_unknown_payload_field() {
    let mut payload = regenerate_scene_decision_payload();
    payload["silent_fallback"] = json!(true);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene payload must reject unknown fields");

    assert!(err.contains("regenerate_scene 决策载荷.silent_fallback 是未知字段"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_user_action() {
    let mut payload = regenerate_scene_decision_payload();
    payload
        .as_object_mut()
        .expect("fixture payload must be object")
        .remove("user_action");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene payload must keep explicit user_action");

    assert!(err.contains("regenerate_scene 决策载荷.user_action 必须是字符串"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_invalid_user_action() {
    let mut payload = regenerate_scene_decision_payload();
    payload["user_action"] = json!("keep_local");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene payload user_action must match the strategy");

    assert!(err.contains("regenerate_scene 决策载荷.user_action 无效"));
    assert!(err.contains("user_action=keep_local"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_invalid_reason_type() {
    let mut payload = regenerate_scene_decision_payload();
    payload["reason"] = json!({"kind": "source_changed"});
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene payload reason must keep service JSON shape");

    assert!(err.contains("regenerate_scene 决策载荷.reason 必须是字符串或 null"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_invalid_result_mode() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["mode"] = json!("baseline_fallback");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result mode must remain explicit LLM partial regeneration");

    assert!(err.contains("regenerate_scene result.mode 无效"));
    assert!(err.contains("baseline_fallback"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_unknown_result_field() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["legacy_summary"] = json!("fallback rebuilt from anchors");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result must reject unknown fields");

    assert!(err.contains("regenerate_scene result.legacy_summary 是未知字段"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_stage_output() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]
        .as_object_mut()
        .expect("fixture stage_outputs must be object")
        .remove("visual_planning");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene stage_outputs must contain all runtime stages");

    assert!(err.contains("result.stage_outputs.visual_planning 缺失"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_unknown_stage_output() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["legacy_anchor_rebuild"] = json!({});
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene stage_outputs must reject unknown stages");

    assert!(err.contains("regenerate_scene result.stage_outputs.legacy_anchor_rebuild 是未知字段"));
}
