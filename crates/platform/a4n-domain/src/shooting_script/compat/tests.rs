use super::*;
use serde_json::json;

fn fixture() -> Value {
    let mut value =
        serde_json::to_value(crate::shooting_script::validation::tests::valid_script()).unwrap();
    value["schema_version"] = json!("chapter_shooting_script_v1");
    let shot = &mut value["scenes"][0]["shots"][0];
    shot["estimated_natural_seconds"] = json!(2.0);
    shot["performance_beats"][0]["estimated_seconds"] = json!(2.0);
    value
}

#[test]
fn legacy_upgrade_preserves_content_and_rejects_corrupt_identity() {
    let mut value = fixture();
    let legacy: v1::ChapterShootingScriptV1 = serde_json::from_value(value.clone()).unwrap();
    legacy.validate().unwrap();
    let old_hash = serialized_sha256(&legacy).unwrap();
    let mut mappings = HashUpgrades::default();
    let hash = mappings.upgrade_record(&mut value, &old_hash).unwrap();
    assert_eq!(
        value,
        serde_json::to_value(crate::shooting_script::validation::tests::valid_script()).unwrap()
    );
    assert_ne!(hash, old_hash);
    assert_eq!(mappings.upgrade_record(&mut value, &hash).unwrap(), hash);
    let mut corrupt = fixture();
    corrupt["scenes"][0]["title"] = json!("modified without updating integrity hash");
    assert!(
        mappings
            .upgrade_record(&mut corrupt, &old_hash)
            .unwrap_err()
            .contains("哈希")
    );
    let mut references = json!({"shooting_script_identity": {"record_id": 17, "revision": 2, "script_sha256": old_hash}, "already_stale": {"script_sha256": "unmatched"}});
    mappings.rewrite_references(&mut references);
    assert_eq!(
        references["shooting_script_identity"]["script_sha256"],
        hash
    );
    assert_eq!(references["shooting_script_identity"]["record_id"], 17);
    assert_eq!(references["already_stale"]["script_sha256"], "unmatched");
}

#[test]
fn legacy_estimates_do_not_gate_retained_content() {
    let mut value = fixture();
    value["scenes"][0]["shots"][0]["estimated_natural_seconds"] = json!(-1.0);
    value["scenes"][0]["shots"][0]["performance_beats"][0]["estimated_seconds"] = json!(0.0);
    let legacy: v1::ChapterShootingScriptV1 = serde_json::from_value(value.clone()).unwrap();
    let hash = serialized_sha256(&legacy).unwrap();
    HashUpgrades::default()
        .upgrade_record(&mut value, &hash)
        .unwrap();
    assert_eq!(
        value,
        serde_json::to_value(crate::shooting_script::validation::tests::valid_script()).unwrap()
    );
}

#[test]
fn progress_upgrade_keeps_cursor_raw_evidence_and_unrelated_fields() {
    let raw = fixture().to_string();
    let mut progress = json!({"snapshot": {"version": 1, "progress": {
        "workflow_run_id": 81, "completed_unit_count": 1, "total_unit_count": 2,
        "script": fixture()
    }}, "response_content": raw, "author_metadata": {"estimated_seconds": "keep this unrelated field"},
    "result_ref": {"kind": "chapter_shooting_script_progress", "id": "81", "revision": crate::generation::canonical_json_sha256(&fixture()).unwrap()}});
    let mut mappings = HashUpgrades::default();
    assert!(mappings.upgrade_embedded(&mut progress).unwrap());
    assert!(!mappings.upgrade_embedded(&mut progress).unwrap());
    assert!(mappings.rewrite_references(&mut progress));
    assert!(!mappings.rewrite_references(&mut progress));
    assert_eq!(
        progress["result_ref"]["revision"],
        crate::generation::canonical_json_sha256(&progress["snapshot"]["progress"]["script"])
            .unwrap()
    );
    assert_eq!(progress["snapshot"]["progress"]["workflow_run_id"], 81);
    assert_eq!(progress["snapshot"]["progress"]["completed_unit_count"], 1);
    assert_eq!(progress["snapshot"]["progress"]["total_unit_count"], 2);
    assert_eq!(progress["response_content"], raw);
    assert_eq!(
        progress["author_metadata"]["estimated_seconds"],
        "keep this unrelated field"
    );
    assert_eq!(
        progress["snapshot"]["progress"]["script"]["schema_version"],
        "chapter_shooting_script_v2"
    );
}

