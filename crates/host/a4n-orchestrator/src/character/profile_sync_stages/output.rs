mod evidence;
mod fields;
mod validation;

use a4n_domain::json_contract::reject_unknown_fields;
use serde_json::{Value, json};

use crate::character::{
    project_character_attribute_changes, required_array, required_i32_at, required_string,
    required_string_array,
};

use super::contracts::{
    CharacterProfileSyncStage, CharacterProfileSyncStageInput, CharacterProfileSyncStageOutputs,
};
use evidence::{
    ChapterEvidenceIndex, ensure_stage_anchor_exists, validate_sync_evidence_against_chapters,
};
use validation::{
    chapter_state_drafts, chapter_state_drafts_json, sort_sync_items_by_chapter,
    validate_attribute_change, validate_behavior, validate_chapter_state,
    validate_deletion_candidate, validate_sync_item_chapters, validate_unique_deletion_candidates,
};

pub(super) fn parse_character_profile_sync_stage_output(
    stage: CharacterProfileSyncStage,
    content: &str,
    input: &CharacterProfileSyncStageInput,
    outputs: &mut CharacterProfileSyncStageOutputs,
) -> Result<Value, String> {
    let value: Value = serde_json::from_str(content.trim())
        .map_err(|error| format!("{}必须是 JSON 对象: {error}", stage.title()))?;
    if !value.is_object() {
        return Err(format!("{}必须是 JSON 对象", stage.title()));
    }
    reject_unknown_fields(&value, "root", &[stage.root_field()])?;
    match stage {
        CharacterProfileSyncStage::EvidenceEvents => {
            outputs.evidence_events = parse_sync_evidence_events(&value, input)?;
            Ok(json!({ "events": outputs.evidence_events }))
        }
        CharacterProfileSyncStage::AttributeChanges => {
            let mut changes = parse_sync_attribute_changes(&value, input)?;
            let stage_output = json!({ "attribute_changes": changes });
            validate_sync_evidence_against_chapters(
                &mut changes,
                &mut [],
                &mut [],
                &input.chapter_data,
            )?;
            project_character_attribute_changes(input.baseline_state.clone(), &changes)?;
            outputs.changes = changes;
            Ok(stage_output)
        }
        CharacterProfileSyncStage::ChapterStates => {
            outputs.chapter_states = parse_sync_chapter_states(&value, input)?;
            Ok(json!({ "chapter_states": chapter_state_drafts_json(&outputs.chapter_states) }))
        }
        CharacterProfileSyncStage::BehaviorRecords => {
            let mut behaviors = parse_sync_behaviors(&value, input)?;
            let stage_output = json!({ "behaviors": behaviors });
            validate_sync_evidence_against_chapters(
                &mut [],
                &mut behaviors,
                &mut [],
                &input.chapter_data,
            )?;
            outputs.behaviors = behaviors;
            Ok(stage_output)
        }
        CharacterProfileSyncStage::DeletionCandidates => {
            let mut candidates = parse_sync_deletion_candidates(&value, input)?;
            let stage_output = json!({ "deletion_candidates": candidates });
            validate_sync_evidence_against_chapters(
                &mut [],
                &mut [],
                &mut candidates,
                &input.chapter_data,
            )?;
            outputs.deletion_candidates = candidates;
            Ok(stage_output)
        }
    }
}

fn parse_sync_evidence_events(
    value: &Value,
    input: &CharacterProfileSyncStageInput,
) -> Result<Vec<Value>, String> {
    let events = required_array(value, "events")?.clone();
    validate_sync_item_chapters("events", &events, &input.selected_chapter_numbers)?;
    let chapter_index = ChapterEvidenceIndex::new(&input.chapter_data);
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
        ensure_stage_anchor_exists(event, "events", index, "evidence_anchor_id", &chapter_index)?;
    }
    Ok(events)
}

fn parse_sync_attribute_changes(
    value: &Value,
    input: &CharacterProfileSyncStageInput,
) -> Result<Vec<Value>, String> {
    let mut changes = required_array(value, "attribute_changes")?.clone();
    for (index, change) in changes.iter().enumerate() {
        validate_attribute_change(index, change)?;
    }
    validate_sync_item_chapters(
        "attribute_changes",
        &changes,
        &input.selected_chapter_numbers,
    )?;
    sort_sync_items_by_chapter(&mut changes)?;
    Ok(changes)
}

fn parse_sync_chapter_states(
    value: &Value,
    input: &CharacterProfileSyncStageInput,
) -> Result<Vec<super::contracts::CharacterProfileSyncChapterStateDraft>, String> {
    for (index, state) in required_array(value, "chapter_states")?.iter().enumerate() {
        validate_chapter_state(index, state)?;
    }
    chapter_state_drafts(value, &input.selected_chapter_numbers)
}

fn parse_sync_behaviors(
    value: &Value,
    input: &CharacterProfileSyncStageInput,
) -> Result<Vec<Value>, String> {
    let mut behaviors = required_array(value, "behaviors")?.clone();
    for (index, behavior) in behaviors.iter().enumerate() {
        validate_behavior(index, behavior)?;
    }
    validate_sync_item_chapters("behaviors", &behaviors, &input.selected_chapter_numbers)?;
    sort_sync_items_by_chapter(&mut behaviors)?;
    Ok(behaviors)
}

fn parse_sync_deletion_candidates(
    value: &Value,
    input: &CharacterProfileSyncStageInput,
) -> Result<Vec<Value>, String> {
    let mut candidates = required_array(value, "deletion_candidates")?.clone();
    for (index, candidate) in candidates.iter().enumerate() {
        validate_deletion_candidate(index, candidate)?;
    }
    validate_unique_deletion_candidates(&candidates)?;
    validate_sync_item_chapters(
        "deletion_candidates",
        &candidates,
        &input.selected_chapter_numbers,
    )?;
    sort_sync_items_by_chapter(&mut candidates)?;
    Ok(candidates)
}
