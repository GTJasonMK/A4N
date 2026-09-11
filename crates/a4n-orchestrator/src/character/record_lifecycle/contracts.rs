use a4n_error::AppError;
use a4n_rag_core::RagSyncError;

use crate::rag::{CharacterProfileRagDeletionSummary, CharacterTimelineRagDeletionSummary};
use a4n_db::character::{CharacterProfileDeletionSummary, CharacterProfileRollbackResult};

#[derive(Debug, thiserror::Error)]
pub enum CharacterRecordLifecycleError {
    #[error(transparent)]
    Persistence(AppError),
    #[error(transparent)]
    Rag(RagSyncError),
}

#[derive(Debug, PartialEq, Eq)]
pub struct CharacterProfileRecordDeletionResult {
    pub deletion: CharacterProfileDeletionSummary,
    pub rag: CharacterProfileRagDeletionSummary,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CharacterTimelineRollbackLifecycleResult {
    pub rollback: CharacterProfileRollbackResult,
    pub rag: CharacterTimelineRagDeletionSummary,
}
