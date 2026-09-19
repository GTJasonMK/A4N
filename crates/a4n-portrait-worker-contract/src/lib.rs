//! Typed commands owned by the portrait record read worker.

pub use a4n_api_contract::portrait::{PortraitGenerationSeed, PortraitSummary};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const PORTRAIT_WORKER_ID: &str = "portrait-worker";
pub const PORTRAIT_WORKER_CONTRACT_VERSION: &str = "portrait-records-v3";
pub const PORTRAIT_LIST_OPERATION: &str = "portrait.records.list";
pub const PORTRAIT_PRIMARY_OPERATION: &str = "portrait.records.primary";
pub const PORTRAIT_BY_ID_OPERATION: &str = "portrait.records.by_id";
pub const PORTRAIT_MISSING_SEEDS_OPERATION: &str = "portrait.records.missing_seeds";
pub const PORTRAIT_SET_PRIMARY_OPERATION: &str = "portrait.records.set_primary";
pub const PORTRAIT_READ_CAPABILITY: &str = "portrait.records.read";
pub const PORTRAIT_MUTATION_CAPABILITY: &str = "portrait.records.mutation";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PortraitProjectInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PortraitProfileInput {
    pub profile_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PortraitRecordInput {
    pub portrait_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PortraitLookupResponse {
    pub portrait: Option<PortraitSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PortraitGenerationSeedWire {
    pub profile_name: String,
    pub prompt: String,
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
            operation_id: PORTRAIT_LIST_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(1),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: PORTRAIT_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: PortraitProjectInput { project_id: 1 },
        };
        request
            .validate()
            .expect("portrait request should validate");
    }

    #[test]
    fn nullable_lookup_result_has_an_unambiguous_payload_shape() {
        let response = PortraitLookupResponse { portrait: None };
        let value = serde_json::to_value(&response).expect("lookup response should serialize");
        assert_eq!(value, serde_json::json!({ "portrait": null }));
        assert_eq!(
            serde_json::from_value::<PortraitLookupResponse>(value)
                .expect("response should deserialize"),
            response
        );
    }
}
