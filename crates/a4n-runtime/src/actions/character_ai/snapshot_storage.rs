#[cfg(test)]
use a4n_core::models::character::attribute_event;
use a4n_orchestrator::character::CharacterSyncChapterStateRecordInput;
#[cfg(test)]
use a4n_orchestrator::character::character_sync_attribute_events_at_chapter;
#[cfg(test)]
use serde_json::Value;

use super::sync_response::ChapterStateDraft;

pub(super) fn chapter_state_record_inputs(
    chapter_states: &[ChapterStateDraft],
) -> Vec<CharacterSyncChapterStateRecordInput> {
    chapter_states
        .iter()
        .map(|chapter_state| CharacterSyncChapterStateRecordInput {
            chapter_number: chapter_state.chapter_number,
            state_summary: chapter_state.state_summary.clone(),
            current_location: chapter_state.current_location.clone(),
            current_goal: chapter_state.current_goal.clone(),
            emotional_state: chapter_state.emotional_state.clone(),
        })
        .collect()
}

#[cfg(test)]
pub(super) fn attribute_events_at_chapter(
    events: &[attribute_event::Model],
    chapter_number: i32,
) -> Value {
    character_sync_attribute_events_at_chapter(events, chapter_number)
}
