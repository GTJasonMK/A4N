mod cleanup;
mod conflict;
mod contracts;
mod projection;
mod records;
mod rollback;

pub use cleanup::delete_ai_timeline_artifacts_from_chapter_checked;
pub use cleanup::delete_timeline_artifacts_from_chapter_checked;
pub use conflict::character_conflict_context_rows;
pub(in crate::character) use contracts::CharacterStateIndexUpsertInput;
pub use contracts::{
    CharacterConflictContextRows, CharacterProfileRollbackResult, CharacterTimelineDeletionSummary,
};
pub(in crate::character) use projection::{
    CharacterAttributeOwnership, character_attribute_ownership,
};
pub use projection::{
    replay_profile_state_through_chapter, replay_profile_state_through_chapter_by_id,
    retract_profile_chapter_facts,
};
pub use records::behavior_records_before_chapter;
pub(in crate::character) use records::upsert_state_index;
pub use rollback::rollback_profile_to_chapter;

#[cfg(test)]
mod tests;
