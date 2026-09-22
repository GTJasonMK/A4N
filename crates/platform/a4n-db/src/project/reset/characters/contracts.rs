use a4n_db_models::models::character::protagonist_profile;

#[derive(Debug, Clone, Default)]
pub(in crate::project::reset) struct ResetCharacterProfilesResult {
    pub(in crate::project::reset) reset_character_profiles: u64,
    pub(in crate::project::reset) deleted_character_profiles: u64,
    pub(in crate::project::reset) deleted_character_snapshots: u64,
    pub(in crate::project::reset) deleted_character_timeline_rows: u64,
    pub(in crate::project::reset) deleted_character_portraits: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct CharacterProfileRelatedCounts {
    pub(super) attribute_events: u64,
    pub(super) character_states: u64,
    pub(super) behaviors: u64,
    pub(super) deletion_marks: u64,
    pub(super) character_snapshots: u64,
    pub(super) character_portraits: u64,
}

impl CharacterProfileRelatedCounts {
    pub(super) fn timeline_rows(self) -> u64 {
        self.attribute_events + self.character_states + self.behaviors + self.deletion_marks
    }
}

pub(super) struct CharacterProfileResetScope {
    pub(super) text_profile_ids: Vec<i64>,
    pub(super) retained_profiles: Vec<protagonist_profile::Model>,
    pub(super) retained_profile_ids: Vec<i64>,
    pub(super) related_counts: CharacterProfileRelatedCounts,
}
