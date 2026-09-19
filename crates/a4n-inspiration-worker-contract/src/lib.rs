//! Typed read commands owned by the inspiration history worker.

pub use a4n_api_contract::inspiration::{InspirationHistoryInput, InspirationMessageSummary};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

pub const INSPIRATION_HISTORY_WORKER_ID: &str = "inspiration-history-worker";
pub const INSPIRATION_HISTORY_WORKER_CONTRACT_VERSION: &str = "inspiration-history-v1";
pub const INSPIRATION_HISTORY_OPERATION: &str = "inspiration.read.history";
pub const INSPIRATION_HISTORY_CAPABILITY: &str = "inspiration.history.read";

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
            operation_id: INSPIRATION_HISTORY_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(9),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: INSPIRATION_HISTORY_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: InspirationHistoryInput { project_id: 9 },
        };
        request
            .validate()
            .expect("inspiration history request validates");
    }
}
