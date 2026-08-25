use crate::project::ChapterDerivedCleanupArtifacts;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ChapterDerivedRagCleanupPlan<'a> {
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) chapter_number: i32,
    pub(super) stale_profile_ids: &'a [i64],
    pub(super) stale_character_state_ids: &'a [i64],
    pub(super) stale_attribute_event_ids: &'a [i64],
    pub(super) changed_foreshadowing_ids: &'a [i64],
}

impl<'a> ChapterDerivedRagCleanupPlan<'a> {
    pub(super) fn new(
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        artifacts: &'a ChapterDerivedCleanupArtifacts,
    ) -> Self {
        Self {
            project_id,
            chapter_id,
            chapter_number,
            stale_profile_ids: &artifacts.stale_profile_ids,
            stale_character_state_ids: &artifacts.stale_character_state_ids,
            stale_attribute_event_ids: &artifacts.stale_attribute_event_ids,
            changed_foreshadowing_ids: &artifacts.changed_foreshadowing_ids,
        }
    }
}
