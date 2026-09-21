//! Typed commands owned by the project asset graph worker.

pub use a4n_api_contract::project::{AssetDependencyGraph, AssetGraphNodeDetail};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const ASSET_GRAPH_WORKER_ID: &str = "asset-graph-worker";
pub const ASSET_GRAPH_WORKER_CONTRACT_VERSION: &str = "project-asset-graph-v1";
pub const ASSET_GRAPH_OPERATION: &str = "project.asset_graph.read";
pub const ASSET_GRAPH_NODE_DETAIL_OPERATION: &str = "project.asset_graph.node_detail";
pub const ASSET_GRAPH_READ_CAPABILITY: &str = "project.asset_graph.read";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectAssetGraphInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectAssetGraphNodeDetailInput {
    pub project_id: i64,
    pub node_id: String,
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
            operation_id: ASSET_GRAPH_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(1),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: ASSET_GRAPH_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ProjectAssetGraphInput { project_id: 1 },
        };
        request
            .validate()
            .expect("asset graph request should validate");
    }
}
