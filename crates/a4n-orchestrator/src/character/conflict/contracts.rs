use std::collections::BTreeSet;

use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use crate::character::conflict_context_rules::CharacterConflictChapterScope;

pub type CharacterConflictProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterConflictCheckInput {
    pub project_id: i64,
    pub name: String,
    pub start_chapter: Option<i32>,
    pub end_chapter: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct CharacterConflictRequestPreview {
    pub request: ChatRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterConflictTaskScope {
    pub project_id: i64,
    pub total: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RequiredCharacterConflictInput {
    pub(super) project_id: i64,
    pub(super) name: String,
    pub(super) chapter_scope: CharacterConflictChapterScope,
}

pub(super) struct PreparedCharacterConflictCheck {
    pub(super) project_id: i64,
    pub(super) name: String,
    pub(super) chapter_scope: CharacterConflictChapterScope,
    pub(super) call_key: String,
    pub(super) request: ChatRequest,
    pub(super) source_ids: BTreeSet<String>,
}

pub(super) type CharacterConflictCheckResult = Result<Vec<Value>, AppError>;
