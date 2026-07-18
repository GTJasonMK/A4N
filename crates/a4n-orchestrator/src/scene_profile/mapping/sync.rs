use a4n_core::error::AppError;
use a4n_core::profile_text::required_text;
use a4n_db::scene_profile::{
    SceneEntityStateRecordInput, SceneOccurrenceRecordInput, SceneProfileRecordDraft,
    SceneSyncRecordInput, SceneTransitionRecordInput,
};
use serde_json::json;

use super::super::contracts::{
    SceneEntityStateDraft, SceneOccurrenceDraft, SceneProfileDraft, SceneSyncInput,
    SceneTransitionDraft,
};
use super::super::rules::{
    normalize_movement_type_value, normalize_scene_entity_state_key_value,
    normalize_scene_entity_type_value, trimmed_optional, validate_positive_i32,
};
use super::profile::{normalize_profile_draft, normalize_scene_name, scene_profile_record_input};

/// 将场景同步业务输入规范化为 DB 写入 DTO。
pub fn normalize_scene_sync_record_input(
    input: SceneSyncInput,
) -> Result<SceneSyncRecordInput, AppError> {
    scene_sync_record_input(input)
}

fn scene_sync_record_input(input: SceneSyncInput) -> Result<SceneSyncRecordInput, AppError> {
    Ok(SceneSyncRecordInput {
        profiles: input
            .profiles
            .into_iter()
            .map(scene_profile_record_draft)
            .collect::<Result<Vec<_>, _>>()?,
        occurrences: input
            .occurrences
            .into_iter()
            .map(scene_occurrence_record_input)
            .collect::<Result<Vec<_>, _>>()?,
        transitions: input
            .transitions
            .into_iter()
            .map(scene_transition_record_input)
            .collect::<Result<Vec<_>, _>>()?,
        entity_states: input
            .entity_states
            .into_iter()
            .map(scene_entity_state_record_input)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn scene_profile_record_draft(
    draft: SceneProfileDraft,
) -> Result<SceneProfileRecordDraft, AppError> {
    let draft = normalize_profile_draft(draft)?;
    let scene_ref = draft.scene_ref.clone();
    let normalized = normalize_scene_name(&draft.name)?;
    Ok(SceneProfileRecordDraft {
        scene_ref,
        scene_profile_id: draft.scene_profile_id,
        input: scene_profile_record_input(draft, normalized),
    })
}

fn scene_occurrence_record_input(
    draft: SceneOccurrenceDraft,
) -> Result<SceneOccurrenceRecordInput, AppError> {
    Ok(SceneOccurrenceRecordInput {
        occurrence_ref: draft.occurrence_ref,
        scene_ref: draft.scene_ref,
        occurrence_order: validate_positive_i32(draft.occurrence_order, "occurrence_order")?,
        timeline_position: trimmed_optional(draft.timeline_position.as_deref()),
        summary: required_text(&draft.summary, "场景出场摘要")?,
        mood: trimmed_optional(draft.mood.as_deref()),
        weather_or_lighting: trimmed_optional(draft.weather_or_lighting.as_deref()),
        source_span_ids_json: json!(draft.source_anchor_ids),
        evidence_json: json!(draft.evidence),
        characters_json: draft.characters,
        objects_json: draft.objects,
        key_events_json: draft.key_events,
        entry_state_json: draft.entry_state,
        exit_state_json: draft.exit_state,
    })
}

fn scene_transition_record_input(
    draft: SceneTransitionDraft,
) -> Result<SceneTransitionRecordInput, AppError> {
    Ok(SceneTransitionRecordInput {
        from_scene_ref: draft.from_scene_ref,
        to_scene_ref: draft.to_scene_ref,
        occurrence_ref: draft.occurrence_ref,
        transition_order: validate_positive_i32(draft.transition_order, "transition_order")?,
        movement_type: normalize_movement_type_value(&draft.movement_type)?,
        route_description: required_text(&draft.route_description, "位移描述")?,
        time_cost: trimmed_optional(draft.time_cost.as_deref()),
        distance_hint: trimmed_optional(draft.distance_hint.as_deref()),
        participants_json: draft.participants,
        object_refs_json: draft.object_refs,
        source_span_ids_json: json!(draft.source_anchor_ids),
        evidence_json: json!(draft.evidence),
    })
}

fn scene_entity_state_record_input(
    draft: SceneEntityStateDraft,
) -> Result<SceneEntityStateRecordInput, AppError> {
    Ok(SceneEntityStateRecordInput {
        scene_ref: draft.scene_ref,
        occurrence_ref: draft.occurrence_ref,
        entity_type: normalize_scene_entity_type_value(&draft.entity_type)?,
        entity_id: draft.entity_id,
        entity_name_snapshot: required_text(&draft.entity_name_snapshot, "实体名称快照")?,
        state_key: normalize_scene_entity_state_key_value(&draft.state_key)?,
        state_before_json: draft.state_before,
        state_after_json: draft.state_after,
        position_description: trimmed_optional(draft.position_description.as_deref()),
        action_description: required_text(&draft.action_description, "实体动作描述")?,
        source_span_ids_json: json!(draft.source_anchor_ids),
        evidence_json: json!(draft.evidence),
    })
}
