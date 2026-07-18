use crate::character::AppliedCharacterSyncPersistenceRecord;

#[derive(Debug, Clone, Copy)]
pub struct CharacterSyncRagInput<'a> {
    pub project_id: i64,
    pub profile_id: i64,
    pub character_name: &'a str,
    pub start_chapter: i32,
    pub clear_to_chapter: i32,
    pub applied: &'a AppliedCharacterSyncPersistenceRecord,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterSyncRagSummary {
    pub deleted_state_rag: usize,
    pub state_rag_indexed_count: usize,
    pub state_rag_indexed: bool,
    pub state_rag_error: Option<String>,
    pub deleted_attribute_event_rag: usize,
    pub attribute_event_rag_indexed_count: usize,
    pub attribute_event_rag_indexed: bool,
    pub attribute_event_rag_error: Option<String>,
}
