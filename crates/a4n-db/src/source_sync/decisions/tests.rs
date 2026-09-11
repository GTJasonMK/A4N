use super::guards::{
    ensure_clean_source_sync_target, ensure_source_sync_decision_targets_generation_spec,
    source_sync_generation_spec_id_for_conflict,
};
use super::{
    PreparedSourceSyncDecisionRecordInput, SourceSyncRefreshSpecRecordInput,
    apply_prepared_source_sync_decisions_records_transaction,
};
use crate::source_sync::{
    SourceSyncAnchorMappingRecord, SourceSyncDbError, SourceSyncGalgameSceneWriteRecordInput,
    SourceSyncReplacementSegmentRecordInput, SourceSyncReplacementSpecRecordInput,
};
use a4n_db_models::models::source_sync::source_sync_conflict;
use a4n_domain::source_sync::{SourceSyncConflictStatus, SourceSyncResolutionStrategy};
use chrono::Utc;
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};

fn conflict_record(
    conflict_scope: &str,
    target_kind: &str,
    target_id: i64,
    local_state_kind: &str,
    local_summary_json: Value,
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
        local_state_kind: local_state_kind.to_string(),
        severity: "blocking".to_string(),
        status: "pending".to_string(),
        resolution_strategy: None,
        source_summary_json: json!({}),
        local_summary_json,
        affected_refs_json: json!({}),
        decision_json: None,
        created_at: Utc::now().naive_utc(),
        resolved_at: None,
    }
}

fn refresh_input(generation_spec_id: i64) -> SourceSyncRefreshSpecRecordInput {
    SourceSyncRefreshSpecRecordInput {
        generation_spec_id,
        prompt_kind: "video_prompt".to_string(),
        params_json: json!({}),
        text_anchor_refs: Vec::new(),
        entity_refs: Vec::new(),
        media_refs: Vec::new(),
    }
}

fn replacement_input(generation_spec_id: i64) -> SourceSyncReplacementSpecRecordInput {
    SourceSyncReplacementSpecRecordInput {
        generation_spec_id,
        prompt_kind: "video_prompt".to_string(),
        params_json: json!({}),
        text_anchor_refs: Vec::new(),
        entity_refs: Vec::new(),
        media_refs: Vec::new(),
        replacement_segment: SourceSyncReplacementSegmentRecordInput {
            title: "replacement".to_string(),
            duration_secs: 1.0,
            order_index: 1,
            previous_segment_id: None,
            next_segment_id: None,
        },
    }
}

#[test]
fn prepared_decision_variant_metadata_is_stable() {
    let decisions = vec![
        PreparedSourceSyncDecisionRecordInput::KeepLocal {
            conflict_id: 1,
            decision_payload_json: json!({"id": 1}),
        },
        PreparedSourceSyncDecisionRecordInput::Detach {
            conflict_id: 2,
            decision_payload_json: json!({"id": 2}),
        },
        PreparedSourceSyncDecisionRecordInput::RebindAnchor {
            conflict_id: 3,
            generation_spec_id: 30,
            anchor_mappings: vec![SourceSyncAnchorMappingRecord {
                old_text_anchor_id: 1,
                new_text_anchor_id: 2,
            }],
            decision_payload_json: json!({"id": 3}),
        },
        PreparedSourceSyncDecisionRecordInput::Manual {
            conflict_id: 4,
            prompt_artifact_id: 40,
            prompt_text: "manual".to_string(),
            decision_payload_json: json!({"id": 4}),
        },
        PreparedSourceSyncDecisionRecordInput::DeleteLocal {
            conflict_id: 5,
            decision_payload_json: json!({"id": 5}),
        },
        PreparedSourceSyncDecisionRecordInput::AdoptSource {
            conflict_id: 6,
            input: refresh_input(60),
            decision_payload_json: json!({"id": 6}),
        },
        PreparedSourceSyncDecisionRecordInput::RebuildPrompt {
            conflict_id: 7,
            input: refresh_input(70),
            decision_payload_json: json!({"id": 7}),
        },
        PreparedSourceSyncDecisionRecordInput::CreateReplacement {
            conflict_id: 8,
            input: replacement_input(80),
            decision_payload_json: json!({"id": 8}),
        },
        PreparedSourceSyncDecisionRecordInput::RegenerateScene {
            conflict_id: 9,
            galgame_scene_id: 90,
            reason: None,
            expected_scene_index: 1,
            expected_source_snapshot_chapter_id: 100,
            generated_scene: SourceSyncGalgameSceneWriteRecordInput {
                title: "scene".to_string(),
                location: None,
                time_of_day: None,
                participants_json: json!([]),
                background_note: None,
                lines: Vec::new(),
            },
            stage_outputs_json: json!({}),
            decision_payload_json: json!({"id": 9}),
        },
    ];
    let expected = [
        (
            SourceSyncResolutionStrategy::KeepLocal,
            SourceSyncConflictStatus::Ignored,
        ),
        (
            SourceSyncResolutionStrategy::Detach,
            SourceSyncConflictStatus::Ignored,
        ),
        (
            SourceSyncResolutionStrategy::RebindAnchor,
            SourceSyncConflictStatus::Resolved,
        ),
        (
            SourceSyncResolutionStrategy::Manual,
            SourceSyncConflictStatus::Resolved,
        ),
        (
            SourceSyncResolutionStrategy::DeleteLocal,
            SourceSyncConflictStatus::Resolved,
        ),
        (
            SourceSyncResolutionStrategy::AdoptSource,
            SourceSyncConflictStatus::Resolved,
        ),
        (
            SourceSyncResolutionStrategy::RebuildPrompt,
            SourceSyncConflictStatus::Resolved,
        ),
        (
            SourceSyncResolutionStrategy::CreateReplacement,
            SourceSyncConflictStatus::Resolved,
        ),
        (
            SourceSyncResolutionStrategy::RegenerateScene,
            SourceSyncConflictStatus::Resolved,
        ),
    ];

    for (index, decision) in decisions.iter().enumerate() {
        let conflict_id = i64::try_from(index + 1).expect("small fixture index");
        assert_eq!(decision.conflict_id(), conflict_id);
        assert_eq!(decision.strategy(), expected[index].0);
        assert_eq!(decision.conflict_status(), expected[index].1);
        assert_eq!(
            decision.decision_payload_json(),
            &json!({"id": conflict_id})
        );
    }
}

