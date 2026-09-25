//! Wire contracts for durable workflow lifecycle mutations.

pub use a4n_api_contract::workflow::WorkflowCancellationResult;
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

pub const WORKFLOW_LIFECYCLE_WORKER_ID: &str = "workflow-lifecycle-worker";
pub const WORKFLOW_LIFECYCLE_WORKER_CONTRACT_VERSION: &str = "workflow-lifecycle-v2";
pub const WORKFLOW_LIFECYCLE_CAPABILITY: &str = "workflow.lifecycle";
pub const WORKFLOW_LIFECYCLE_MARK_RUNNING_OPERATION: &str = "workflow.lifecycle.mark_running";
pub const WORKFLOW_LIFECYCLE_CANCEL_OPERATION: &str = "workflow.lifecycle.cancel";
pub const WORKFLOW_LIFECYCLE_RESUME_OPERATION: &str = "workflow.lifecycle.resume";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowRunIdInput {
    pub workflow_run_id: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowCancellationInput {
    pub workflow_run_id: i64,
    pub reason: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResumeWorkflowInput {
    pub workflow_run_id: i64,
    pub active_task_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationAck {
    pub accepted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn lifecycle_request_uses_the_worker_contract() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::Service,
                subject: "runtime".into(),
            },
            operation_id: WORKFLOW_LIFECYCLE_CANCEL_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: WORKFLOW_LIFECYCLE_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: WorkflowCancellationInput {
                workflow_run_id: 9,
                reason: "用户取消".into(),
            },
        };

        request.validate().expect("lifecycle request validates");
    }
}
