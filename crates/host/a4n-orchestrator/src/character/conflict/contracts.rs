use std::collections::BTreeSet;

use a4n_llm_provider_contract::provider::ChatRequest;

pub use a4n_character_host_contract::CharacterConflictProgressReporter;

use crate::character::conflict::context_rules::CharacterConflictChapterScope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ValidatedCharacterConflictInput {
    pub(super) project_id: i64,
    pub(super) name: String,
    pub(super) chapter_scope: CharacterConflictChapterScope,
}

pub(super) struct PreparedCharacterConflictCheck {
    pub(super) input: ValidatedCharacterConflictInput,
    pub(super) profile_id: i64,
    pub(super) call_key: String,
    pub(super) request: ChatRequest,
    pub(super) source_ids: BTreeSet<String>,
}
