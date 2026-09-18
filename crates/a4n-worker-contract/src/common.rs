use std::collections::HashSet;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};

use crate::WORKER_SCHEMA_VERSION;

pub const WORKER_HEALTH_PATH: &str = "/healthz";
pub const WORKER_CAPABILITIES_PATH: &str = "/capabilities";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkerContractError {
    #[error("unsupported worker schema version: expected {expected}, got {actual}")]
    SchemaVersion { expected: u32, actual: u32 },
    #[error("{field} must not be blank")]
    BlankField { field: &'static str },
    #[error("{field} {reason}")]
    InvalidValue {
        field: &'static str,
        reason: &'static str,
    },
    #[error("{0}")]
    InvalidRelation(&'static str),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerEnvelope<T> {
    pub schema_version: u32,
    pub request_id: String,
    pub trace_id: String,
    pub principal: WorkerPrincipal,
    pub operation_id: String,
    pub idempotency_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_project_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_revision: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_config_snapshot_ref: Option<String>,
    pub lease: WorkerLease,
    pub payload: T,
}

/// Compatibility name for the Agent worker wire envelope.
///
/// This alias intentionally keeps the existing JSON shape and field names.
pub type WorkerRequest<T> = WorkerEnvelope<T>;

impl<T> WorkerEnvelope<T> {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_worker_schema_version(self.schema_version)?;
        validate_non_blank("request_id", &self.request_id)?;
        validate_non_blank("trace_id", &self.trace_id)?;
        validate_non_blank("operation_id", &self.operation_id)?;
        validate_non_blank("idempotency_key", &self.idempotency_key)?;
        self.principal.validate()?;
        self.lease.validate()?;
        validate_positive_optional_id("project_id", self.project_id)?;
        validate_positive_optional_id("root_project_id", self.root_project_id)?;
        if self.root_project_id.is_some() && self.project_id.is_none() {
            return Err(WorkerContractError::InvalidRelation(
                "root_project_id requires project_id",
            ));
        }
        if self.target_revision == Some(0) {
            return Err(WorkerContractError::InvalidValue {
                field: "target_revision",
                reason: "must be positive when present",
            });
        }
        if let Some(snapshot_ref) = &self.runtime_config_snapshot_ref {
            validate_non_blank("runtime_config_snapshot_ref", snapshot_ref)?;
        }
        Ok(())
    }

    pub fn scope(&self) -> WorkerScope {
        WorkerScope {
            project_id: self.project_id,
            root_project_id: self.root_project_id,
            resource_type: None,
            resource_id: None,
        }
    }

    pub fn operation_ref(&self) -> WorkerOperationRef {
        WorkerOperationRef {
            operation_id: self.operation_id.clone(),
            task_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerPrincipal {
    pub kind: WorkerPrincipalKind,
    pub subject: String,
}

impl WorkerPrincipal {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_non_blank("principal.subject", &self.subject)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerPrincipalKind {
    User,
    Service,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerScope {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_project_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
}

impl Default for WorkerScope {
    fn default() -> Self {
        Self {
            project_id: None,
            root_project_id: None,
            resource_type: None,
            resource_id: None,
        }
    }
}

impl WorkerScope {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_positive_optional_id("scope.project_id", self.project_id)?;
        validate_positive_optional_id("scope.root_project_id", self.root_project_id)?;
        if self.root_project_id.is_some() && self.project_id.is_none() {
            return Err(WorkerContractError::InvalidRelation(
                "scope.root_project_id requires scope.project_id",
            ));
        }
        if self.resource_type.is_some() != self.resource_id.is_some() {
            return Err(WorkerContractError::InvalidRelation(
                "scope.resource_type and scope.resource_id must be provided together",
            ));
        }
        if let Some(resource_type) = &self.resource_type {
            validate_non_blank("scope.resource_type", resource_type)?;
        }
        if let Some(resource_id) = &self.resource_id {
            validate_non_blank("scope.resource_id", resource_id)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerLease {
    pub worker_id: String,
    pub epoch: u64,
    pub fence_token: String,
    pub expires_at: String,
}

impl WorkerLease {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_non_blank("lease.worker_id", &self.worker_id)?;
        if self.epoch == 0 {
            return Err(WorkerContractError::InvalidValue {
                field: "lease.epoch",
                reason: "must be positive",
            });
        }
        validate_non_blank("lease.fence_token", &self.fence_token)?;
        validate_non_blank("lease.expires_at", &self.expires_at)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerOperationStatus {
    Accepted,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerOperationRef {
    pub operation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

impl WorkerOperationRef {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_non_blank("operation.operation_id", &self.operation_id)?;
        if let Some(task_id) = &self.task_id {
            validate_non_blank("operation.task_id", task_id)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerCancelRequest {
    pub operation: WorkerOperationRef,
    pub reason: String,
}

impl WorkerCancelRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        self.operation.validate()?;
        validate_non_blank("reason", &self.reason)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerRecoveryRequest {
    pub operation: WorkerOperationRef,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_ref: Option<String>,
}

impl WorkerRecoveryRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        self.operation.validate()?;
        validate_non_blank("reason", &self.reason)?;
        if let Some(checkpoint_ref) = &self.checkpoint_ref {
            validate_non_blank("checkpoint_ref", checkpoint_ref)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerRevision(u64);

impl WorkerRevision {
    pub fn new(value: u64) -> Result<Self, WorkerContractError> {
        if value == 0 {
            return Err(WorkerContractError::InvalidValue {
                field: "revision",
                reason: "must be positive",
            });
        }
        Ok(Self(value))
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

impl Serialize for WorkerRevision {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.0)
    }
}

impl<'de> Deserialize<'de> for WorkerRevision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(u64::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerResourceRef {
    pub kind: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<WorkerRevision>,
}

impl WorkerResourceRef {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_non_blank("resource_ref.kind", &self.kind)?;
        validate_non_blank("resource_ref.id", &self.id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerPagination {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

impl WorkerPagination {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        if let Some(next_cursor) = &self.next_cursor {
            validate_non_blank("pagination.next_cursor", next_cursor)?;
        }
        if self.has_more != self.next_cursor.is_some() {
            return Err(WorkerContractError::InvalidRelation(
                "pagination.has_more must match pagination.next_cursor",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerErrorCode {
    InvalidRequest,
    Unauthorized,
    Conflict,
    LeaseLost,
    NotFound,
    Unavailable,
    Timeout,
    Cancelled,
    VersionMismatch,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerError {
    pub code: WorkerErrorCode,
    pub message: String,
    pub retryable: bool,
}

impl WorkerError {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_non_blank("error.message", &self.message)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerResponse<T> {
    pub schema_version: u32,
    pub request_id: String,
    pub trace_id: String,
    pub operation: WorkerOperationRef,
    pub status: WorkerOperationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<T>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_refs: Vec<WorkerResourceRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub committed_revision: Option<WorkerRevision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<WorkerPagination>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<WorkerError>,
}

impl<T> WorkerResponse<T> {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_worker_schema_version(self.schema_version)?;
        validate_non_blank("request_id", &self.request_id)?;
        validate_non_blank("trace_id", &self.trace_id)?;
        self.operation.validate()?;
        for resource in &self.resource_refs {
            resource.validate()?;
        }
        if let Some(provider_job_id) = &self.provider_job_id {
            validate_non_blank("provider_job_id", provider_job_id)?;
        }
        if let Some(result_summary) = &self.result_summary {
            validate_non_blank("result_summary", result_summary)?;
        }
        if let Some(pagination) = &self.pagination {
            pagination.validate()?;
        }
        if let Some(error) = &self.error {
            error.validate()?;
        }
        match self.status {
            WorkerOperationStatus::Failed if self.error.is_none() => Err(
                WorkerContractError::InvalidRelation("failed response requires error"),
            ),
            WorkerOperationStatus::Accepted
            | WorkerOperationStatus::Running
            | WorkerOperationStatus::Completed
                if self.error.is_some() =>
            {
                Err(WorkerContractError::InvalidRelation(
                    "only failed response may include error",
                ))
            }
            WorkerOperationStatus::Accepted
            | WorkerOperationStatus::Running
            | WorkerOperationStatus::Cancelled
                if self.payload.is_some() =>
            {
                Err(WorkerContractError::InvalidRelation(
                    "non-completed response cannot include payload",
                ))
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerHealth {
    pub schema_version: u32,
    pub worker_id: String,
    pub status: WorkerHealthStatus,
    pub version: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerHealthStatus {
    Healthy,
    Degraded,
    Unavailable,
}

impl WorkerHealth {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_worker_schema_version(self.schema_version)?;
        validate_non_blank("worker_id", &self.worker_id)?;
        validate_non_blank("version", &self.version)
    }

    pub fn validate_for(&self, expected_worker_id: &str) -> Result<(), WorkerContractError> {
        self.validate()?;
        validate_non_blank("expected_worker_id", expected_worker_id)?;
        if self.worker_id != expected_worker_id {
            return Err(WorkerContractError::InvalidRelation(
                "worker health worker_id does not match expected worker",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerCapability {
    pub name: String,
    pub version: String,
    pub enabled: bool,
}

impl WorkerCapability {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_non_blank("capability.name", &self.name)?;
        validate_non_blank("capability.version", &self.version)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerCapabilities {
    pub schema_version: u32,
    pub worker_id: String,
    #[serde(default)]
    pub capabilities: Vec<WorkerCapability>,
}

impl WorkerCapabilities {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_worker_schema_version(self.schema_version)?;
        validate_non_blank("worker_id", &self.worker_id)?;
        let mut names = HashSet::with_capacity(self.capabilities.len());
        for capability in &self.capabilities {
            capability.validate()?;
            if !names.insert(&capability.name) {
                return Err(WorkerContractError::InvalidRelation(
                    "capabilities must not contain duplicate names",
                ));
            }
        }
        Ok(())
    }

    pub fn validate_for(&self, expected_worker_id: &str) -> Result<(), WorkerContractError> {
        self.validate()?;
        validate_non_blank("expected_worker_id", expected_worker_id)?;
        if self.worker_id != expected_worker_id {
            return Err(WorkerContractError::InvalidRelation(
                "worker capabilities worker_id does not match expected worker",
            ));
        }
        Ok(())
    }
}

pub fn validate_worker_schema_version(actual: u32) -> Result<(), WorkerContractError> {
    if actual != WORKER_SCHEMA_VERSION {
        return Err(WorkerContractError::SchemaVersion {
            expected: WORKER_SCHEMA_VERSION,
            actual,
        });
    }
    Ok(())
}

pub(crate) fn validate_non_blank(
    field: &'static str,
    value: &str,
) -> Result<(), WorkerContractError> {
    if value.trim().is_empty() {
        return Err(WorkerContractError::BlankField { field });
    }
    Ok(())
}

pub(crate) fn validate_positive_optional_id(
    field: &'static str,
    value: Option<i64>,
) -> Result<(), WorkerContractError> {
    if value.is_some_and(|value| value <= 0) {
        return Err(WorkerContractError::InvalidValue {
            field,
            reason: "must be positive when present",
        });
    }
    Ok(())
}

pub(crate) fn validate_positive_id(
    field: &'static str,
    value: i64,
) -> Result<(), WorkerContractError> {
    if value <= 0 {
        return Err(WorkerContractError::InvalidValue {
            field,
            reason: "must be positive",
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde::de::DeserializeOwned;

    use super::*;

    fn valid_envelope() -> WorkerEnvelope<serde_json::Value> {
        WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request-1".into(),
            trace_id: "trace-1".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::Service,
                subject: "control".into(),
            },
            operation_id: "content.update".into(),
            idempotency_key: "idem-1".into(),
            project_id: Some(7),
            root_project_id: Some(7),
            target_revision: Some(3),
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: "content-worker".into(),
                epoch: 2,
                fence_token: "fence-2".into(),
                expires_at: "2030-01-01 00:00:00".into(),
            },
            payload: serde_json::json!({"content": "draft"}),
        }
    }

    fn valid_operation() -> WorkerOperationRef {
        WorkerOperationRef {
            operation_id: "content.update".into(),
            task_id: Some("task-1".into()),
        }
    }

    fn valid_response(status: WorkerOperationStatus) -> WorkerResponse<serde_json::Value> {
        WorkerResponse {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request-1".into(),
            trace_id: "trace-1".into(),
            operation: valid_operation(),
            status,
            payload: (status == WorkerOperationStatus::Completed)
                .then(|| serde_json::json!({"ok": true})),
            resource_refs: vec![WorkerResourceRef {
                kind: "chapter".into(),
                id: "7".into(),
                revision: Some(WorkerRevision::new(3).expect("revision")),
            }],
            committed_revision: Some(WorkerRevision::new(3).expect("revision")),
            provider_job_id: None,
            result_summary: Some("updated".into()),
            pagination: None,
            error: (status == WorkerOperationStatus::Failed).then(|| WorkerError {
                code: WorkerErrorCode::Internal,
                message: "failed".into(),
                retryable: true,
            }),
        }
    }

    fn assert_unknown_field_rejected<T: DeserializeOwned + std::fmt::Debug>(
        mut value: serde_json::Value,
    ) {
        value["future_field"] = serde_json::json!(true);
        let error = serde_json::from_value::<T>(value)
            .expect_err("common worker contract must reject unknown fields");
        assert!(error.to_string().contains("unknown field `future_field`"));
    }

    #[test]
    fn legacy_agent_request_alias_preserves_wire_shape_and_common_metadata() {
        let request: WorkerRequest<_> = valid_envelope();
        request.validate().expect("envelope should validate");
        let encoded = serde_json::to_value(&request).expect("request should serialize");
        assert!(encoded.get("scope").is_none());
        assert_eq!(encoded["idempotency_key"], "idem-1");
        assert_eq!(encoded["target_revision"], 3);
        assert_eq!(request.scope().project_id, Some(7));
        assert_eq!(request.operation_ref().operation_id, "content.update");
        assert_eq!(request.operation_ref().task_id, None);

        let decoded: WorkerRequest<serde_json::Value> =
            serde_json::from_value(encoded).expect("legacy request should deserialize");
        assert_eq!(decoded, request);
    }

    #[test]
    fn envelope_rejects_blank_idempotency_principal_and_invalid_scope_revision() {
        let mut request = valid_envelope();
        request.idempotency_key = " ".into();
        assert!(matches!(
            request.validate(),
            Err(WorkerContractError::BlankField {
                field: "idempotency_key"
            })
        ));

        let mut request = valid_envelope();
        request.principal.subject = " ".into();
        assert!(matches!(
            request.validate(),
            Err(WorkerContractError::BlankField {
                field: "principal.subject"
            })
        ));

        let mut request = valid_envelope();
        request.root_project_id = Some(8);
        request.project_id = None;
        assert_eq!(
            request.validate(),
            Err(WorkerContractError::InvalidRelation(
                "root_project_id requires project_id"
            ))
        );

        let mut request = valid_envelope();
        request.target_revision = Some(0);
        assert!(matches!(
            request.validate(),
            Err(WorkerContractError::InvalidValue {
                field: "target_revision",
                ..
            })
        ));

        let invalid_scope = WorkerScope {
            project_id: Some(7),
            root_project_id: None,
            resource_type: Some("chapter".into()),
            resource_id: None,
        };
        assert!(matches!(
            invalid_scope.validate(),
            Err(WorkerContractError::InvalidRelation(_))
        ));
    }

    #[test]
    fn common_envelopes_reject_unknown_fields() {
        assert_unknown_field_rejected::<WorkerEnvelope<serde_json::Value>>(
            serde_json::to_value(valid_envelope()).expect("envelope should serialize"),
        );
        assert_unknown_field_rejected::<WorkerResponse<serde_json::Value>>(
            serde_json::to_value(valid_response(WorkerOperationStatus::Completed))
                .expect("response should serialize"),
        );
        assert_unknown_field_rejected::<WorkerCancelRequest>(serde_json::json!({
            "operation": {"operation_id": "content.update"},
            "reason": "user requested"
        }));
        assert_unknown_field_rejected::<WorkerRecoveryRequest>(serde_json::json!({
            "operation": {"operation_id": "content.update"},
            "reason": "worker restarted"
        }));
        assert_unknown_field_rejected::<WorkerHealth>(serde_json::json!({
            "schema_version": WORKER_SCHEMA_VERSION,
            "worker_id": "content-worker",
            "status": "healthy",
            "version": "2026.09.17"
        }));
        assert_unknown_field_rejected::<WorkerCapabilities>(serde_json::json!({
            "schema_version": WORKER_SCHEMA_VERSION,
            "worker_id": "content-worker",
            "capabilities": []
        }));
    }

    #[test]
    fn operation_cancel_and_recovery_contracts_validate_identity_and_reason() {
        let cancel = WorkerCancelRequest {
            operation: valid_operation(),
            reason: "user requested".into(),
        };
        cancel.validate().expect("cancel request should validate");
        assert!(matches!(
            WorkerCancelRequest {
                reason: " ".into(),
                ..cancel
            }
            .validate(),
            Err(WorkerContractError::BlankField { field: "reason" })
        ));

        let recovery = WorkerRecoveryRequest {
            operation: valid_operation(),
            reason: "worker restarted".into(),
            checkpoint_ref: Some("checkpoint-1".into()),
        };
        recovery
            .validate()
            .expect("recovery request should validate");
        assert!(matches!(
            WorkerRecoveryRequest {
                checkpoint_ref: Some(" ".into()),
                ..recovery
            }
            .validate(),
            Err(WorkerContractError::BlankField {
                field: "checkpoint_ref"
            })
        ));
    }

    #[test]
    fn response_requires_status_consistent_error_payload_and_pagination() {
        valid_response(WorkerOperationStatus::Accepted)
            .validate()
            .expect("accepted response should validate");
        valid_response(WorkerOperationStatus::Completed)
            .validate()
            .expect("completed response should validate");
        valid_response(WorkerOperationStatus::Failed)
            .validate()
            .expect("failed response should validate");

        let mut failed = valid_response(WorkerOperationStatus::Failed);
        failed.error = None;
        assert_eq!(
            failed.validate(),
            Err(WorkerContractError::InvalidRelation(
                "failed response requires error"
            ))
        );

        let mut running = valid_response(WorkerOperationStatus::Running);
        running.payload = Some(serde_json::json!({"progress": 1}));
        assert_eq!(
            running.validate(),
            Err(WorkerContractError::InvalidRelation(
                "non-completed response cannot include payload"
            ))
        );

        let mut response = valid_response(WorkerOperationStatus::Completed);
        response.pagination = Some(WorkerPagination {
            next_cursor: Some("cursor-2".into()),
            has_more: true,
        });
        response.validate().expect("paged response should validate");
        response.pagination = Some(WorkerPagination {
            next_cursor: None,
            has_more: true,
        });
        assert!(matches!(
            response.validate(),
            Err(WorkerContractError::InvalidRelation(_))
        ));
    }

    #[test]
    fn revision_and_response_round_trip_with_zero_rejected() {
        let revision = WorkerRevision::new(4).expect("positive revision should validate");
        let encoded = serde_json::to_string(&revision).expect("revision should serialize");
        assert_eq!(encoded, "4");
        let decoded: WorkerRevision = serde_json::from_str(&encoded).expect("revision");
        assert_eq!(decoded, revision);
        assert!(serde_json::from_str::<WorkerRevision>("0").is_err());

        let response = valid_response(WorkerOperationStatus::Completed);
        let encoded = serde_json::to_string(&response).expect("response should serialize");
        let decoded: WorkerResponse<serde_json::Value> =
            serde_json::from_str(&encoded).expect("response should deserialize");
        assert_eq!(decoded, response);
    }

    #[test]
    fn health_and_capabilities_validate_schema_identity_and_duplicates() {
        let health = WorkerHealth {
            schema_version: WORKER_SCHEMA_VERSION,
            worker_id: "content-worker".into(),
            status: WorkerHealthStatus::Healthy,
            version: "2026.09.17".into(),
        };
        health
            .validate_for("content-worker")
            .expect("health should validate");
        assert!(matches!(
            health.validate_for("other-worker"),
            Err(WorkerContractError::InvalidRelation(_))
        ));

        let capabilities = WorkerCapabilities {
            schema_version: WORKER_SCHEMA_VERSION,
            worker_id: "content-worker".into(),
            capabilities: vec![WorkerCapability {
                name: "content.read".into(),
                version: "v1".into(),
                enabled: true,
            }],
        };
        capabilities
            .validate_for("content-worker")
            .expect("capabilities should validate");

        let duplicate = WorkerCapabilities {
            capabilities: vec![
                WorkerCapability {
                    name: "content.read".into(),
                    version: "v1".into(),
                    enabled: true,
                },
                WorkerCapability {
                    name: "content.read".into(),
                    version: "v2".into(),
                    enabled: true,
                },
            ],
            ..capabilities
        };
        assert!(matches!(
            duplicate.validate(),
            Err(WorkerContractError::InvalidRelation(_))
        ));
    }
}
