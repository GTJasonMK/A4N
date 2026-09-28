use a4n_db_models::models::generation::{
    generation_run, workflow_checkpoint, workflow_progress_event, workflow_run,
};
#[cfg(test)]
use a4n_db_models::models::task::task_queue;
use a4n_progress::ProgressMeta;
#[cfg(test)]
use a4n_queue::priority::{TaskKind, TaskPriority, TaskStatus};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, NotSet,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::Deserialize;
use serde_json::{Value, json};

mod errors;
pub(in crate::workflow) use errors::WorkflowValidationError;
pub use errors::{WorkflowCheckpointContractError, WorkflowReadError, WorkflowWriteError};

mod status;
pub(crate) use a4n_domain::workflow::{
    WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING, WORKFLOW_STATUS_SUPERSEDED,
};
pub(in crate::workflow) use status::MAX_PERSISTED_PROGRESS_EVENTS;
pub(crate) use status::is_terminal_workflow_status;
pub use status::workflow_run_is_running_or_resumable;

mod inputs;
pub use inputs::{
    ClaimExclusiveWorkflowRunInput, GetWorkflowRunInput, ListWorkflowRunsInput,
    UpsertWorkflowCheckpointInput, UpsertWorkflowRunInput, WorkflowProgressEventInput,
};

mod summaries;
pub use summaries::{
    CancelWorkflowRunPersistenceResult, GenerationRunSummary, PersistedTaskInfo,
    TaskProgressEventSummary, WorkflowCheckpointSummary, WorkflowRunAccessRecord,
    WorkflowRunDetailSummary, WorkflowRunProgressSummary, WorkflowRunRecoveryRecord,
    WorkflowRunStatusRecord, WorkflowRunSummary,
};

mod validation;
pub(in crate::workflow) use validation::{
    non_empty_text, normalize_optional_text, required_text, validate_positive_i64,
    validate_workflow_status,
};

mod runs;
pub(crate) use runs::locked_workflow_run_by_id;
pub(in crate::workflow) use runs::trim_workflow_progress_events;
pub use runs::{
    claim_exclusive_workflow_run, completed_workflow_checkpoint_by_fingerprint,
    continue_completed_workflow_run, delete_workflow_checkpoints_by_keys,
    invalidate_chapter_analysis_after_source_change, latest_workflow_run_by_business_key,
    latest_workflow_run_by_business_key_for_update,
    latest_workflow_run_recovery_records_by_business_keys,
    latest_workflow_run_status_record_by_business_key, latest_workflow_runs_by_business_keys,
    latest_workflow_runs_for_chapters, list_workflow_run_models, list_workflow_runs,
    mark_workflow_current_step, mark_workflow_status,
    mutate_completed_workflow_checkpoint_artifacts_unless_step_exists,
    persist_workflow_progress_snapshot, record_workflow_progress_event,
    resume_or_upsert_workflow_run, resume_workflow_run, resume_workflow_run_with_active_task,
    upsert_workflow_checkpoint, upsert_workflow_checkpoint_in_transaction, upsert_workflow_run,
    upsert_workflow_run_in_transaction, workflow_checkpoint_model_by_step_in,
    workflow_checkpoint_models, workflow_checkpoint_models_for_runs,
    workflow_checkpoint_statuses_for_runs, workflow_checkpoint_step_statuses,
    workflow_progress_events, workflow_run_access_record_by_active_task_id, workflow_run_detail,
    workflow_run_model_by_id, workflow_run_models_by_ids, workflow_run_status_record_by_id,
    workflow_run_statuses_by_ids, workflow_run_summary_by_id, workflow_runs_by_project_kind,
};
#[cfg(test)]
pub(in crate::workflow) use runs::{
    completed_at_for_status_value, ensure_workflow_status_transition,
    requires_explicit_interrupted_resume, should_resume_existing_run, should_update_existing_run,
    validate_workflow_progress_range, validate_workflow_run_input, workflow_fingerprint_matches,
    workflow_run_progress_summary, workflow_run_summary,
};

mod tasks;
pub(crate) use tasks::task_info_from_row;
#[cfg(test)]
pub(in crate::workflow) use tasks::{
    WorkflowCheckpointVideoRefs, generated_video_ids_for_cancellation_checkpoints,
    persisted_task_info_from_row,
};
pub use tasks::{
    cancel_workflow_run_persistence, persisted_task_by_id, persisted_task_history,
    recover_stale_in_process_workflows,
};

mod llm_checkpoint;
#[cfg(test)]
pub(in crate::workflow) use llm_checkpoint::validate_completed_llm_call_checkpoint_json;
pub use llm_checkpoint::{
    CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput,
    completed_llm_call_checkpoint_from_model, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, reject_completed_llm_call_checkpoint,
    required_llm_call_workflow_run_id, save_completed_llm_call_checkpoint,
};

// ── LLM call level checkpoint 合同（阶段 11 收口） ──────────────────────────

#[cfg(test)]
mod tests;
