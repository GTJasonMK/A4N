//! Typed read commands owned by the theme worker.

pub use a4n_api_contract::theme::{ThemePreferenceSummary, ThemeResourceRecord, ThemeSummary};
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const THEME_READ_WORKER_ID: &str = "theme-read-worker";
pub const THEME_READ_WORKER_CONTRACT_VERSION: &str = "theme-read-v1";
pub const THEME_LIST_OPERATION: &str = "theme.read.list";
pub const THEME_EXPORT_OPERATION: &str = "theme.read.export";
pub const THEME_RESOURCES_OPERATION: &str = "theme.read.resources";
pub const THEME_PREFERENCE_OPERATION: &str = "theme.read.preference";
pub const THEME_READ_CAPABILITY: &str = "theme.read";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ThemeUserInput {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ThemeIdInput {
    pub theme_id: i64,
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
            operation_id: THEME_LIST_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: THEME_READ_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ThemeUserInput {},
        };
        request.validate().expect("theme request validates");
    }
}
