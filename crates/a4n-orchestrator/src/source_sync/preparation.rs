use super::contracts::{
    ManualPromptPayload, RebindAnchorPayload, RefreshSpecPayload, RegenerateGalgameScenePayload,
    ReplacementSpecPayload, SourceSyncDecisionInput,
};
use super::decisions::{
    PreparedSourceSyncDecisionContext, parse_decision_payload,
    prepared_galgame_scene_regeneration_decision,
};
use super::rules::normalize_strategy;
use a4n_core::error::AppError;

pub(super) fn prepared_source_sync_decisions(
    decisions: &[SourceSyncDecisionInput],
    prepared_context: &PreparedSourceSyncDecisionContext,
) -> Result<Vec<a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput>, AppError> {
    let mut prepared = Vec::with_capacity(decisions.len());
    for decision in decisions {
        let prepared_decision = match normalize_strategy(&decision.strategy)? {
            "keep_local" => a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::KeepLocal {
                conflict_id: decision.conflict_id,
                decision_payload_json: decision.decision_payload_json.clone(),
            },
            "detach" => a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::Detach {
                conflict_id: decision.conflict_id,
                decision_payload_json: decision.decision_payload_json.clone(),
            },
            "rebind_anchor" => {
                let payload: RebindAnchorPayload =
                    parse_decision_payload(&decision.decision_payload_json)?;
                a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::RebindAnchor {
                    conflict_id: decision.conflict_id,
                    generation_spec_id: payload.generation_spec_id,
                    anchor_mappings: payload
                        .anchor_mappings
                        .into_iter()
                        .map(
                            |mapping| a4n_db::source_sync::SourceSyncAnchorMappingRecord {
                                old_text_anchor_id: mapping.old_text_anchor_id,
                                new_text_anchor_id: mapping.new_text_anchor_id,
                            },
                        )
                        .collect(),
                    decision_payload_json: decision.decision_payload_json.clone(),
                }
            }
            "manual" => {
                let payload: ManualPromptPayload =
                    parse_decision_payload(&decision.decision_payload_json)?;
                a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::Manual {
                    conflict_id: decision.conflict_id,
                    prompt_artifact_id: payload.prompt_artifact_id,
                    prompt_text: payload.prompt_text,
                    decision_payload_json: decision.decision_payload_json.clone(),
                }
            }
            "delete_local" => {
                a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::DeleteLocal {
                    conflict_id: decision.conflict_id,
                    decision_payload_json: decision.decision_payload_json.clone(),
                }
            }
            "adopt_source" => {
                let payload: RefreshSpecPayload =
                    parse_decision_payload(&decision.decision_payload_json)?;
                a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::AdoptSource {
                    conflict_id: decision.conflict_id,
                    input: source_sync_refresh_spec_record_input(payload),
                    decision_payload_json: decision.decision_payload_json.clone(),
                }
            }
            "rebuild_prompt" => {
                let payload: RefreshSpecPayload =
                    parse_decision_payload(&decision.decision_payload_json)?;
                a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::RebuildPrompt {
                    conflict_id: decision.conflict_id,
                    input: source_sync_refresh_spec_record_input(payload),
                    decision_payload_json: decision.decision_payload_json.clone(),
                }
            }
            "create_replacement" => {
                let payload: ReplacementSpecPayload =
                    parse_decision_payload(&decision.decision_payload_json)?;
                a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::CreateReplacement {
                    conflict_id: decision.conflict_id,
                    input: a4n_db::source_sync::SourceSyncReplacementSpecRecordInput {
                        generation_spec_id: payload.generation_spec_id,
                        prompt_kind: payload.prompt_kind,
                        params_json: payload.params_json,
                        text_anchor_refs: payload.text_anchor_refs,
                        entity_refs: payload.entity_refs,
                        media_refs: payload.media_refs,
                        replacement_segment:
                            a4n_db::source_sync::SourceSyncReplacementSegmentRecordInput {
                                title: payload.replacement_segment.title,
                                duration_secs: payload.replacement_segment.duration_secs,
                                order_index: payload.replacement_segment.order_index,
                                previous_segment_id: payload
                                    .replacement_segment
                                    .previous_segment_id,
                                next_segment_id: payload.replacement_segment.next_segment_id,
                            },
                    },
                    decision_payload_json: decision.decision_payload_json.clone(),
                }
            }
            "regenerate_scene" => {
                let payload: RegenerateGalgameScenePayload =
                    parse_decision_payload(&decision.decision_payload_json)?;
                prepared_galgame_scene_regeneration_decision(
                    prepared_context,
                    decision.conflict_id,
                    payload,
                    decision.decision_payload_json.clone(),
                )?
            }
            other => {
                return Err(AppError::InvalidArgument(format!(
                    "同步冲突解决策略规范化后仍不受支持: {other}"
                )));
            }
        };
        prepared.push(prepared_decision);
    }
    Ok(prepared)
}

fn source_sync_refresh_spec_record_input(
    payload: RefreshSpecPayload,
) -> a4n_db::source_sync::SourceSyncRefreshSpecRecordInput {
    a4n_db::source_sync::SourceSyncRefreshSpecRecordInput {
        generation_spec_id: payload.generation_spec_id,
        prompt_kind: payload.prompt_kind,
        params_json: payload.params_json,
        text_anchor_refs: payload.text_anchor_refs,
        entity_refs: payload.entity_refs,
        media_refs: payload.media_refs,
    }
}
