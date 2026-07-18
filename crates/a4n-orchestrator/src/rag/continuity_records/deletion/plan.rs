use crate::character::{CharacterProfileDeletionSummary, CharacterTimelineDeletionSummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CharacterRagDeletionPlan<'a> {
    pub(super) project_id: i64,
    pub(super) profile_id: Option<i64>,
    pub(super) state_index_ids: &'a [i64],
    pub(super) attribute_event_ids: &'a [i64],
}

impl<'a> CharacterRagDeletionPlan<'a> {
    pub(super) fn for_profile(deletion: &'a CharacterProfileDeletionSummary) -> Self {
        Self {
            project_id: deletion.project_id,
            profile_id: Some(deletion.profile_id),
            state_index_ids: &deletion.state_index_ids,
            attribute_event_ids: &deletion.attribute_event_ids,
        }
    }

    pub(super) fn for_timeline(
        project_id: i64,
        deletion: &'a CharacterTimelineDeletionSummary,
    ) -> Self {
        Self {
            project_id,
            profile_id: None,
            state_index_ids: &deletion.state_index_ids,
            attribute_event_ids: &deletion.attribute_event_ids,
        }
    }
}
