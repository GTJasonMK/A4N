//! Typed project permission queries owned by the project-access worker.

pub use a4n_api_contract::collaboration::{ProjectAccessRole, ProjectCollaborationRootSummary};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const PROJECT_ACCESS_WORKER_ID: &str = "project-access-worker";
pub const PROJECT_ACCESS_WORKER_CONTRACT_VERSION: &str = "project-access-v1";
pub const PROJECT_ACCESS_VERIFY_OPERATION: &str = "project.access.verify";
pub const PROJECT_ACCESS_VERIFY_BATCH_OPERATION: &str = "project.access.verify_batch";
pub const PROJECT_ACCESS_FILTER_OPERATION: &str = "project.access.filter";
pub const PROJECT_ACCESS_ROOT_OPERATION: &str = "project.access.root";
pub const PROJECT_ACCESS_CAPABILITY: &str = "project.access.read";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectAccessInput {
    pub project_id: i64,
    pub required: ProjectAccessRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectAccessBatchInput {
    pub project_ids: Vec<i64>,
    pub required: ProjectAccessRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectAccessRootInput {
    pub project_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn user_permission_scope_is_stable() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: PROJECT_ACCESS_VERIFY_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(9),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: PROJECT_ACCESS_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ProjectAccessInput {
                project_id: 9,
                required: ProjectAccessRole::Viewer,
            },
        };
        request
            .validate()
            .expect("project access request validates");
    }
}
