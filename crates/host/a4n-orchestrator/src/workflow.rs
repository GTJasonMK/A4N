mod checkpoint;
mod errors;
mod llm_once;
mod rules;
pub(crate) use llm_once::{WorkflowLlmOnceInput, workflow_json_request};
/// Projects the workflow owner's persistence summary for Host-facing ports.
pub(crate) fn workflow_run_summary_to_api(
    run: a4n_db::workflow::WorkflowRunSummary,
) -> a4n_api_contract::workflow::WorkflowRunSummary {
    a4n_workflow_core::workflow_run_summary_to_api(run)
}

pub(crate) use checkpoint::attach_llm_call_checkpoint_trace;
pub(crate) use checkpoint::{
    CompletedWorkflowCheckpointInput, required_checkpoint_field, required_checkpoint_json,
    workflow_checkpoint_fingerprint_sha256,
};

pub(crate) use a4n_db::workflow::{
    CancelWorkflowRunPersistenceResult, ClaimExclusiveWorkflowRunInput, CompletedLlmCallCheckpoint,
    CompletedLlmCallCheckpointInput, ListWorkflowRunsInput, UpsertWorkflowCheckpointInput,
    UpsertWorkflowRunInput, WorkflowCheckpointContractError, WorkflowProgressEventInput,
    WorkflowReadError, WorkflowRunRecoveryRecord, WorkflowRunStatusRecord, WorkflowRunSummary,
    WorkflowWriteError, llm_call_checkpoint_fingerprint_sha256, required_llm_call_workflow_run_id,
    workflow_run_is_running_or_resumable,
};
pub(crate) use a4n_domain::workflow::{
    WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING, WORKFLOW_STATUS_SUPERSEDED,
};

pub(crate) use errors::{
    workflow_checkpoint_contract_app_error, workflow_read_app_error, workflow_write_app_error,
};
pub(crate) use rules::require_workflow_can_resume;
pub(crate) use rules::{
    WorkflowRunFinalizeOutcome, is_video_status_final_for_cancel_interrupt,
    require_positive_workflow_run_id, validate_begin_workflow_run_input,
    workflow_run_finalize_status,
};
