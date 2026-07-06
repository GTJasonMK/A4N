use super::decisions::{
    PreparedSourceSyncDecisionContext, ensure_decision_targets_generation_spec,
    galgame_portrait_spec_references_lines, generation_spec_id_for_conflict,
    parse_decision_payload, require_galgame_scene_regeneration_plan,
    validate_galgame_scene_regeneration_plan_for_apply,
};
use super::diff::{
    AnchorChange, anchor_change_conflict, classify_anchor_change, severity_for_impacted_local_state,
};
use super::*;
use crate::galgame::{
    GalgamePersistLine, GalgamePersistScene, GalgameSceneRegenerationPlan, LINE_TYPE_CHOICE,
    LINE_TYPE_DIALOGUE, SCENE_ENTRY_MAIN,
};
use serde_json::json;

fn anchor(
    id: i64,
    snapshot_id: i64,
    source_chapter_id: i64,
    paragraph_index: i32,
    start_offset: i32,
    end_offset: i32,
    quote_hash: &str,
) -> text_anchor::Model {
    text_anchor::Model {
        id,
        project_id: 1,
        source_chapter_id: Some(source_chapter_id),
        source_snapshot_id: Some(snapshot_id),
        source_snapshot_chapter_id: Some(snapshot_id * 10 + source_chapter_id),
        source_kind: "chapter_content".to_string(),
        source_id: snapshot_id * 10 + source_chapter_id,
        paragraph_index,
        start_offset,
        end_offset,
        quote: "他推开门。".to_string(),
        quote_hash: quote_hash.to_string(),
        source_version_hash: "source-hash".to_string(),
        created_at: Utc::now().naive_utc(),
    }
}

fn conflict_with_target(
    conflict_scope: &str,
    target_kind: &str,
    target_id: i64,
    local_summary_json: serde_json::Value,
) -> source_sync_conflict::Model {
    source_sync_conflict::Model {
        id: 1,
        sync_run_id: 10,
        derived_project_id: 2,
        root_project_id: 1,
        from_snapshot_id: 100,
        to_snapshot_id: 101,
        conflict_scope: conflict_scope.to_string(),
        target_kind: target_kind.to_string(),
        target_id,
        source_change_kind: "updated".to_string(),
        local_state_kind: "clean".to_string(),
        severity: "warning".to_string(),
        status: CONFLICT_STATUS_PENDING.to_string(),
        resolution_strategy: None,
        source_summary_json: json!({}),
        local_summary_json,
        affected_refs_json: json!({}),
        decision_json: None,
        created_at: Utc::now().naive_utc(),
        resolved_at: None,
    }
}

#[test]
fn classify_anchor_change_detects_unchanged_and_moved() {
    let old = anchor(1, 10, 5, 0, 0, 5, "same");
    let unchanged = anchor(2, 11, 5, 0, 0, 5, "same");
    let moved = anchor(3, 11, 5, 0, 8, 13, "same");

    assert_eq!(
        classify_anchor_change(&old, &[unchanged]),
        AnchorChange::Unchanged
    );
    assert_eq!(
        classify_anchor_change(&old, &[moved]),
        AnchorChange::Moved(3)
    );
}

#[test]
fn classify_anchor_change_treats_missing_and_ambiguous_as_blocking() {
    let old = anchor(1, 10, 5, 0, 0, 5, "same");
    let first = anchor(2, 11, 5, 0, 8, 13, "same");
    let second = anchor(3, 11, 5, 1, 20, 25, "same");

    assert_eq!(classify_anchor_change(&old, &[]), AnchorChange::Missing);
    assert_eq!(
        classify_anchor_change(&old, &[first, second]),
        AnchorChange::Ambiguous
    );
    assert_eq!(
        anchor_change_conflict(&AnchorChange::Missing, "clean"),
        Some(("deleted", None, "blocking"))
    );
    assert_eq!(
        anchor_change_conflict(&AnchorChange::Ambiguous, "draft"),
        Some(("split", None, "blocking"))
    );
}

