//! Typed commands owned by the source snapshot read worker.

pub use a4n_api_contract::project::SourceSnapshotChapterSummary;
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const SOURCE_SNAPSHOT_WORKER_ID: &str = "source-snapshot-worker";
pub const SOURCE_SNAPSHOT_WORKER_CONTRACT_VERSION: &str = "source-snapshot-read-v1";
pub const SOURCE_SNAPSHOT_CHAPTERS_OPERATION: &str = "project.source_snapshot.chapters";
pub const SOURCE_SNAPSHOT_READ_CAPABILITY: &str = "project.source_snapshot.read";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceSnapshotProjectInput {
    pub project_id: i64,
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
            operation_id: SOURCE_SNAPSHOT_CHAPTERS_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(1),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: SOURCE_SNAPSHOT_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: SourceSnapshotProjectInput { project_id: 1 },
        };
        request
            .validate()
            .expect("source snapshot request should validate");
    }
}