#[test]
fn completed_calls_require_intact_fingerprints_and_preserve_wire_evidence() {
    use super::checkpoint::{
        TIMING_UPGRADE_KEY, upgrade_completed_checkpoint, validate_checkpoint_upgrade,
    };
    use crate::generation::canonical_json_sha256;

    let step = "shooting_design_shot:1:shot-1";
    let scene = fixture()["scenes"][0].clone();
    let input =
        json!({"messages": [{"role": "system", "content": "ShootingSceneV1 estimated_seconds"}]});
    let input_hash = canonical_json_sha256(&input).unwrap();
    let original = json!({"parsed_output": scene, "response_content": scene.to_string(), "usage": {"total_tokens": 23}, "finish_reason": "stop"});
    let mut checkpoint = original.clone();
    let mut artifacts = None;
    let mut hashes = HashUpgrades::default();
    assert!(
        upgrade_completed_checkpoint(
            step,
            None,
            Some(&input_hash),
            &mut checkpoint,
            &mut artifacts,
            &mut hashes
        )
        .unwrap_err()
        .contains("缺少输入指纹")
    );
    assert!(
        upgrade_completed_checkpoint(
            step,
            Some(&input),
            None,
            &mut checkpoint,
            &mut artifacts,
            &mut hashes
        )
        .unwrap_err()
        .contains("缺少输入指纹哈希")
    );
    assert!(
        upgrade_completed_checkpoint(
            step,
            Some(&input),
            Some("corrupt"),
            &mut checkpoint,
            &mut artifacts,
            &mut hashes
        )
        .unwrap_err()
        .contains("损坏")
    );
    assert!(
        upgrade_completed_checkpoint(
            step,
            Some(&input),
            Some(&input_hash),
            &mut checkpoint,
            &mut artifacts,
            &mut hashes
        )
        .unwrap()
    );
    assert!(
        !upgrade_completed_checkpoint(
            step,
            Some(&input),
            Some(&input_hash),
            &mut checkpoint,
            &mut artifacts,
            &mut hashes
        )
        .unwrap()
    );
    assert_eq!(checkpoint["response_content"], original["response_content"]);
    assert_eq!(
        artifacts.as_ref().unwrap()[TIMING_UPGRADE_KEY]["original_checkpoint"],
        original
    );
    assert!(
        checkpoint["parsed_output"]["shots"][0]
            .get("estimated_natural_seconds")
            .is_none()
    );
    let mut corrupt = checkpoint.clone();
    corrupt["usage"]["total_tokens"] = json!(999);
    assert!(validate_checkpoint_upgrade(&corrupt, artifacts.as_ref().unwrap()).is_err());
    corrupt = checkpoint;
    corrupt["parsed_output"]["shots"][0]["purpose"] = json!("changed");
    assert!(validate_checkpoint_upgrade(&corrupt, artifacts.as_ref().unwrap()).is_err());
}

#[test]
fn embedded_legacy_scripts_drop_removed_visual_constraint_events() {
    use crate::chapter_visual_plan::{ChapterVisualPlanV3, SceneBindingV3};

    let mut value = fixture();
    let mut plan = ChapterVisualPlanV3::starter();
    plan.segments[0].prompt = "推进当前动作".into();
    plan.segments[0].scene_binding = Some(SceneBindingV3::Scene { scene_profile_id: 1 });
    let projection = plan.project().unwrap().remove(0);
    value["scenes"][0]["shots"][0]["visual_constraints"] =
        serde_json::to_value(projection).unwrap();
    value["scenes"][0]["shots"][0]["visual_constraints"]["camera_events"] =
        serde_json::json!([]);

    let mut hashes = HashUpgrades::default();
    assert!(hashes.upgrade_embedded(&mut value).unwrap());
    assert!(value["scenes"][0]["shots"][0]["visual_constraints"]
        .get("camera_events")
        .is_none());
}

#[test]
fn completed_scene_checkpoint_drops_removed_visual_constraint_events() {
    use super::checkpoint::upgrade_completed_checkpoint;
    use crate::chapter_visual_plan::{ChapterVisualPlanV3, SceneBindingV3};
    use crate::generation::canonical_json_sha256;

    let mut plan = ChapterVisualPlanV3::starter();
    plan.segments[0].prompt = "推进当前动作".into();
    plan.segments[0].scene_binding = Some(SceneBindingV3::Scene { scene_profile_id: 1 });
    let projection = plan.project().unwrap().remove(0);
    let mut scene = fixture()["scenes"][0].clone();
    scene["shots"][0]["visual_constraints"] = serde_json::to_value(projection).unwrap();
    scene["shots"][0]["visual_constraints"]["camera_events"] = json!([]);

    let step = "shooting_design_shot:1:shot-1";
    let input = json!({"legacy_contract": "ShootingSceneV1 estimated_seconds"});
    let input_hash = canonical_json_sha256(&input).unwrap();
    let response_content = scene.to_string();
    let mut checkpoint = json!({
        "parsed_output": scene,
        "response_content": response_content,
        "usage": {"total_tokens": 23},
        "finish_reason": "stop"
    });
    let original_response = checkpoint["response_content"].clone();
    let mut artifacts = None;

    assert!(upgrade_completed_checkpoint(
        step,
        Some(&input),
        Some(&input_hash),
        &mut checkpoint,
        &mut artifacts,
        &mut HashUpgrades::default(),
    )
    .unwrap());
    assert_eq!(checkpoint["response_content"], original_response);
    assert!(checkpoint["parsed_output"]["shots"][0]["visual_constraints"]
        .get("camera_events")
        .is_none());
}