#[test]
fn severity_for_impacted_local_state_is_conservative() {
    assert_eq!(
        severity_for_impacted_local_state("clean", "updated"),
        "warning"
    );
    assert_eq!(
        severity_for_impacted_local_state("generated", "updated"),
        "blocking"
    );
    assert_eq!(
        severity_for_impacted_local_state("clean", "deleted"),
        "blocking"
    );
}

#[test]
fn source_sync_decision_input_rejects_unknown_fields() {
    let err = serde_json::from_value::<SourceSyncDecisionInput>(json!({
        "conflict_id": 1,
        "strategy": "manual",
        "decision_payload_json": {},
        "extra": true
    }))
    .expect_err("source sync decision input must reject unknown fields");

    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn source_sync_decision_payload_rejects_unknown_nested_fields() {
    let payload = json!({
        "generation_spec_id": 1,
        "prompt_kind": "chapter_video_segment",
        "params_json": {},
        "text_anchor_refs": [
            {
                "text_anchor_id": 10,
                "role": "source",
                "extra": true
            }
        ],
        "entity_refs": [],
        "media_refs": []
    });

    let err = parse_decision_payload::<RefreshSpecPayload>(&payload)
        .expect_err("source sync payload must reject unknown nested fields");

    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn regenerate_scene_payload_rejects_unknown_fields() {
    let payload = json!({
        "galgame_scene_id": 1,
        "reason": "source_changed",
        "silent_fallback": true
    });

    let err = parse_decision_payload::<RegenerateGalgameScenePayload>(&payload)
        .expect_err("regenerate_scene payload must reject unknown fields");

    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn regenerate_scene_requires_prepared_llm_plan() {
    let prepared_context = PreparedSourceSyncDecisionContext::default();

    let err = require_galgame_scene_regeneration_plan(&prepared_context, 42)
        .expect_err("regenerate_scene must not fall back to baseline anchor regeneration");

    assert!(
        err.to_string()
            .contains("missing_galgame_scene_regeneration_plan_for_conflict_42")
    );
    assert!(err.to_string().contains("regenerate_scene"));
}

#[test]
fn regenerate_scene_apply_plan_accepts_complete_single_scene_contract() {
    let plan = valid_regenerate_scene_apply_plan();

    validate_galgame_scene_regeneration_plan_for_apply(
        &plan,
        7,
        1010,
        &std::collections::BTreeSet::from([9001]),
    )
    .expect("valid regenerate_scene apply plan");
}

#[test]
fn regenerate_scene_apply_plan_rejects_choice_line_even_without_options() {
    let mut plan = valid_regenerate_scene_apply_plan();
    plan.scene.lines[0].line_type = LINE_TYPE_CHOICE.to_string();

    let err = validate_galgame_scene_regeneration_plan_for_apply(
        &plan,
        7,
        1010,
        &std::collections::BTreeSet::from([9001]),
    )
    .expect_err("source sync regenerate_scene must not write choice rows");

    assert!(err.to_string().contains("不允许写入选择支"));
}

#[test]
fn regenerate_scene_apply_plan_rejects_cross_chapter_anchor() {
    let plan = valid_regenerate_scene_apply_plan();

    let err = validate_galgame_scene_regeneration_plan_for_apply(
        &plan,
        7,
        1010,
        &std::collections::BTreeSet::from([9002]),
    )
    .expect_err("line text_anchor_id must belong to the target snapshot chapter");

    assert!(
        err.to_string()
            .contains("text_anchor_id 不属于目标源快照章节")
    );
}

#[test]
fn regenerate_scene_apply_plan_rejects_incomplete_stage_outputs() {
    let mut plan = valid_regenerate_scene_apply_plan();
    plan.stage_outputs_json = json!({
        "scene_segmentation": {
            "scenes": [{
                "index": 1,
                "title": "雨夜",
                "source_span_ids": ["s1"]
            }]
        },
        "script_adaptation": {
            "scene_lines": [{
                "scene_index": 1,
                "lines": [{
                    "index": 1,
                    "line_type": "dialogue",
                    "speaker_name": "林晓雨",
                    "text": "你终于来了。",
                    "source_span_ids": ["s1"]
                }]
            }]
        }
    });

    let err = validate_galgame_scene_regeneration_plan_for_apply(
        &plan,
        7,
        1010,
        &std::collections::BTreeSet::from([9001]),
    )
    .expect_err("apply plan must preserve full stage_outputs");

    assert!(err.to_string().contains("stage_outputs 缺少"));
}

#[test]
fn regenerate_scene_detects_portrait_specs_referencing_deleted_lines() {
    let deleted_line_ids = std::collections::BTreeSet::from([10, 11]);

    assert!(galgame_portrait_spec_references_lines(
        &json!({
            "asset_kind": "portrait_expression",
            "source_line_ids": [9, 10]
        }),
        &deleted_line_ids
    ));
    assert!(!galgame_portrait_spec_references_lines(
        &json!({
            "asset_kind": "portrait_expression",
            "source_line_ids": [12, 13]
        }),
        &deleted_line_ids
    ));
}

#[test]
fn regenerate_scene_ignores_malformed_portrait_source_line_ids_for_cleanup_match() {
    let deleted_line_ids = std::collections::BTreeSet::from([10]);

    assert!(!galgame_portrait_spec_references_lines(
        &json!({
            "asset_kind": "portrait_expression",
            "source_line_ids": ["10"]
        }),
        &deleted_line_ids
    ));
    assert!(!galgame_portrait_spec_references_lines(
        &json!({
            "asset_kind": "portrait_expression"
        }),
        &deleted_line_ids
    ));
}

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

fn valid_regenerate_scene_apply_plan() -> GalgameSceneRegenerationPlan {
    GalgameSceneRegenerationPlan {
        scene: GalgamePersistScene {
            scene_index: 7,
            source_snapshot_chapter_id: 1010,
            title: "雨夜重逢".to_string(),
            location: Some("旧车站".to_string()),
            time_of_day: Some("night".to_string()),
            participants_json: json!([{"profile_id": 7, "name": "林晓雨"}]),
            background_note: Some("旧车站的雨夜月台。".to_string()),
            entry_kind: SCENE_ENTRY_MAIN.to_string(),
            next_scene_index: None,
            lines: vec![GalgamePersistLine {
                line_index: 1,
                line_type: LINE_TYPE_DIALOGUE.to_string(),
                speaker_profile_id: Some(7),
                text: "你终于来了。".to_string(),
                expression: Some("smile".to_string()),
                pose: Some("standing".to_string()),
                position: Some("left".to_string()),
                presentation_json: json!({}),
                text_anchor_id: Some(9001),
                choices: Vec::new(),
            }],
        },
        stage_outputs_json: json!({
            "scene_segmentation": {
                "scenes": [{
                    "index": 1,
                    "title": "雨夜重逢",
                    "location": "旧车站",
                    "time_of_day": "night",
                    "participants": ["林晓雨"],
                    "summary": "旧车站的雨夜月台。",
                    "source_span_ids": ["s1"]
                }]
            },
            "script_adaptation": {
                "scene_lines": [{
                    "scene_index": 1,
                    "lines": [{
                        "index": 1,
                        "line_type": "dialogue",
                        "speaker_name": "林晓雨",
                        "text": "你终于来了。",
                        "source_span_ids": ["s1"]
                    }]
                }]
            },
            "character_staging": {
                "scene_lines": [{
                    "scene_index": 1,
                    "lines": [{
                        "index": 1,
                        "line_type": "dialogue",
                        "speaker_profile_id": 7,
                        "speaker_name": "林晓雨",
                        "text": "你终于来了。",
                        "expression": "smile",
                        "pose": "standing",
                        "position": "left",
                        "source_span_ids": ["s1"]
                    }]
                }]
            },
            "visual_planning": {
                "backgrounds": [{
                    "scene_index": 1,
                    "background_note": "旧车站的雨夜月台。",
                    "source_span_ids": ["s1"]
                }],
                "event_cgs": []
            },
            "branch_design": {
                "choices": []
            }
        }),
    }
}
