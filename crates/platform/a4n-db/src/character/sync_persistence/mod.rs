mod contracts;
mod execution;
mod snapshots;
mod validation;
mod write_steps;

pub use contracts::{
    AppliedCharacterSyncPersistenceRecord, CharacterSyncAttributeEventRecordInput,
    CharacterSyncPersistenceInput, CharacterSyncProtectedAttribute,
    CharacterSyncWriteStepRecordInput, CharacterSyncWriteSummary,
};
pub use execution::apply_character_sync_persistence_record;
pub use snapshots::character_sync_attribute_events_at_chapter;

pub(in crate::character) use contracts::CharacterSyncStoredSnapshotsRecord;

#[cfg(test)]
mod tests;
