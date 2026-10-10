use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};

use crate::WORKER_SCHEMA_VERSION;

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

    pub fn validate_worker_command(
        &self,
        operation: &str,
        worker_id: &str,
        worker_label: &str,
    ) -> Result<(), WorkerFailure> {
        self.validate()
            .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
        if self.operation_id != operation {
            return Err(WorkerFailure::invalid_request(format!(
                "operation_id 不匹配: expected={operation}, actual={}",
                self.operation_id
            )));
        }
        if self.lease.worker_id != worker_id {
            return Err(WorkerFailure::invalid_request(format!(
                "lease.worker_id 不属于 {worker_label}"
            )));
        }
        Ok(())
    }

    pub fn require_project_scope(&self, payload_project_id: i64) -> Result<i64, WorkerFailure> {
        self.require_project_scope_with_message(
            payload_project_id,
            "worker scope project_id 与 payload 不一致或无效",
        )
    }

    pub fn require_project_scope_with_message(
        &self,
        payload_project_id: i64,
        error_message: &'static str,
    ) -> Result<i64, WorkerFailure> {
        if payload_project_id <= 0 || self.project_id != Some(payload_project_id) {
            return Err(WorkerFailure::invalid_request(error_message));
        }
        Ok(payload_project_id)
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

pub fn validate_optional_project_scope(
    scope_project_id: Option<i64>,
    resource_project_id: i64,
) -> Result<(), WorkerFailure> {
    if resource_project_id <= 0
        || scope_project_id.is_some_and(|scope| scope != resource_project_id)
    {
        return Err(WorkerFailure::invalid_request(
            "worker scope project_id 与资源 project_id 不一致或无效",
        ));
    }
    Ok(())
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

    pub fn require_kind(
        &self,
        kind: WorkerPrincipalKind,
        error_message: &'static str,
    ) -> Result<(), WorkerFailure> {
        if self.kind != kind {
            return Err(WorkerFailure::unauthorized(error_message));
        }
        Ok(())
    }

    pub fn require_user_id(&self) -> Result<i64, WorkerFailure> {
        self.require_kind(WorkerPrincipalKind::User, "仅允许用户 principal")?;
        self.parse_user_id()
    }

    pub fn parse_user_id(&self) -> Result<i64, WorkerFailure> {
        self.subject
            .parse::<i64>()
            .ok()
            .filter(|id| *id > 0)
            .ok_or_else(|| WorkerFailure::unauthorized("用户 principal subject 无效"))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerPrincipalKind {
    User,
    Service,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
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
    pub fn control_request(worker_id: impl Into<String>) -> Self {
        Self {
            worker_id: worker_id.into(),
            epoch: 1,
            fence_token: "control-request".into(),
            expires_at: "9999-12-31T23:59:59Z".into(),
        }
    }

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

#[derive(Debug)]
pub struct WorkerFailure {
    code: WorkerErrorCode,
    message: String,
    retryable: bool,
}

impl WorkerFailure {
    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn require_positive_id(value: i64, field: &str) -> Result<i64, Self> {
        if value <= 0 {
            return Err(Self::invalid_request(format!("{field} 必须大于 0")));
        }
        Ok(value)
    }

    pub fn require_non_blank(value: &str, error_message: impl Into<String>) -> Result<&str, Self> {
        trimmed_non_blank(value).ok_or_else(|| Self::invalid_request(error_message))
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(WorkerErrorCode::InvalidRequest, message, false)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(WorkerErrorCode::Unauthorized, message, false)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(WorkerErrorCode::NotFound, message, false)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(WorkerErrorCode::Conflict, message, false)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(WorkerErrorCode::Unavailable, message, true)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(WorkerErrorCode::Internal, message, false)
    }

    fn new(code: WorkerErrorCode, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retryable,
        }
    }
}

impl From<WorkerFailure> for WorkerError {
    fn from(failure: WorkerFailure) -> Self {
        Self {
            code: failure.code,
            message: failure.message,
            retryable: failure.retryable,
        }
    }
}

impl WorkerError {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_non_blank("error.message", &self.message)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, bound(deserialize = "T: Deserialize<'de>"))]
pub struct WorkerResponse<T> {
    pub schema_version: u32,
    pub request_id: String,
    pub trace_id: String,
    pub operation: WorkerOperationRef,
    pub status: WorkerOperationStatus,
    #[serde(
        default,
        deserialize_with = "deserialize_present_payload",
        skip_serializing_if = "Option::is_none"
    )]
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

fn deserialize_present_payload<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    // 字段缺失与业务结果为 null 不同；后者仍是一次有结果的完成响应。
    T::deserialize(deserializer).map(Some)
}

impl<T> WorkerResponse<T> {
    pub fn completed_for<P>(request: &WorkerEnvelope<P>, payload: T) -> Self {
        Self::completed_for_parts(
            request.request_id.clone(),
            request.trace_id.clone(),
            request.operation_id.clone(),
            payload,
        )
    }

    pub fn failed_for<P>(request: &WorkerEnvelope<P>, error: WorkerError) -> Self {
        Self::failed_for_parts(
            request.request_id.clone(),
            request.trace_id.clone(),
            request.operation_id.clone(),
            error,
        )
    }

    pub fn completed_for_parts(
        request_id: String,
        trace_id: String,
        operation_id: String,
        payload: T,
    ) -> Self {
        Self {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id,
            trace_id,
            operation: WorkerOperationRef {
                operation_id,
                task_id: None,
            },
            status: WorkerOperationStatus::Completed,
            payload: Some(payload),
            resource_refs: Vec::new(),
            committed_revision: None,
            provider_job_id: None,
            result_summary: None,
            pagination: None,
            error: None,
        }
    }

    pub fn failed_for_parts(
        request_id: String,
        trace_id: String,
        operation_id: String,
        error: WorkerError,
    ) -> Self {
        Self {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id,
            trace_id,
            operation: WorkerOperationRef {
                operation_id,
                task_id: None,
            },
            status: WorkerOperationStatus::Failed,
            payload: None,
            resource_refs: Vec::new(),
            committed_revision: None,
            provider_job_id: None,
            result_summary: None,
            pagination: None,
            error: Some(error),
        }
    }

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
    if trimmed_non_blank(value).is_none() {
        return Err(WorkerContractError::BlankField { field });
    }
    Ok(())
}

fn trimmed_non_blank(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
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
