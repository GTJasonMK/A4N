use std::collections::BTreeSet;

pub use a4n_api_contract::character::{CharacterConflictCheckInput, CharacterConflictSummary};
use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;

use crate::character::conflict_context_rules::CharacterConflictChapterScope;

pub type CharacterConflictProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

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
