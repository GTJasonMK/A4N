//! Typed commands owned by the foreshadowing worker.

pub use a4n_api_contract::profile::{
    CreateForeshadowingInput, DeleteForeshadowingInput, ForeshadowingSummary,
    UpdateForeshadowingInput,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const FORESHADOWING_WORKER_ID: &str = "foreshadowing-worker";
pub const FORESHADOWING_WORKER_CONTRACT_VERSION: &str = "foreshadowing-records-v2";
pub const FORESHADOWING_LIST_OPERATION: &str = "profile.foreshadowing.list";
pub const FORESHADOWING_CREATE_OPERATION: &str = "profile.foreshadowing.create";
pub const FORESHADOWING_UPDATE_OPERATION: &str = "profile.foreshadowing.update";
pub const FORESHADOWING_DELETE_OPERATION: &str = "profile.foreshadowing.delete";
pub const FORESHADOWING_READ_CAPABILITY: &str = "profile.foreshadowing.read";
pub const FORESHADOWING_MUTATION_CAPABILITY: &str = "profile.foreshadowing.mutation";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ForeshadowingProjectInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CreateInput(pub CreateForeshadowingInput);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdateInput(pub UpdateForeshadowingInput);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeleteInput(pub DeleteForeshadowingInput);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ForeshadowingMutationResult {
    pub project_id: i64,
    pub foreshadowing_id: i64,
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
            operation_id: FORESHADOWING_LIST_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(1),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: FORESHADOWING_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ForeshadowingProjectInput { project_id: 1 },
        };
        request
            .validate()
            .expect("foreshadowing request should validate");
    }
}
