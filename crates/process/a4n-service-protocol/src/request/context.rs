use super::envelope::ServiceRequest;
use super::secrets::SecretForwardingPolicy;
use super::trace::{SchemaVersion, TraceContext};
use crate::response::ServiceError;
use crate::validation::validate_service_request_context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ServiceRequestContext {
    pub request_id: String,
    pub trace_context: TraceContext,
    pub schema_version: SchemaVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_project_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_config_snapshot: Option<serde_json::Value>,
    pub secret_policy: SecretForwardingPolicy,
}

impl ServiceRequestContext {
    pub fn new(
        request_id: impl Into<String>,
        trace_context: TraceContext,
        secret_policy: SecretForwardingPolicy,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            trace_context,
            schema_version: SchemaVersion::protocol_only(),
            user_id: None,
            project_id: None,
            root_project_id: None,
            runtime_config_snapshot: None,
            secret_policy,
        }
    }

    pub fn deny_all(request_id: impl Into<String>, trace_context: TraceContext) -> Self {
        Self::new(
            request_id,
            trace_context,
            SecretForwardingPolicy::deny_all(),
        )
    }

    pub fn local_user_secret(request_id: impl Into<String>, trace_context: TraceContext) -> Self {
        Self::new(
            request_id,
            trace_context,
            SecretForwardingPolicy::local_user_secret(),
        )
    }

    pub fn with_payload<T>(
        self,
        capability_version: impl Into<String>,
        payload: T,
    ) -> Result<ServiceRequest<T>, ServiceError> {
        let request = ServiceRequest {
            request_id: self.request_id,
            trace_context: self.trace_context,
            schema_version: self.schema_version,
            user_id: self.user_id,
            project_id: self.project_id,
            root_project_id: self.root_project_id,
            runtime_config_snapshot: self.runtime_config_snapshot,
            secret_policy: self.secret_policy,
            capability_version: capability_version.into(),
            payload,
        };
        validate_service_request_context(&request)?;
        Ok(request)
    }
}
