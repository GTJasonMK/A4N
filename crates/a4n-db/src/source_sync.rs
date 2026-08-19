use sea_orm::DbErr;

use crate::project::{
    SOURCE_SYNC_CONFLICT, SOURCE_SYNC_CURRENT, SOURCE_SYNC_DETACHED, SOURCE_SYNC_ROOT_UPDATED,
};

mod candidates;
mod conflicts;
mod decisions;
mod dry_run;
mod galgame;
mod generation_specs;
mod plan;
mod preview;
mod run;
mod snapshots;

pub use candidates::{
    SourceSyncAnchorConflictCandidateRecord, SourceSyncProfileGenerationSpecRefRecord,
    anchor_conflict_candidates_for_run, generation_specs_for_profile_ref,
    generation_specs_for_source_chapter, local_state_for_generation_spec,
};
pub use conflicts::{
    SourceSyncConflictRecordInput, SourceSyncDecisionRecordInput, detach_derived_project_record,
    source_sync_conflict_for_decision,
};
pub(in crate::source_sync) use conflicts::{
    insert_source_sync_conflict_record, insert_source_sync_decision_record,
    resolve_source_sync_conflict_record,
};
pub use decisions::{
    PreparedSourceSyncDecisionRecordInput, SourceSyncRefreshSpecRecordInput,
    apply_prepared_source_sync_decisions_records_transaction,
};
pub use dry_run::{
    CreateSourceSyncDryRunRecordInput, UpdateSourceSyncDryRunSummaryInput,
    insert_source_sync_dry_run_record, update_source_sync_dry_run_summary,
};
use galgame::validate_prepared_galgame_scene_regeneration_record;
pub use galgame::{
    SourceSyncGalgameAnchorConflictCandidateRecord, SourceSyncGalgameSceneConflictCandidateRecord,
    SourceSyncGalgameSceneLineWriteRecordInput, SourceSyncGalgameSceneRegenerationApplyRecord,
    SourceSyncGalgameSceneRegenerationSourceRecord, SourceSyncGalgameSceneWriteRecordInput,
    galgame_scene_regeneration_source_record,
};
pub(in crate::source_sync) use galgame::{
    apply_galgame_scene_regeneration_record, galgame_anchor_conflict_candidates_for_run,
    galgame_scene_conflict_candidates_for_chapter_change,
};
pub use generation_specs::{
    SourceSyncAnchorMappingRecord, SourceSyncEntityRefRecordInput,
    SourceSyncManualPromptArtifactRecordInput, SourceSyncMediaRefRecordInput,
    SourceSyncPromptArtifactRecordInput, SourceSyncReplacementSegmentRecordInput,
    SourceSyncReplacementSpecRecordInput, SourceSyncTextAnchorRefRecordInput,
};
use generation_specs::{
    create_source_sync_replacement_spec_record, source_sync_params_with_prompt_kind,
};
pub(in crate::source_sync) use generation_specs::{
    delete_generation_spec_records, delete_local_generation_spec_record,
    generation_spec_for_source_sync_run, insert_generation_spec_ref_records,
    insert_manual_source_sync_prompt_artifact_record, rebind_generation_spec_anchor_records,
    render_and_insert_source_sync_prompt_artifact_record, replace_generation_spec_ref_records,
    source_snapshot_chapter_for_source, update_generation_spec_params_record,
    update_generation_spec_to_snapshot_record,
};
pub use plan::SourceSyncPlanRecord;
use plan::source_sync_plan_record;
pub use preview::create_source_sync_preview_records;
pub(in crate::source_sync) use run::finalize_source_sync_apply_run_record;
pub use run::{
    FinalizeSourceSyncApplyRunInput, apply_unconflicted_source_sync_records_transaction,
    load_source_sync_apply_run_record, source_sync_run_by_id,
};
pub use snapshots::{
    SourceSnapshotChapterChangeRecord, SourceSnapshotDiffRecord, SourceSnapshotProfileChangeRecord,
    diff_source_snapshots, source_sync_chapter_scope_for_derived_project,
};

const SYNC_RUN_STATUS_PLANNED: &str =
    a4n_domain::source_sync::SourceSyncRunStatus::Planned.as_str();
const SYNC_RUN_MODE_DRY_RUN: &str = a4n_domain::source_sync::SourceSyncRunMode::DryRun.as_str();
const SYNC_RUN_MODE_APPLY: &str = a4n_domain::source_sync::SourceSyncRunMode::Apply.as_str();
const SYNC_RUN_STATUS_APPLYING: &str =
    a4n_domain::source_sync::SourceSyncRunStatus::Applying.as_str();
const SYNC_RUN_STATUS_COMPLETED: &str =
    a4n_domain::source_sync::SourceSyncRunStatus::Completed.as_str();
const CONFLICT_STATUS_PENDING: &str =
    a4n_domain::source_sync::SourceSyncConflictStatus::Pending.as_str();
const CONFLICT_STATUS_RESOLVED: &str =
    a4n_domain::source_sync::SourceSyncConflictStatus::Resolved.as_str();
#[derive(Debug, thiserror::Error)]
pub enum SourceSyncDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("当前状态 {current} 不允许执行 {operation}")]
    InvalidState { current: String, operation: String },
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    Internal(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, SourceSyncDbError> {
    let value = value.trim();
    if value.is_empty() {
        Err(SourceSyncDbError::InvalidArgument(format!(
            "{label} 不能为空"
        )))
    } else {
        Ok(value)
    }
}

fn required_owned_text(value: String, label: &str) -> Result<String, SourceSyncDbError> {
    Ok(required_text(&value, label)?.to_string())
}

fn source_sync_generation_db_error(err: crate::generation::GenerationDbError) -> SourceSyncDbError {
    match err {
        crate::generation::GenerationDbError::InvalidArgument(message) => {
            SourceSyncDbError::InvalidArgument(message)
        }
        crate::generation::GenerationDbError::NotFound(message) => {
            SourceSyncDbError::NotFound(message)
        }
        crate::generation::GenerationDbError::Database(err) => SourceSyncDbError::Database(err),
    }
}

fn validate_positive_id(value: i64, label: &str) -> Result<(), SourceSyncDbError> {
    if value > 0 {
        Ok(())
    } else {
        Err(SourceSyncDbError::InvalidArgument(format!(
            "{label} 必须为正数"
        )))
    }
}
