#[cfg(test)]
use a4n_service_protocol::redact_sensitive_value;
use a4n_service_protocol::{SchemaVersion, SecretForwardingPolicy, TraceContext};
use serde_json::Value;

use crate::runtime_config;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SidecarRequestContext {
    pub(super) user_id: Option<i64>,
    pub(super) project_id: Option<i64>,
    pub(super) root_project_id: Option<i64>,
    pub(super) trace_context: Option<TraceContext>,
    pub(super) schema_version: SchemaVersion,
    pub(super) runtime_config_snapshot: Option<Value>,
    pub(super) secret_policy: SecretForwardingPolicy,
}

impl SidecarRequestContext {
    pub(crate) fn stateless() -> Self {
        Self {
            user_id: None,
            project_id: None,
            root_project_id: None,
            trace_context: None,
            schema_version: SchemaVersion::protocol_only(),
            runtime_config_snapshot: None,
            secret_policy: SecretForwardingPolicy::deny_all(),
        }
    }

    pub(crate) fn current_global() -> Self {
        Self {
            user_id: runtime_config::current_user_id(),
            runtime_config_snapshot: runtime_config::redacted_current_snapshot_value(),
            ..Self::stateless()
        }
    }

    #[cfg(test)]
    pub(crate) fn with_user_id(mut self, user_id: Option<i64>) -> Self {
        self.user_id = user_id;
        self
    }

    pub(crate) fn with_project_scope(
        mut self,
        project_id: Option<i64>,
        root_project_id: Option<i64>,
    ) -> Self {
        self.project_id = project_id;
        self.root_project_id = root_project_id;
        self
    }

    pub(crate) fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    #[cfg(test)]
    pub(crate) fn with_runtime_config_snapshot(mut self, snapshot: Option<Value>) -> Self {
        self.runtime_config_snapshot = snapshot.map(|mut value| {
            redact_sensitive_value(&mut value);
            value
        });
        self
    }

    pub(crate) fn with_secret_policy(mut self, secret_policy: SecretForwardingPolicy) -> Self {
        self.secret_policy = secret_policy;
        self
    }
}
