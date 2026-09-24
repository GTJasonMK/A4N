//! Typed workflow read commands owned by the workflow read worker.

pub use a4n_api_contract::task::{GetTaskHistoryInput, TaskInfoSummary};
pub use a4n_api_contract::workflow::{
    ActiveTaskLookupRecord, GetWorkflowRunInput, ListWorkflowRunsInput, TaskLookupRecord,
    WorkflowProgressEvent, WorkflowProgressEventsInput, WorkflowResumeRecord,
    WorkflowRunDetailSummary, WorkflowRunSummary, WorkflowTaskAccessRecord,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

pub const WORKFLOW_READ_WORKER_ID: &str = "workflow-read-worker";
pub const WORKFLOW_READ_WORKER_CONTRACT_VERSION: &str = "workflow-read-v2";
pub const WORKFLOW_READ_LIST_OPERATION: &str = "workflow.read.list";
pub const WORKFLOW_READ_DETAIL_OPERATION: &str = "workflow.read.detail";
pub const WORKFLOW_READ_SUMMARY_OPERATION: &str = "workflow.read.summary";
pub const WORKFLOW_READ_RESUME_RECORD_OPERATION: &str = "workflow.read.resume_record";
pub const WORKFLOW_READ_PROGRESS_EVENTS_OPERATION: &str = "workflow.read.progress_events";
pub const WORKFLOW_READ_CAPABILITY: &str = "workflow.read";
pub const TASK_READ_HISTORY_OPERATION: &str = "task.read.history";
pub const TASK_READ_HISTORY_CAPABILITY: &str = "task.read.history";
pub const WORKFLOW_TASK_LOOKUP_CAPABILITY: &str = "workflow.task_lookup";
pub const TASK_LOOKUP_BY_ID_OPERATION: &str = "task.lookup.by_id";
pub const TASK_LOOKUP_ACTIVE_OPERATION: &str = "task.lookup.active";
pub const WORKFLOW_ACCESS_ACTIVE_TASK_OPERATION: &str = "workflow.access.active_task";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskIdInput {
    pub task_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmptyInput {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowRunIdInput {
    pub workflow_run_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn workflow_read_scope_is_stable() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: WORKFLOW_READ_DETAIL_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(9),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: WORKFLOW_READ_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: GetWorkflowRunInput {
                workflow_run_id: 9,
                include_generation_runs: true,
            },
        };
        request.validate().expect("workflow read request validates");
    }
}
