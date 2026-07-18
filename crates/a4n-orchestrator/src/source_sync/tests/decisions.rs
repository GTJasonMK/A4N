use serde_json::json;

use super::super::decisions::{PreparedSourceSyncDecisionContext, parse_decision_payload};
use super::super::{RefreshSpecPayload, SourceSyncDecisionInput, prepared_source_sync_decisions};

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
fn prepared_source_sync_decisions_prepares_supported_mixed_batch() {
    let decisions = vec![
        SourceSyncDecisionInput {
            conflict_id: 11,
            strategy: " keep_local ".to_string(),
            decision_payload_json: json!({"reason": "preserve_local"}),
        },
        SourceSyncDecisionInput {
            conflict_id: 12,
            strategy: "detach".to_string(),
            decision_payload_json: json!({"reason": "independent_project"}),
        },
        SourceSyncDecisionInput {
            conflict_id: 13,
            strategy: "rebind_anchor".to_string(),
            decision_payload_json: json!({
                "generation_spec_id": 30,
                "anchor_mappings": [{
                    "old_text_anchor_id": 40,
                    "new_text_anchor_id": 41
                }]
            }),
        },
        SourceSyncDecisionInput {
            conflict_id: 14,
            strategy: "manual".to_string(),
            decision_payload_json: json!({
                "prompt_artifact_id": 50,
                "prompt_text": "manual prompt"
            }),
        },
        SourceSyncDecisionInput {
            conflict_id: 15,
            strategy: "delete_local".to_string(),
            decision_payload_json: json!({"reason": "discard_draft"}),
        },
        SourceSyncDecisionInput {
            conflict_id: 16,
            strategy: "adopt_source".to_string(),
            decision_payload_json: json!({
                "generation_spec_id": 60,
                "prompt_kind": "chapter_video_segment",
                "params_json": {"style": "cinematic"},
                "text_anchor_refs": [{"text_anchor_id": 61, "role": "source"}],
                "entity_refs": [],
                "media_refs": []
            }),
        },
        SourceSyncDecisionInput {
            conflict_id: 17,
            strategy: "rebuild_prompt".to_string(),
            decision_payload_json: json!({
                "generation_spec_id": 70,
                "prompt_kind": "chapter_video_segment",
                "params_json": {},
                "text_anchor_refs": [{"text_anchor_id": 71, "role": "source"}],
                "entity_refs": [],
                "media_refs": []
            }),
        },
        SourceSyncDecisionInput {
            conflict_id: 18,
            strategy: "create_replacement".to_string(),
            decision_payload_json: json!({
                "generation_spec_id": 80,
                "prompt_kind": "chapter_video_segment",
                "params_json": {},
                "text_anchor_refs": [{"text_anchor_id": 81, "role": "source"}],
                "entity_refs": [],
                "media_refs": [],
                "replacement_segment": {
                    "title": "replacement",
                    "duration_secs": 3.5,
                    "order_index": 2,
                    "previous_segment_id": null,
                    "next_segment_id": null
                }
            }),
        },
    ];

    let prepared =
        prepared_source_sync_decisions(&decisions, &PreparedSourceSyncDecisionContext::default())
            .expect("prepared decisions should be valid");

    assert!(matches!(
        &prepared[0],
        a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::KeepLocal {
            conflict_id: 11,
            decision_payload_json,
        } if decision_payload_json == &json!({"reason": "preserve_local"})
    ));
    assert!(matches!(
        &prepared[1],
        a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::Detach {
            conflict_id: 12,
            decision_payload_json,
        } if decision_payload_json == &json!({"reason": "independent_project"})
    ));
    assert!(matches!(
        &prepared[2],
        a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::RebindAnchor {
            conflict_id: 13,
            generation_spec_id: 30,
            anchor_mappings,
            ..
        } if anchor_mappings.len() == 1
            && anchor_mappings[0].old_text_anchor_id == 40
            && anchor_mappings[0].new_text_anchor_id == 41
    ));
    assert!(matches!(
        &prepared[3],
        a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::Manual {
            conflict_id: 14,
            prompt_artifact_id: 50,
            prompt_text,
            ..
        } if prompt_text == "manual prompt"
    ));
    assert!(matches!(
        &prepared[4],
        a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::DeleteLocal {
            conflict_id: 15,
            ..
        }
    ));
    assert!(matches!(
        &prepared[5],
        a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::AdoptSource {
            conflict_id: 16,
            input,
            ..
        } if input.generation_spec_id == 60
            && input.prompt_kind == "chapter_video_segment"
            && input.text_anchor_refs.len() == 1
    ));
    assert!(matches!(
        &prepared[6],
        a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::RebuildPrompt {
            conflict_id: 17,
            input,
            ..
        } if input.generation_spec_id == 70
            && input.text_anchor_refs[0].text_anchor_id == 71
    ));
    assert!(matches!(
        &prepared[7],
        a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::CreateReplacement {
            conflict_id: 18,
            input,
            ..
        } if input.generation_spec_id == 80
            && input.replacement_segment.title == "replacement"
            && input.replacement_segment.duration_secs == 3.5
    ));
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
