use super::support::*;
use super::*;

#[test]
fn validate_package_manifest_rejects_regenerate_scene_stage_output_missing_speaker() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["character_staging"]["scene_lines"][0]["lines"][0]["speaker_profile_id"] =
        json!(999);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene stage output speaker_profile_id must exist in package");

    assert!(err.contains("speaker_profile_id"));
    assert!(err.contains("角色档案不存在"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_result_scene_mismatch() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["galgame_scene_id"] = json!(2202);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package
        .galgame_scenes
        .push(galgame_scene_record_with_id(2202, 1001, "main", None));
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result scene must match the decision target scene");

    assert!(err.contains("result.galgame_scene_id 与决策目标场不一致"));
    assert!(err.contains("galgame_scene_id=2202"));
    assert!(err.contains("expected=2201"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_payload_target_scene_mismatch() {
    let mut payload = regenerate_scene_decision_payload();
    payload["galgame_scene_id"] = json!(2202);
    payload["result"]["galgame_scene_id"] = json!(2202);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package
        .galgame_scenes
        .push(galgame_scene_record_with_id(2202, 1001, "main", None));
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene payload scene must match the conflict target scene");

    assert!(err.contains("regenerate_scene galgame_scene_id 与冲突目标场不一致"));
    assert!(err.contains("source_sync_conflict_id=811"));
    assert!(err.contains("galgame_scene_id=2202"));
    assert!(err.contains("expected=2201"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_result_new_chapter_scene_mismatch() {
    let payload = regenerate_scene_decision_payload();
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.galgame_scenes[0].source_snapshot_chapter_id = 1002;
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result chapter must match the current Galgame scene chapter");

    assert!(
        err.contains("result.new_source_snapshot_chapter_id 与 Galgame 场当前源快照章节不一致")
    );
    assert!(err.contains("galgame_scene_id=2201"));
    assert!(err.contains("scene_source_snapshot_chapter_id=1002"));
    assert!(err.contains("result_new_source_snapshot_chapter_id=1001"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_stage_output_chapter_mismatch() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["shooting_script_projection"]["scenes"][0]["source_snapshot_chapter_id"] =
        json!(1002);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("stage output source_snapshot_chapter_id must match regenerated chapter");

    assert!(err.contains("source_snapshot_chapter_id 与重建目标章节不一致"));
    assert!(err.contains("source_snapshot_chapter_id=1002"));
    assert!(err.contains("expected=1001"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_source_span_chapter_mismatch() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["shooting_script_projection"]["scenes"][0]["source_span_ids"] =
        json!(["ch2:ta702"]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("source_span_ids chapter number must match the referenced text anchor");

    assert!(err.contains("source_span_ids 章节编号与重建目标章节不一致"));
    assert!(err.contains("source_span_id=ch2:ta702"));
    assert!(err.contains("expected=1"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_source_span_outside_target_chapter() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["shooting_script_projection"]["scenes"][0]["source_span_ids"] =
        json!(["ch2:ta703"]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    let mut second_anchor = snapshot_text_anchor_record(&package.source_snapshot_chapters[1]);
    second_anchor.id = 703;
    package.text_anchors.push(second_anchor);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("source_span_ids anchors must belong to the regenerated snapshot chapter");

    assert!(err.contains("source_span_ids 章节编号与重建目标章节不一致"));
    assert!(err.contains("source_span_id=ch2:ta703"));
    assert!(err.contains("expected=1"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_shooting_script_projection_empty_source_span()
{
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["shooting_script_projection"]["scenes"][0]["source_span_ids"] =
        json!([]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene shooting_script_projection scenes must keep source evidence");

    assert!(
        err.contains("result.stage_outputs.shooting_script_projection.scenes[0].source_span_ids")
    );
    assert!(err.contains("必须是非空字符串数组"));
}
