//! Typed commands owned by the profile-material worker.

pub use a4n_api_contract::profile::{
    ListProfileMaterialsInput, ProfileMaterialGroup, SetProfilePrimaryMaterialInput,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const PROFILE_MATERIAL_WORKER_ID: &str = "profile-material-worker";
pub const PROFILE_MATERIAL_WORKER_CONTRACT_VERSION: &str = "profile-material-v1";
pub const PROFILE_MATERIAL_LIST_OPERATION: &str = "profile.material.list";
pub const PROFILE_MATERIAL_SET_PRIMARY_OPERATION: &str = "profile.material.set_primary";
pub const PROFILE_MATERIAL_READ_CAPABILITY: &str = "profile.material.read";
pub const PROFILE_MATERIAL_MUTATION_CAPABILITY: &str = "profile.material.mutation";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProfileMaterialSetPrimaryResponse {
    pub group: ProfileMaterialGroup,
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::profile_material::{
        ProfileMaterialFilter, ProfileMaterialKindFilter, ProfileMaterialSort,
    };
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
            operation_id: PROFILE_MATERIAL_LIST_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(1),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: PROFILE_MATERIAL_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ListProfileMaterialsInput {
                project_id: 1,
                kind: ProfileMaterialKindFilter::All,
                query: None,
                filter: ProfileMaterialFilter::All,
                sort: ProfileMaterialSort::MissingFirst,
                image_limit_per_profile: 8,
            },
        };
        request
            .validate()
            .expect("profile material request should validate");
    }
}
