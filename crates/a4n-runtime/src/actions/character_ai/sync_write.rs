use serde_json::Value;

use super::json_utils::required_i32;
pub(super) use super::sync_write_records::{
    BuildCharacterSyncWriteStepsInput, build_character_sync_write_step_records,
};

#[derive(Debug, Clone)]
pub(super) struct AttributeEventDraft {
    pub(super) chapter_number: i32,
    pub(super) sequence: usize,
    pub(super) stage: u8,
    pub(super) category: String,
    pub(super) key: String,
    pub(super) operation: String,
    pub(super) old_value: Value,
    pub(super) new_value: Value,
    pub(super) change_description: String,
    pub(super) cause: String,
    pub(super) evidence: String,
    pub(super) from_classification: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SyncWriteStepKind {
    AttributeEvent,
    Behavior,
    DeletionCandidate,
    ImplicitReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SyncWriteStep {
    pub(super) chapter_number: i32,
    pub(super) kind: SyncWriteStepKind,
    pub(super) index: usize,
}

pub(super) fn sync_write_steps(
    chapter_numbers: &[i32],
    attribute_events: &[AttributeEventDraft],
    behaviors: &[Value],
    deletion_candidates: &[Value],
    implicit_review_decision_count: usize,
    target_chapter: i32,
) -> Result<Vec<SyncWriteStep>, String> {
    let mut steps = Vec::new();
    for chapter_number in chapter_numbers {
        for (index, event) in attribute_events.iter().enumerate() {
            if event.chapter_number == *chapter_number {
                steps.push(SyncWriteStep {
                    chapter_number: *chapter_number,
                    kind: SyncWriteStepKind::AttributeEvent,
                    index,
                });
            }
        }

        for (index, behavior) in behaviors.iter().enumerate() {
            if required_i32(behavior, "chapter_number")? == *chapter_number {
                steps.push(SyncWriteStep {
                    chapter_number: *chapter_number,
                    kind: SyncWriteStepKind::Behavior,
                    index,
                });
            }
        }

        for (index, candidate) in deletion_candidates.iter().enumerate() {
            if required_i32(candidate, "chapter_number")? == *chapter_number {
                steps.push(SyncWriteStep {
                    chapter_number: *chapter_number,
                    kind: SyncWriteStepKind::DeletionCandidate,
                    index,
                });
            }
        }

        if *chapter_number == target_chapter {
            for index in 0..implicit_review_decision_count {
                steps.push(SyncWriteStep {
                    chapter_number: *chapter_number,
                    kind: SyncWriteStepKind::ImplicitReview,
                    index,
                });
            }
        }
    }
    Ok(steps)
}
