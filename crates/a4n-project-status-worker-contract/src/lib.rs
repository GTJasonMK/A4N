//! Typed read commands owned by the project-status worker.

pub use a4n_api_contract::project::{
    ProjectRollbackPreview, ProjectStatus, ProjectStatusMachineSummary,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const PROJECT_STATUS_WORKER_ID: &str = "project-status-worker";
pub const PROJECT_STATUS_WORKER_CONTRACT_VERSION: &str = "project-status-read-v1";
pub const PROJECT_STATUS_MACHINE_OPERATION: &str = "project.status.machine";
pub const PROJECT_STATUS_PREVIEW_ROLLBACK_OPERATION: &str = "project.status.preview_rollback";
pub const PROJECT_STATUS_READ_CAPABILITY: &str = "project.status.read";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectStatusProjectInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectStatusRollbackInput {
    pub project_id: i64,
    pub target: ProjectStatus,
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn command_scope_is_stable() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: PROJECT_STATUS_MACHINE_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(1),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: PROJECT_STATUS_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ProjectStatusProjectInput { project_id: 1 },
        };
        request
            .validate()
            .expect("project status request validates");
    }
}
