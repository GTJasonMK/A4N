#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterProfileRagDeletionSummary {
    pub deleted_profile_rag: usize,
    pub deleted_state_rag: usize,
    pub deleted_attribute_event_rag: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterTimelineRagDeletionSummary {
    pub deleted_state_rag: usize,
    pub deleted_attribute_event_rag: usize,
}
