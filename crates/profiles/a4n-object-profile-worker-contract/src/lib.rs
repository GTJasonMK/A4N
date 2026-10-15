//! Typed commands owned by the object-profile worker.

pub use a4n_api_contract::profile::{
    CreateObjectProfileInput, DeleteObjectProfileInput, ObjectProfileDeleteResult,
    ObjectProfileInput, ObjectProfileListItem, ObjectProfileMutationResult, ObjectProfileSummary,
    UpdateObjectProfileInput,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const OBJECT_PROFILE_WORKER_ID: &str = "object-profile-worker";
pub const OBJECT_PROFILE_WORKER_CONTRACT_VERSION: &str = "object-profile-mutation-v2";
pub const OBJECT_PROFILE_LIST_OPERATION: &str = "profile.object.list";
pub const OBJECT_PROFILE_LIST_INDEX_OPERATION: &str = "profile.object.list_index";
pub const OBJECT_PROFILE_CREATE_OPERATION: &str = "profile.object.create";
pub const OBJECT_PROFILE_UPDATE_OPERATION: &str = "profile.object.update";
pub const OBJECT_PROFILE_DELETE_OPERATION: &str = "profile.object.delete";
pub const OBJECT_PROFILE_READ_CAPABILITY: &str = "profile.object.read";
pub const OBJECT_PROFILE_MUTATION_CAPABILITY: &str = "profile.object.mutation";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObjectProfileProjectInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObjectProfileMutationResponse {
    pub result: ObjectProfileMutationResult,
    pub downstream_image_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObjectProfileDeleteResponse {
    pub result: ObjectProfileDeleteResult,
    pub downstream_image_ids: Vec<i64>,
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
            operation_id: OBJECT_PROFILE_LIST_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(1),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: OBJECT_PROFILE_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ObjectProfileProjectInput { project_id: 1 },
        };
        request
            .validate()
            .expect("object profile request should validate");

        let mut list_index = request;
        list_index.operation_id = OBJECT_PROFILE_LIST_INDEX_OPERATION.into();
        list_index.idempotency_key = "list-index-key".into();
        list_index
            .validate()
            .expect("object profile index request should validate");
    }
}
