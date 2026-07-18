//! 源文本同步计划、冲突决策与 Galgame 场景再生成编排。

mod contracts;
mod decisions;
mod preparation;
mod preview;
mod rules;
mod runs;
mod service;

pub use contracts::{
    GalgameSourceSyncExecutionContext, SourceSyncDecisionInput, SourceSyncExecutionContext,
    SourceSyncPlan,
};
pub use rules::{
    CONFLICT_SEVERITIES, CONFLICT_STATUS_IGNORED, CONFLICT_STATUS_PENDING,
    CONFLICT_STATUS_RESOLVED, CONFLICT_STATUSES, DECISION_ACTORS, LOCAL_STATE_KINDS,
    RESOLUTION_STRATEGIES, SOURCE_CHANGE_KINDS, SYNC_RUN_MODE_APPLY, SYNC_RUN_MODE_DRY_RUN,
    SYNC_RUN_STATUS_APPLYING, SYNC_RUN_STATUS_COMPLETED, SYNC_RUN_STATUS_PLANNED,
    SYNC_RUN_STATUSES, is_valid_conflict_severity, is_valid_conflict_status,
    is_valid_decision_actor, is_valid_local_state_kind, is_valid_resolution_strategy,
    is_valid_source_change_kind, is_valid_sync_run_mode, is_valid_sync_run_status,
};

#[cfg(test)]
use a4n_core::models::source_sync::source_sync_conflict;
#[cfg(test)]
use contracts::{RefreshSpecPayload, RegenerateGalgameScenePayload};
#[cfg(test)]
use preparation::prepared_source_sync_decisions;
#[cfg(test)]
use rules::source_sync_decisions_require_galgame_preparation;

#[cfg(test)]
mod tests;

fn source_sync_db_error(
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
    SourceSyncMediaRefRecordInput, SourceSyncPreviewRecord,
    SourceSyncProfileGenerationSpecRefRecord, SourceSyncPromptArtifactRecordInput,
    SourceSyncReplacementSegmentRecordInput, SourceSyncTextAnchorRefRecordInput,
    UpdateSourceSyncDryRunSummaryInput,
};
