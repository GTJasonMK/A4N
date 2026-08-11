use super::context::ServiceRequestContext;
use super::secrets::SecretForwardingPolicy;
use super::trace::{SchemaVersion, TraceContext};
use crate::{ServiceError, ServiceResponse, validate_known_service_request};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceRequest<T> {
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
    pub capability_version: String,
    pub payload: T,
}

impl<T> ServiceRequest<T> {
    pub fn validate_known_service(&self, service_name: &str) -> Result<(), ServiceError> {
        validate_known_service_request(self, service_name)
    }

    pub fn ok_response<U>(&self, payload: U) -> ServiceResponse<U> {
        ServiceResponse::ok(self.trace_context.trace_id.clone(), payload)
    }

    pub fn error_response<U>(&self, error: ServiceError) -> ServiceResponse<U> {
        ServiceResponse::error(self.trace_context.trace_id.clone(), error)
    }

    pub fn result_response<U, E>(
        &self,
        result: Result<U, E>,
        map_error: impl FnOnce(E) -> ServiceError,
    ) -> ServiceResponse<U> {
        ServiceResponse::from_result(self.trace_context.trace_id.clone(), result, map_error)
    }

    pub fn execute_known_service<U, E>(
        self,
        service_name: &str,
        operation: impl FnOnce(T) -> Result<U, E>,
        map_error: impl FnOnce(E) -> ServiceError,
    ) -> ServiceResponse<U> {
        if let Err(error) = self.validate_known_service(service_name) {
            return self.error_response(error);
        }
        let trace_id = self.trace_context.trace_id.clone();
        ServiceResponse::from_result(trace_id, operation(self.payload), map_error)
    }

    pub fn context(&self) -> ServiceRequestContext {
        ServiceRequestContext {
            request_id: self.request_id.clone(),
            trace_context: self.trace_context.clone(),
            schema_version: self.schema_version.clone(),
            user_id: self.user_id,
            project_id: self.project_id,
            root_project_id: self.root_project_id,
            runtime_config_snapshot: self.runtime_config_snapshot.clone(),
            secret_policy: self.secret_policy.clone(),
        }
    }

    pub fn into_context_and_payload(self) -> (ServiceRequestContext, String, T) {
        let Self {
            request_id,
            trace_context,
            schema_version,
            user_id,
            project_id,
            root_project_id,
            runtime_config_snapshot,
            secret_policy,
            capability_version,
            payload,
        } = self;

        (
            ServiceRequestContext {
                request_id,
                trace_context,
                schema_version,
                user_id,
                project_id,
                root_project_id,
                runtime_config_snapshot,
                secret_policy,
            },
            capability_version,
            payload,
        )
    }
}
