use super::support::*;
use super::*;

#[test]
fn validate_package_manifest_rejects_regenerate_scene_script_adaptation_missing_scene_lines() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["script_adaptation"]
        .as_object_mut()
        .expect("fixture script_adaptation must be object")
        .remove("scene_lines");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene script_adaptation must contain scene_lines");

    assert!(err.contains("stage_outputs.script_adaptation"));
    assert!(err.contains("missing field `scene_lines`"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_script_adaptation_unknown_field() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["script_adaptation"]["legacy_lines"] = json!([]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene script_adaptation must reject unknown stage fields");

    assert!(err.contains("stage_outputs.script_adaptation"));
    assert!(err.contains("unknown field `legacy_lines`"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_script_adaptation_multiple_scenes() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["script_adaptation"]["scene_lines"]
        .as_array_mut()
        .expect("fixture script_adaptation scene_lines must be array")
        .push(json!({
            "scene_index": 2,
            "lines": [{
                "index": 1,
                "origin": "adapted", "beat_ref": {"source_chapter_id":2001,"unit_id":"opening","beat_id":"narration"},
                "line_type": "narration",
                "text": "第二场不允许出现在局部重跑结果。",
                "source_span_ids": ["ch1:ta702"]
            }]
        }));
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene script_adaptation must remain single-scene");

    assert!(err.contains("局部重跑必须返回且只返回 scene_index=1 的非空剧本行"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_script_adaptation_line_without_source_span() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["script_adaptation"]["scene_lines"][0]["lines"][0]["source_span_ids"] =
        json!([]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene script_adaptation lines must keep source evidence");

    assert!(err.contains("stage_outputs.script_adaptation.lines[].source_span_ids"));
    assert!(err.contains("必须保留原文引用"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_character_staging_line_missing_source_span() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["character_staging"]["scene_lines"][0]["lines"][0]
        .as_object_mut()
        .expect("fixture staged line must be object")
        .remove("source_span_ids");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene character_staging lines must keep source evidence");

    assert!(err.contains("stage_outputs.character_staging"));
    assert!(err.contains("missing field `source_span_ids`"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_visual_planning_event_cg_wrong_scene() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["visual_planning"]["event_cgs"] = json!([{
        "scene_index": 2,
        "line_index": 1,
        "cg_note": "跨场 CG 不允许出现在局部重跑结果。",
        "importance": "normal",
        "source_span_ids": ["ch1:ta702"]
    }]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene visual_planning event CG must reference scene_index=1");

    assert!(err.contains("局部重跑事件 CG 不能引用其他场"));
    assert!(err.contains("scene_index=2"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_visual_planning_background_empty_source_span()
{
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["visual_planning"]["backgrounds"][0]["source_span_ids"] =
        json!([]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene visual_planning backgrounds must keep source evidence");

    assert!(err.contains("stage_outputs.visual_planning.backgrounds[0].source_span_ids"));
    assert!(err.contains("必须保留原文引用"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_visual_planning_missing_backgrounds() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["visual_planning"]
        .as_object_mut()
        .expect("fixture visual_planning must be object")
        .remove("backgrounds");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene visual_planning must contain backgrounds");

    assert!(err.contains("stage_outputs.visual_planning"));
    assert!(err.contains("missing field `backgrounds`"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_visual_planning_missing_event_cgs() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["visual_planning"]
        .as_object_mut()
        .expect("fixture visual_planning must be object")
        .remove("event_cgs");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene visual_planning must contain event_cgs");

    assert!(err.contains("stage_outputs.visual_planning"));
    assert!(err.contains("missing field `event_cgs`"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_visual_planning_event_cg_missing_source_span()
{
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["visual_planning"]["event_cgs"] = json!([{
        "scene_index": 1,
        "line_index": 1,
        "cg_note": "缺少 source_span_ids 的 CG 不允许导入。",
        "importance": "normal"
    }]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene visual_planning event CGs must keep source evidence");

    assert!(err.contains("stage_outputs.visual_planning"));
    assert!(err.contains("missing field `source_span_ids`"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_branch_design_missing_choices() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["branch_design"]
        .as_object_mut()
        .expect("fixture branch_design must be object")
        .remove("choices");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene branch_design must contain choices");

    assert!(err.contains("stage_outputs.branch_design"));
    assert!(err.contains("missing field `choices`"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_branch_design_choices() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["branch_design"] = json!({
        "choices": [{
            "after_beat_ref": {"source_chapter_id":2001,"unit_id":"opening","beat_id":"narration"},
            "question": "要不要追上去？",
            "options": [
                {
                    "branch_id": "catch_up",
                    "option_text": "追上去",
                    "branch_title": "追逐",
                    "branch_objective": "追上对方确认信息",
                    "dramatic_action": "主角加快脚步追上对方",
                    "reaction": "对方回头停下",
                    "player_knowledge": ["对方愿意等待"],
                    "returns_to": {"source_chapter_id":2001,"unit_id":"opening"}
                },
                {
                    "branch_id": "stay",
                    "option_text": "留下",
                    "branch_title": "停留",
                    "branch_objective": "留在原地观察",
                    "dramatic_action": "主角停在路口",
                    "reaction": "对方独自离开",
                    "player_knowledge": ["分离会改变后续关系"],
                    "returns_to": {"source_chapter_id":2001,"unit_id":"opening"}
                }
            ]
        }]
    });
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene stage_outputs branch_design must not add choices");

    assert!(err.contains("stage_outputs.branch_design 不允许包含 choices"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_branch_design_non_array_choices() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["branch_design"] = json!({
        "choices": {}
    });
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene stage_outputs branch_design choices must be an array");

    assert!(err.contains("stage_outputs.branch_design"));
    assert!(err.contains("expected a sequence"));
}
