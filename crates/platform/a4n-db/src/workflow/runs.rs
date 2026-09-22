use super::*;

mod checkpoints;
mod claim;
mod mutations;
mod progress;
mod projections;
mod queries;
mod validation;

pub use checkpoints::{
    completed_workflow_checkpoint_by_fingerprint, delete_workflow_checkpoints_by_keys,
    mutate_completed_workflow_checkpoint_artifacts_unless_step_exists, upsert_workflow_checkpoint,
    upsert_workflow_checkpoint_in_transaction, workflow_checkpoint_model_by_step_in,
    workflow_checkpoint_models, workflow_checkpoint_models_for_runs,
    workflow_checkpoint_statuses_for_runs, workflow_checkpoint_step_statuses,
};
pub use claim::claim_exclusive_workflow_run;
pub use mutations::{
    invalidate_chapter_analysis_after_source_change, mark_workflow_current_step,
    mark_workflow_status, resume_or_upsert_workflow_run, resume_workflow_run,
    resume_workflow_run_with_active_task, upsert_workflow_run, upsert_workflow_run_in_transaction,
};
pub use progress::{
    persist_workflow_progress_snapshot, record_workflow_progress_event, workflow_progress_events,
};
pub use queries::{
    latest_workflow_run_by_business_key, latest_workflow_run_by_business_key_for_update,
    latest_workflow_run_recovery_records_by_business_keys,
    latest_workflow_run_status_record_by_business_key, latest_workflow_runs_by_business_keys,
    list_workflow_run_models, list_workflow_runs, workflow_run_access_record_by_active_task_id,
    workflow_run_detail, workflow_run_model_by_id, workflow_run_models_by_ids,
    workflow_run_status_record_by_id, workflow_run_statuses_by_ids, workflow_run_summary_by_id,
    workflow_runs_by_project_kind,
};

pub(super) use progress::trim_workflow_progress_events;
pub(crate) use queries::locked_workflow_run_by_id;

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
