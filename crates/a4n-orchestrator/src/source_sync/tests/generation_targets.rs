use serde_json::json;

use super::super::decisions::{
    ensure_decision_targets_generation_spec, generation_spec_id_for_conflict,
};
use super::support::conflict_with_target;

#[test]
fn generation_spec_decision_target_uses_conflict_target_for_generation_spec_scope() {
    let conflict = conflict_with_target("generation_spec", "segment", 42, json!({}));

    assert_eq!(
        generation_spec_id_for_conflict(&conflict, "rebuild_prompt").unwrap(),
        42
    );
    ensure_decision_targets_generation_spec(&conflict, 42, "rebuild_prompt")
        .expect("matching generation_spec target should pass");
}

#[test]
fn generation_spec_decision_target_uses_conflict_target_for_text_anchor_segment_scope() {
    let conflict = conflict_with_target("text_anchor", "segment", 43, json!({}));

    assert_eq!(
        generation_spec_id_for_conflict(&conflict, "rebind_anchor").unwrap(),
        43
    );
}

#[test]
fn generation_spec_decision_target_uses_local_summary_for_profile_scope() {
    let conflict = conflict_with_target(
        "profile",
        "character",
        7,
        json!({ "generation_spec_id": 44 }),
    );

    assert_eq!(
        generation_spec_id_for_conflict(&conflict, "manual").unwrap(),
        44
    );
}

#[test]
fn generation_spec_decision_target_rejects_payload_for_other_spec() {
    let conflict = conflict_with_target("generation_spec", "segment", 42, json!({}));

    let err = ensure_decision_targets_generation_spec(&conflict, 99, "manual")
        .expect_err("decision payload must target the conflicted generation_spec");

    assert!(err.to_string().contains("决策目标生成意图不匹配"));
    assert!(err.to_string().contains("generation_spec=42"));
    assert!(err.to_string().contains("generation_spec=99"));
}

#[test]
fn generation_spec_decision_target_rejects_unbound_conflict() {
    let conflict = conflict_with_target("text_anchor", "chapter", 9, json!({}));

    let err = generation_spec_id_for_conflict(&conflict, "delete_local")
        .expect_err("generation_spec decisions need an affected generation_spec");

    assert!(err.to_string().contains("未提供 generation_spec_id"));
}
