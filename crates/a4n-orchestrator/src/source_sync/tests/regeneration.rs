use serde_json::json;

use a4n_db::source_sync::galgame_portrait_spec_references_lines;

use super::super::decisions::{
    PreparedSourceSyncDecisionContext, parse_decision_payload,
    prepared_galgame_scene_regeneration_decision, require_galgame_scene_regeneration_plan,
    validate_galgame_scene_regeneration_plan_for_apply,
};
use super::super::{
    RegenerateGalgameScenePayload, SourceSyncDecisionInput, prepared_source_sync_decisions,
    source_sync_decisions_require_galgame_preparation,
};
use super::support::valid_regenerate_scene_apply_plan;
use crate::galgame::LINE_TYPE_CHOICE;

#[test]
fn regenerate_scene_requires_prepared_context_before_db_plan_conversion() {
    let decisions = vec![
        SourceSyncDecisionInput {
            conflict_id: 11,
            strategy: "keep_local".to_string(),
            decision_payload_json: json!({}),
        },
        SourceSyncDecisionInput {
            conflict_id: 12,
            strategy: "regenerate_scene".to_string(),
            decision_payload_json: json!({"galgame_scene_id": 99, "reason": null}),
        },
    ];

    assert!(
        source_sync_decisions_require_galgame_preparation(&decisions)
            .expect("regenerate strategy should be valid")
    );
    let err =
        prepared_source_sync_decisions(&decisions, &PreparedSourceSyncDecisionContext::default())
            .expect_err("regenerate decision must not reach DB without prepared LLM plan");
    assert!(
        err.to_string()
            .contains("missing_galgame_scene_regeneration_plan")
    );
}
#[test]
fn regenerate_scene_prepared_decision_preserves_validated_plan() {
    let mut context = PreparedSourceSyncDecisionContext::default();
    context
        .galgame_scene_regeneration_plans
        .insert(12, valid_regenerate_scene_apply_plan());

    let decision = prepared_galgame_scene_regeneration_decision(
        &context,
        12,
        RegenerateGalgameScenePayload {
            galgame_scene_id: 88,
            reason: Some("source_changed".to_string()),
        },
        json!({"galgame_scene_id": 88, "reason": "source_changed"}),
    )
    .expect("validated plan should convert to DB prepared decision");

    assert!(matches!(
        decision,
        a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::RegenerateScene {
            conflict_id: 12,
            galgame_scene_id: 88,
            reason: Some(reason),
            expected_scene_index: 7,
            expected_source_snapshot_chapter_id: 1010,
            generated_scene,
            stage_outputs_json,
            ..
        } if reason == "source_changed"
            && generated_scene.title == "雨夜重逢"
            && generated_scene.lines.len() == 1
            && generated_scene.lines[0].text_anchor_id == Some(9001)
            && stage_outputs_json.get("shooting_script_projection").is_some()
    ));
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
        "shooting_script_projection": {
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
