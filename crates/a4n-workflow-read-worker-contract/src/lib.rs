//! Typed workflow read commands owned by the workflow read worker.

pub use a4n_api_contract::task::{GetTaskHistoryInput, TaskInfoSummary};
pub use a4n_api_contract::workflow::{
    GetWorkflowRunInput, ListWorkflowRunsInput, WorkflowRunDetailSummary, WorkflowRunSummary,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

pub const WORKFLOW_READ_WORKER_ID: &str = "workflow-read-worker";
pub const WORKFLOW_READ_WORKER_CONTRACT_VERSION: &str = "workflow-read-v1";
pub const WORKFLOW_READ_LIST_OPERATION: &str = "workflow.read.list";
pub const WORKFLOW_READ_DETAIL_OPERATION: &str = "workflow.read.detail";
pub const WORKFLOW_READ_CAPABILITY: &str = "workflow.read";
pub const TASK_READ_HISTORY_OPERATION: &str = "task.read.history";

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
