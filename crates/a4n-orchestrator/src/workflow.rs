mod checkpoint;
mod errors;
mod rules;
/// Projects the workflow owner's persistence summary for Host-facing ports.
pub fn workflow_run_summary_to_api(
    run: a4n_db::workflow::WorkflowRunSummary,
) -> a4n_api_contract::workflow::WorkflowRunSummary {
    a4n_api_contract::workflow::WorkflowRunSummary {
        id: run.id,
        business_key: run.business_key,
        workflow_kind: run.workflow_kind,
        project_id: run.project_id,
        root_project_id: run.root_project_id,
        source_snapshot_id: run.source_snapshot_id,
        scope_table: run.scope_table,
        scope_id: run.scope_id,
        chapter_id: run.chapter_id,
        mode: run.mode,
        input_fingerprint_json: run.input_fingerprint_json,
        status: run.status,
        current_step_key: run.current_step_key,
        progress: run.progress.map(|progress| {
            a4n_api_contract::workflow::WorkflowRunProgressSummary {
                current: progress.current,
                total: progress.total,
                message: progress.message,
                percent: progress.percent,
                meta: progress.meta,
                updated_at: progress.updated_at,
            }
        }),
        active_task_id: run.active_task_id,
        can_resume: run.can_resume,
        resume_block_reason: run.resume_block_reason,
        error_message: run.error_message,
        created_at: run.created_at,
        updated_at: run.updated_at,
        completed_at: run.completed_at,
    }
}

pub(crate) use checkpoint::attach_llm_call_checkpoint_trace;
pub use checkpoint::{
    CompletedWorkflowCheckpointContractError, CompletedWorkflowCheckpointInput,
    required_checkpoint_field, required_checkpoint_json, workflow_checkpoint_fingerprint_sha256,
};

pub use a4n_db::workflow::{
    CancelWorkflowRunPersistenceResult, ClaimExclusiveWorkflowRunInput, CompletedLlmCallCheckpoint,
    CompletedLlmCallCheckpointInput, GetWorkflowRunInput, ListWorkflowRunsInput, PersistedTaskInfo,
    TaskProgressEventSummary, UpsertWorkflowCheckpointInput, UpsertWorkflowRunInput,
    WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING, WORKFLOW_STATUS_SUPERSEDED,
    WorkflowCheckpointContractError, WorkflowProgressEventInput, WorkflowReadError,
    WorkflowRunAccessRecord, WorkflowRunDetailSummary, WorkflowRunRecoveryRecord,
    WorkflowRunStatusRecord, WorkflowRunSummary, WorkflowWriteError,
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
