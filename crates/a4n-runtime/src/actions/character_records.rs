mod access;
mod attributes;
mod behaviors;
mod deletion_marks;
mod profiles;
mod projections;
mod snapshots;

pub use a4n_api_contract::character::{
    AddProfileAttributeInput, AttributeCountsSummary, AttributeEventSummary, AttributeHistoryInput,
    BehaviorRecordSummary, CharacterProfileCreateResult, CharacterProfileDeleteResult,
    CharacterProfileMutationResult, CharacterProfileSummary, CheckProfileSyncConflictInput,
    CreateProfileInput, DeletionMarkSummary, DiffProfileChaptersInput,
    ExecuteAttributeDeletionInput, GetProfileStateAtChapterInput, GetSnapshotInput,
    ListBehaviorsInput, ListDeletionMarksInput, ListProfilesInput, ListSnapshotsInput,
    ModifyProfileAttributeInput, ProfileChapterStateSummary, ProfileSyncConflictSummary,
    RequestAttributeDeletionInput, ResetDeletionMarksInput, RollbackResultSummary,
    RollbackToChapterInput, SnapshotDeltaSummary, SnapshotReferenceSummary, SnapshotSummary,
    UpdateCharacterProfileInput,
};
pub use attributes::{add_profile_attribute, attribute_history, modify_profile_attribute};
pub use behaviors::list_behaviors;
pub use deletion_marks::{
    execute_attribute_deletion, list_deletion_marks, request_attribute_deletion,
    reset_deletion_marks,
};
pub use profiles::{
    create_profile, delete_profile, get_character_arc_status, get_character_voice_fingerprint,
    list_profiles, update_character_profile,
};
pub(crate) use projections::deletion_mark_summary;
pub use snapshots::{
    check_profile_sync_conflict, diff_profile_chapters, get_profile_state_at_chapter, get_snapshot,
    list_snapshots, rollback_to_chapter,
};

#[cfg(test)]
mod tests;
