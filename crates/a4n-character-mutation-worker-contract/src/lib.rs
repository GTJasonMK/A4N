//! Typed initial character profile mutation commands.

pub use a4n_api_contract::character::{
    CharacterProfileCreateResult, CharacterProfileMutationResult, CreateProfileInput,
    UpdateCharacterProfileInput,
};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const CHARACTER_MUTATION_WORKER_ID: &str = "character-mutation-worker";
pub const CHARACTER_MUTATION_WORKER_CONTRACT_VERSION: &str = "character-mutation-v1";
pub const CHARACTER_MUTATION_CREATE_OPERATION: &str = "character.mutation.create";
pub const CHARACTER_MUTATION_UPDATE_OPERATION: &str = "character.mutation.update";
pub const CHARACTER_MUTATION_CAPABILITY: &str = "character.mutation";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateInput(pub CreateProfileInput);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateInput(pub UpdateCharacterProfileInput);

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };
    use serde_json::json;

    #[test]
    fn character_mutation_scope_is_stable() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: CHARACTER_MUTATION_CREATE_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(9),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: CHARACTER_MUTATION_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: CreateInput(CreateProfileInput {
                project_id: 9,
                name: "林夏".into(),
                base_attributes: json!({}),
            }),
        };
        request.validate().expect("character mutation validates");
    }
}
