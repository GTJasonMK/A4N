use super::contracts::{
    ManualPromptPayload, RebindAnchorPayload, RefreshSpecPayload, RegenerateGalgameScenePayload,
    ReplacementSpecPayload, SourceSyncDecisionInput,
};
use super::decisions::{
    PreparedSourceSyncDecisionContext, parse_decision_payload,
    prepared_galgame_scene_regeneration_decision,
};
use a4n_domain::source_sync::SourceSyncResolutionStrategy;
use a4n_error::AppError;
use serde_json::Value;

pub(super) fn prepared_source_sync_decisions(
    decisions: &[SourceSyncDecisionInput],
    prepared_context: &PreparedSourceSyncDecisionContext,
) -> Result<Vec<a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput>, AppError> {
    let mut prepared = Vec::with_capacity(decisions.len());
    for decision in decisions {
        let decision_payload_json = Value::Object(decision.decision_payload_json.clone());
        let prepared_decision = match decision.strategy {
            SourceSyncResolutionStrategy::KeepLocal => {
                a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::KeepLocal {
                    conflict_id: decision.conflict_id,
                    decision_payload_json,
                }
            }
            SourceSyncResolutionStrategy::Detach => {
                a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::Detach {
                    conflict_id: decision.conflict_id,
                    decision_payload_json,
                }
            }
            SourceSyncResolutionStrategy::RebindAnchor => {
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
                    decision_payload_json,
                }
            }
            SourceSyncResolutionStrategy::Manual => {
                let payload: ManualPromptPayload =
                    parse_decision_payload(&decision.decision_payload_json)?;
                a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::Manual {
                    conflict_id: decision.conflict_id,
                    prompt_artifact_id: payload.prompt_artifact_id,
                    prompt_text: payload.prompt_text,
                    decision_payload_json,
                }
            }
            SourceSyncResolutionStrategy::DeleteLocal => {
                a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::DeleteLocal {
                    conflict_id: decision.conflict_id,
                    decision_payload_json,
                }
            }
            SourceSyncResolutionStrategy::AdoptSource => {
                let payload: RefreshSpecPayload =
                    parse_decision_payload(&decision.decision_payload_json)?;
                a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::AdoptSource {
                    conflict_id: decision.conflict_id,
                    input: source_sync_refresh_spec_record_input(payload),
                    decision_payload_json,
                }
            }
            SourceSyncResolutionStrategy::RebuildPrompt => {
                let payload: RefreshSpecPayload =
                    parse_decision_payload(&decision.decision_payload_json)?;
                a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::RebuildPrompt {
                    conflict_id: decision.conflict_id,
                    input: source_sync_refresh_spec_record_input(payload),
                    decision_payload_json,
                }
            }
            SourceSyncResolutionStrategy::CreateReplacement => {
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
                    decision_payload_json,
                }
            }
            SourceSyncResolutionStrategy::RegenerateScene => {
                let payload: RegenerateGalgameScenePayload =
                    parse_decision_payload(&decision.decision_payload_json)?;
                prepared_galgame_scene_regeneration_decision(
                    prepared_context,
                    decision.conflict_id,
                    payload,
                    decision_payload_json,
                )?
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
