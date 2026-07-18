mod anchors;

use super::prelude::*;
use super::*;
use anchors::ensure_stage_anchor_exists;

pub(super) fn validate_extraction_evidence(
    mut extraction: CharacterSyncExtraction,
    context: &CharacterSyncContext,
) -> Result<CharacterSyncExtraction, String> {
    validate_sync_evidence_against_chapters(
        &mut extraction.changes,
        &mut extraction.behaviors,
        &mut extraction.deletion_candidates,
        &context.chapter_data,
    )?;
    validate_materialized_character_sync_projection(&extraction, &context.baseline_state)?;
    Ok(extraction)
}

pub(super) fn parse_character_sync_stage_output(
    stage: CharacterSyncStage,
    content: &str,
    context: &CharacterSyncContext,
    outputs: &mut CharacterSyncStageOutputs,
) -> Result<Value, String> {
    let value: Value = serde_json::from_str(content.trim())
        .map_err(|err| format!("{}必须是 JSON 对象: {err}", stage.title()))?;
    if !value.is_object() {
        return Err(format!("{}必须是 JSON 对象", stage.title()));
    }
    reject_unknown_fields(&value, "root", &[stage.root_field()])?;
    match stage {
        CharacterSyncStage::EvidenceEvents => {
            outputs.evidence_events = parse_sync_evidence_events(&value, context)?;
            Ok(json!({ "events": outputs.evidence_events }))
        }
        CharacterSyncStage::AttributeChanges => {
            outputs.changes = parse_sync_attribute_changes(&value, context)?;
            Ok(json!({ "attribute_changes": outputs.changes }))
        }
        CharacterSyncStage::ChapterStates => {
            outputs.chapter_states = parse_sync_chapter_states(&value, context)?;
            Ok(json!({ "chapter_states": chapter_state_drafts_json(&outputs.chapter_states) }))
        }
        CharacterSyncStage::BehaviorRecords => {
            outputs.behaviors = parse_sync_behaviors(&value, context)?;
            Ok(json!({ "behaviors": outputs.behaviors }))
        }
        CharacterSyncStage::DeletionCandidates => {
            outputs.deletion_candidates = parse_sync_deletion_candidates(&value, context)?;
            Ok(json!({ "deletion_candidates": outputs.deletion_candidates }))
        }
    }
}

pub(super) fn parse_sync_evidence_events(
    value: &Value,
    context: &CharacterSyncContext,
) -> Result<Vec<Value>, String> {
    let events = required_array(value, "events")?.clone();
    validate_sync_item_chapters("events", &events, &context.selected_chapter_numbers)?;
    for (index, event) in events.iter().enumerate() {
        if !event.is_object() {
            return Err(format!("events[{index}] 必须是 JSON 对象"));
        }
        required_i32_at(
            event,
            "chapter_number",
            &format!("events[{index}].chapter_number"),
        )?;
        required_string(event, "event_type")?;
        required_string(event, "summary")?;
        required_string_array(event, "related_characters")?;
        ensure_stage_anchor_exists(event, "events", index, "evidence_anchor_id", context)?;
    }
    Ok(events)
}

pub(super) fn parse_sync_attribute_changes(
    value: &Value,
    context: &CharacterSyncContext,
) -> Result<Vec<Value>, String> {
    let mut changes = required_array(value, "attribute_changes")?.clone();
    for (index, change) in changes.iter().enumerate() {
        validate_attribute_change(index, change)?;
    }
    validate_sync_item_chapters(
        "attribute_changes",
        &changes,
        &context.selected_chapter_numbers,
    )?;
    validate_sync_item_count_by_chapter(
        "attribute_changes",
        &changes,
        CHARACTER_SYNC_ATTRIBUTE_CHANGES_PER_CHAPTER_MAX,
    )?;
    sort_sync_items_by_chapter(&mut changes)?;
    Ok(changes)
}

pub(super) fn parse_sync_chapter_states(
    value: &Value,
    context: &CharacterSyncContext,
) -> Result<Vec<ChapterStateDraft>, String> {
    for (index, state) in required_array(value, "chapter_states")?.iter().enumerate() {
        validate_chapter_state(index, state)?;
    }
    chapter_state_drafts(value, &context.selected_chapter_numbers)
}

pub(super) fn chapter_state_drafts_json(states: &[ChapterStateDraft]) -> Value {
    Value::Array(
        states
            .iter()
            .map(|state| {
                json!({
                    "chapter_number": state.chapter_number,
                    "state_summary": &state.state_summary,
                    "current_location": &state.current_location,
                    "current_goal": &state.current_goal,
                    "emotional_state": &state.emotional_state})
            })
            .collect(),
    )
}

pub(super) fn parse_sync_behaviors(
    value: &Value,
    context: &CharacterSyncContext,
) -> Result<Vec<Value>, String> {
    let mut behaviors = required_array(value, "behaviors")?.clone();
    for (index, behavior) in behaviors.iter().enumerate() {
        validate_behavior(index, behavior)?;
    }
    validate_sync_item_chapters("behaviors", &behaviors, &context.selected_chapter_numbers)?;
    validate_sync_item_count_by_chapter(
        "behaviors",
        &behaviors,
        CHARACTER_SYNC_BEHAVIORS_PER_CHAPTER_MAX,
    )?;
    sort_sync_items_by_chapter(&mut behaviors)?;
    Ok(behaviors)
}

pub(super) fn parse_sync_deletion_candidates(
    value: &Value,
    context: &CharacterSyncContext,
) -> Result<Vec<Value>, String> {
    let mut candidates = required_array(value, "deletion_candidates")?.clone();
    for (index, candidate) in candidates.iter().enumerate() {
        validate_deletion_candidate(index, candidate)?;
    }
    validate_unique_deletion_candidates(&candidates)?;
    validate_sync_item_chapters(
        "deletion_candidates",
        &candidates,
        &context.selected_chapter_numbers,
    )?;
    validate_sync_item_count_by_chapter(
        "deletion_candidates",
        &candidates,
        CHARACTER_SYNC_DELETION_CANDIDATES_PER_CHAPTER_MAX,
    )?;
    sort_sync_items_by_chapter(&mut candidates)?;
    Ok(candidates)
}
