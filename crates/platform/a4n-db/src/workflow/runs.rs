use super::*;

mod checkpoints;
mod claim;
mod mutations;
mod progress;
mod projections;
mod queries;
mod validation;

#[cfg(test)]
mod chapter_queries_tests;

pub use checkpoints::{
    completed_workflow_checkpoint_by_fingerprint, delete_workflow_checkpoints_by_keys,
    mutate_completed_workflow_checkpoint_artifacts_unless_step_exists, upsert_workflow_checkpoint,
    upsert_workflow_checkpoint_in_transaction, workflow_checkpoint_statuses_for_runs,
    workflow_checkpoint_step_statuses, workflow_checkpoint_summaries,
    workflow_checkpoint_summaries_for_runs, workflow_checkpoint_summary_by_step_in,
};
pub use claim::{claim_exclusive_workflow_run, claim_exclusive_workflow_run_summary};
pub use mutations::{
    continue_completed_workflow_run, continue_completed_workflow_run_summary,
    invalidate_chapter_analysis_after_source_change, mark_workflow_current_step,
    mark_workflow_current_step_summary, mark_workflow_status, mark_workflow_status_in_transaction,
    mark_workflow_status_summary, restart_interrupted_workflow_run,
    restart_interrupted_workflow_run_summary, resume_or_upsert_workflow_run,
    resume_or_upsert_workflow_run_summary, resume_workflow_run, resume_workflow_run_summary,
    resume_workflow_run_with_active_task, resume_workflow_run_with_active_task_summary,
    upsert_workflow_run, upsert_workflow_run_in_transaction, upsert_workflow_run_summary,
    upsert_workflow_run_summary_in_transaction,
};
pub use progress::{
    persist_workflow_progress_snapshot, persist_workflow_progress_snapshot_summary,
    record_workflow_progress_event, workflow_progress_events,
};
pub use queries::{
    latest_workflow_run_by_business_key, latest_workflow_run_by_business_key_for_update,
    latest_workflow_run_recovery_records_by_business_keys,
    latest_workflow_run_status_record_by_business_key, latest_workflow_run_summaries_for_chapters,
    latest_workflow_run_summary_by_business_key,
    latest_workflow_run_summary_by_business_key_for_update,
    latest_workflow_run_summary_by_business_key_with_status, latest_workflow_runs_by_business_keys,
    latest_workflow_runs_for_chapters, list_workflow_run_models, list_workflow_runs,
    workflow_run_access_record_by_active_task_id, workflow_run_detail, workflow_run_model_by_id,
    workflow_run_models_by_ids, workflow_run_resume_record_by_id,
    workflow_run_resume_records_by_project_kind, workflow_run_status_record_by_id,
    workflow_run_statuses_by_ids, workflow_run_summary_by_id, workflow_run_summary_by_id_record,
    workflow_runs_by_project_kind,
};

pub use projections::{
    workflow_checkpoint_summary as workflow_checkpoint_summary_from_model,
    workflow_run_summary as workflow_run_summary_from_model,
};

pub(super) use progress::trim_workflow_progress_events;
pub(crate) use queries::locked_workflow_run_by_id;
pub(super) use validation::ensure_workflow_write_open;

#[cfg(test)]
pub(in crate::workflow) use progress::validate_workflow_progress_range;
#[cfg(test)]
pub(in crate::workflow) use projections::{workflow_run_progress_summary, workflow_run_summary};
#[cfg(test)]
pub(in crate::workflow) use validation::{
    completed_at_for_status_value, ensure_workflow_status_transition,
    requires_explicit_interrupted_resume, should_resume_existing_run, should_update_existing_run,
    validate_workflow_run_input, workflow_fingerprint_matches,
};
