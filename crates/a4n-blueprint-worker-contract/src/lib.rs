//! Typed read commands owned by the Blueprint worker.

pub use a4n_api_contract::blueprint::{
    BlueprintDiffSummary, BlueprintSummary, BlueprintVersionSummary, DiffBlueprintVersionsInput,
    ListBlueprintVersionsInput,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const BLUEPRINT_WORKER_ID: &str = "blueprint-worker";
pub const BLUEPRINT_WORKER_CONTRACT_VERSION: &str = "blueprint-read-v1";
pub const BLUEPRINT_SUMMARY_OPERATION: &str = "blueprint.records.summary";
pub const BLUEPRINT_VERSIONS_OPERATION: &str = "blueprint.records.versions";
pub const BLUEPRINT_DIFF_OPERATION: &str = "blueprint.records.diff";
pub const BLUEPRINT_READ_CAPABILITY: &str = "blueprint.records.read";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BlueprintProjectInput {
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
            operation_id: BLUEPRINT_SUMMARY_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(1),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: BLUEPRINT_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: BlueprintProjectInput { project_id: 1 },
        };
        request
            .validate()
            .expect("blueprint request should validate");
    }
}
