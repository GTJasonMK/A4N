mod checkpoint;
mod errors;
mod rules;

pub(crate) use checkpoint::attach_llm_call_checkpoint_trace;
pub use checkpoint::{
    CompletedWorkflowCheckpointContractError, CompletedWorkflowCheckpointInput,
    required_checkpoint_field, required_checkpoint_json, workflow_checkpoint_fingerprint_sha256,
};

pub use a4n_db::workflow::{
    CancelWorkflowRunPersistenceResult, ClaimExclusiveWorkflowRunInput, CompletedLlmCallCheckpoint,
    CompletedLlmCallCheckpointInput, GenerationRunSummary, GetWorkflowRunInput,
    ListWorkflowRunsInput, PersistedTaskInfo, TaskProgressEventSummary,
    UpsertWorkflowCheckpointInput, UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED,
    WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_INTERRUPTED,
    WORKFLOW_STATUS_RUNNING, WORKFLOW_STATUS_SUPERSEDED, WorkflowCheckpointContractError,
    WorkflowCheckpointSummary, WorkflowProgressEventInput, WorkflowReadError,
    WorkflowRunDetailSummary, WorkflowRunProgressSummary, WorkflowRunSummary, WorkflowWriteError,
    completed_llm_call_checkpoint_from_model, llm_call_checkpoint_fingerprint_sha256,
    required_llm_call_workflow_run_id, workflow_run_is_running_or_resumable,
};

pub(crate) use errors::{
    workflow_checkpoint_contract_app_error, workflow_read_app_error, workflow_write_app_error,
};
pub(crate) use rules::require_workflow_can_resume;
pub use rules::{
    UnifiedResumeDispatch, WorkflowRunFinalizeOutcome, cancel_workflow_requires_target,
    is_video_status_final_for_cancel_interrupt, plan_unified_resume,
    require_positive_workflow_run_id, validate_begin_workflow_run_input,
    validate_optional_project_id, validate_task_history_limit, workflow_run_finalize_status,
};