#[test]
fn clean_target_and_generation_spec_guards_preserve_authoritative_id_rules() {
    let clean = conflict_record("generation_spec", "segment", 42, "clean", json!({}));
    ensure_clean_source_sync_target(&clean, "adopt_source").expect("clean target should pass");
    ensure_source_sync_decision_targets_generation_spec(&clean, 42, "adopt_source")
        .expect("matching generation spec should pass");
    assert_eq!(
        source_sync_generation_spec_id_for_conflict(&clean, "adopt_source")
            .expect("generation_spec scope owns target id"),
        42
    );

    let anchor = conflict_record("text_anchor", "segment", 43, "draft", json!({}));
    ensure_clean_source_sync_target(&anchor, "rebind_anchor").expect("draft target should pass");
    assert_eq!(
        source_sync_generation_spec_id_for_conflict(&anchor, "rebind_anchor")
            .expect("segment anchor conflict owns target id"),
        43
    );

    let summary_bound = conflict_record(
        "text_anchor",
        "galgame_line",
        44,
        "clean",
        json!({"generation_spec_id": 45}),
    );
    assert_eq!(
        source_sync_generation_spec_id_for_conflict(&summary_bound, "manual")
            .expect("local summary supplies generation spec id"),
        45
    );
}

#[test]
fn guards_reject_dirty_missing_and_mismatched_targets() {
    let dirty = conflict_record("generation_spec", "segment", 42, "generated", json!({}));
    assert!(matches!(
        ensure_clean_source_sync_target(&dirty, "adopt_source"),
        Err(SourceSyncDbError::InvalidArgument(message)) if message.contains("本地状态为 generated")
    ));
    assert!(matches!(
        ensure_source_sync_decision_targets_generation_spec(&dirty, 99, "delete_local"),
        Err(SourceSyncDbError::InvalidArgument(message)) if message.contains("目标生成意图不匹配")
    ));

    let missing = conflict_record("text_anchor", "galgame_line", 44, "clean", json!({}));
    assert!(matches!(
        source_sync_generation_spec_id_for_conflict(&missing, "manual"),
        Err(SourceSyncDbError::InvalidArgument(message)) if message.contains("未提供 generation_spec_id")
    ));
}

#[tokio::test]
async fn empty_prepared_batch_is_rejected_before_transaction_begin() {
    let error = apply_prepared_source_sync_decisions_records_transaction(
        &DatabaseConnection::Disconnected,
        1,
        Vec::new(),
    )
    .await
    .expect_err("empty batch must fail before transaction begin");

    assert!(matches!(
        error,
        SourceSyncDbError::InvalidArgument(message)
            if message == "source sync prepared decisions 不能为空"
    ));
}
