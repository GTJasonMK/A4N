//! Typed commands owned by the scene profile read worker.

pub use a4n_api_contract::profile::SceneProfileSummary;
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const SCENE_PROFILE_WORKER_ID: &str = "scene-profile-worker";
pub const SCENE_PROFILE_WORKER_CONTRACT_VERSION: &str = "scene-profile-read-v1";
pub const SCENE_PROFILE_LIST_OPERATION: &str = "profile.scene.list";
pub const SCENE_PROFILE_READ_CAPABILITY: &str = "profile.scene.read";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SceneProfileProjectInput {
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
            operation_id: SCENE_PROFILE_LIST_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(1),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: SCENE_PROFILE_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: SceneProfileProjectInput { project_id: 1 },
        };
        request
            .validate()
            .expect("scene profile request should validate");
    }
}
