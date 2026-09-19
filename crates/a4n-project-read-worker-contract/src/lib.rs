//! Typed read commands owned by the project read worker.

pub use a4n_api_contract::project::{PaginatedProjects, ProjectListInput, ProjectSummary};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const PROJECT_READ_WORKER_ID: &str = "project-read-worker";
pub const PROJECT_READ_WORKER_CONTRACT_VERSION: &str = "project-read-v1";
pub const PROJECT_READ_SUMMARY_OPERATION: &str = "project.read.summary";
pub const PROJECT_READ_LIST_OPERATION: &str = "project.read.list";
pub const PROJECT_READ_IDS_OPERATION: &str = "project.read.ids";
pub const PROJECT_READ_EXISTS_OPERATION: &str = "project.read.exists";
pub const PROJECT_READ_CAPABILITY: &str = "project.read";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectIdInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectServiceInput {}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn user_summary_scope_is_stable() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: PROJECT_READ_SUMMARY_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(9),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: PROJECT_READ_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ProjectIdInput { project_id: 9 },
        };
        request.validate().expect("project read request validates");
    }
}
