use super::conflicts::is_same_galgame_scene_conflict_record;
use crate::source_sync::CONFLICT_STATUS_PENDING;
use a4n_core::models::source_sync::source_sync_conflict;
use chrono::Utc;
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn conflict_record(
    conflict_scope: &str,
    target_kind: &str,
    target_id: i64,
    affected_refs_json: Value,
) -> source_sync_conflict::Model {
    source_sync_conflict::Model {
        id: 1,
        sync_run_id: 10,
        derived_project_id: 20,
        root_project_id: 30,
        from_snapshot_id: 40,
        to_snapshot_id: 50,
        conflict_scope: conflict_scope.to_string(),
        target_kind: target_kind.to_string(),
        target_id,
        source_change_kind: "updated".to_string(),
        local_state_kind: "generated".to_string(),
        severity: "blocking".to_string(),
        status: CONFLICT_STATUS_PENDING.to_string(),
        resolution_strategy: None,
        source_summary_json: json!({}),
        local_summary_json: json!({}),
        affected_refs_json,
        decision_json: None,
        created_at: Utc::now().naive_utc(),
        resolved_at: None,
    }
}

#[test]
fn galgame_scene_conflict_match_accepts_scene_target() {
    let conflict = conflict_record("text_anchor", "galgame_scene", 7, json!({}));

    assert!(is_same_galgame_scene_conflict_record(
        &conflict,
        7,
        &BTreeSet::new()
    ));
}

#[test]
fn galgame_scene_conflict_match_accepts_line_refs_and_old_line_ids() {
    let by_ref = conflict_record(
        "text_anchor",
        "galgame_line",
        11,
        json!({"galgame_scene_id": 7}),
    );
    let by_old_line = conflict_record("text_anchor", "galgame_line", 12, json!({}));
    let old_line_ids = BTreeSet::from([12]);

    assert!(is_same_galgame_scene_conflict_record(
        &by_ref,
        7,
        &old_line_ids
    ));
    assert!(is_same_galgame_scene_conflict_record(
        &by_old_line,
        7,
        &old_line_ids
    ));
}

#[test]
fn galgame_scene_conflict_match_rejects_other_scope_or_target() {
    let wrong_scope = conflict_record("profile", "galgame_scene", 7, json!({}));
    let wrong_scene = conflict_record("text_anchor", "galgame_scene", 8, json!({}));
    let wrong_kind = conflict_record("text_anchor", "generation_spec", 7, json!({}));

    assert!(!is_same_galgame_scene_conflict_record(
        &wrong_scope,
        7,
        &BTreeSet::new()
    ));
    assert!(!is_same_galgame_scene_conflict_record(
        &wrong_scene,
        7,
        &BTreeSet::new()
    ));
    assert!(!is_same_galgame_scene_conflict_record(
        &wrong_kind,
        7,
        &BTreeSet::new()
    ));
}
