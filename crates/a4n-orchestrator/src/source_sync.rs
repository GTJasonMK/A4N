//! 源文本同步计划、冲突决策与 Galgame 场景再生成编排。

mod contracts;
mod decisions;
mod preparation;
mod rules;
mod service;
mod workflow_rules;

pub use contracts::{
    GalgameSourceSyncExecutionContext, SourceSyncDecisionInput, SourceSyncExecutionContext,
};
pub use service::ApplySourceSyncApplicationInput;
pub use workflow_rules::{
    source_sync_apply_business_key, source_sync_apply_workflow_fingerprint,
    source_sync_apply_workflow_run_input,
};

#[cfg(test)]
use contracts::{RefreshSpecPayload, RegenerateGalgameScenePayload};
#[cfg(test)]
use preparation::prepared_source_sync_decisions;
#[cfg(test)]
use rules::source_sync_decisions_require_galgame_preparation;

#[cfg(test)]
mod tests;

pub(crate) fn source_sync_db_error(
    error: a4n_db::source_sync::SourceSyncDbError,
) -> a4n_core::error::AppError {
    match error {
        a4n_db::source_sync::SourceSyncDbError::InvalidArgument(message) => {
            a4n_core::error::AppError::InvalidArgument(message)
        }
        a4n_db::source_sync::SourceSyncDbError::InvalidState { current, operation } => {
            a4n_core::error::AppError::InvalidState { current, operation }
        }
        a4n_db::source_sync::SourceSyncDbError::NotFound(message) => {
            a4n_core::error::AppError::NotFound(message)
        }
        a4n_db::source_sync::SourceSyncDbError::Forbidden(message) => {
            a4n_core::error::AppError::Forbidden(message)
        }
        a4n_db::source_sync::SourceSyncDbError::Internal(message) => {
            a4n_core::error::AppError::Internal(anyhow::anyhow!(message))
        }
        a4n_db::source_sync::SourceSyncDbError::Database(error) => {
            a4n_core::error::AppError::Database(error)
        }
    }
}

pub use a4n_db::source_sync::{
    CreateSourceSyncDryRunRecordInput, FinalizeSourceSyncApplyRunInput,
    SourceSnapshotChapterChangeRecord, SourceSnapshotDiffRecord, SourceSnapshotProfileChangeRecord,
    SourceSyncAnchorConflictCandidateRecord, SourceSyncAnchorMappingRecord,
    SourceSyncConflictRecordInput, SourceSyncDbError, SourceSyncDecisionRecordInput,
    SourceSyncEntityRefRecordInput, SourceSyncGalgameAnchorConflictCandidateRecord,
    SourceSyncGalgameSceneConflictCandidateRecord, SourceSyncGalgameSceneLineWriteRecordInput,
    SourceSyncGalgameSceneRegenerationApplyRecord, SourceSyncGalgameSceneRegenerationSourceRecord,
    SourceSyncGalgameSceneWriteRecordInput, SourceSyncManualPromptArtifactRecordInput,
    SourceSyncMediaRefRecordInput, SourceSyncPlanRecord, SourceSyncProfileGenerationSpecRefRecord,
    SourceSyncPromptArtifactRecordInput, SourceSyncReplacementSegmentRecordInput,
    SourceSyncTextAnchorRefRecordInput, UpdateSourceSyncDryRunSummaryInput,
};
