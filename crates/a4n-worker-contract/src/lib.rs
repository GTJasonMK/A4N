//! Transport-neutral contracts for stateful A4N workers.
//!
//! Capability sidecars use `a4n-service-protocol`. Stateful workers use this
//! smaller contract so task ownership, fencing, and business payloads do not
//! become part of the capability protocol.

use serde::{Deserialize, Serialize};

pub use a4n_api_contract::agent::AgentScope;
use a4n_api_contract::chapter_visual_plan::{ChapterVisualPlanV2, VisualPlanReadView};
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;
use chrono::NaiveDateTime;

pub const WORKER_SCHEMA_VERSION: u32 = 1;
pub const AGENT_RUN_OPERATION: &str = "agent.run";
pub const AGENT_HOST_COMMAND_OPERATION: &str = "agent.host_command";
pub const AGENT_WORKER_AUTH_HEADER: &str = "x-a4n-agent-worker-token";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerRequest<T> {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerClaimRequest {
    pub worker_id: String,
}

impl AgentWorkerClaimRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_non_blank("worker_id", &self.worker_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerRunLeaseRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
}

impl AgentWorkerRunLeaseRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        if self.run_id <= 0 {
            return Err(WorkerContractError::InvalidValue {
                field: "run_id",
                reason: "must be positive",
            });
        }
        self.lease.validate()
    }
}

pub type AgentWorkerLeaseRenewRequest = AgentWorkerRunLeaseRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerUsageRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub usage_json: serde_json::Value,
}

impl AgentWorkerUsageRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        if !self.usage_json.is_object() {
            return Err(WorkerContractError::InvalidValue {
                field: "usage_json",
                reason: "must be an object",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerRunWaitRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub reason: String,
}

impl AgentWorkerRunWaitRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_non_blank("reason", &self.reason)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerContextSummaryRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub context_summary: serde_json::Value,
}

impl AgentWorkerContextSummaryRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        if !self.context_summary.is_object() {
            return Err(WorkerContractError::InvalidValue {
                field: "context_summary",
                reason: "must be an object",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerExecutionLookupRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub execution_key: String,
}

impl AgentWorkerExecutionLookupRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_non_blank("execution_key", &self.execution_key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerExecutionCreateRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub execution_key: String,
    pub execution_kind: String,
    pub tool_name: Option<String>,
    pub effect: Option<String>,
    pub replay_policy: Option<String>,
    pub arguments_json: Option<serde_json::Value>,
    pub resource_preconditions_json: Option<serde_json::Value>,
}

impl AgentWorkerExecutionCreateRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_non_blank("execution_key", &self.execution_key)?;
        validate_non_blank("execution_kind", &self.execution_kind)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerExecutionUpdateRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub execution_id: i64,
    pub status: String,
    pub result_json: Option<serde_json::Value>,
    pub result_summary: Option<String>,
    pub result_refs_json: Option<serde_json::Value>,
    pub error_message: Option<String>,
}

impl AgentWorkerExecutionUpdateRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        if self.execution_id <= 0 {
            return Err(WorkerContractError::InvalidValue {
                field: "execution_id",
                reason: "must be positive",
            });
        }
        validate_non_blank("status", &self.status)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentRunPayload {
    pub run_id: i64,
    pub thread_id: i64,
    pub turn_id: i64,
    pub profile_id: String,
    pub worker_run: AgentWorkerRunWire,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_config_snapshot: Option<serde_json::Value>,
}

impl AgentRunPayload {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        for (field, value) in [
            ("run_id", self.run_id),
            ("thread_id", self.thread_id),
            ("turn_id", self.turn_id),
        ] {
            if value <= 0 {
                return Err(WorkerContractError::InvalidValue {
                    field,
                    reason: "must be positive",
                });
            }
        }
        validate_non_blank("profile_id", &self.profile_id)
            .and_then(|_| self.worker_run.validate(self))
    }
}

impl WorkerRequest<AgentRunPayload> {
    pub fn validate_agent_run(&self) -> Result<(), WorkerContractError> {
        self.validate()?;
        self.payload.validate()?;
        let snapshot = &self.payload.worker_run;
        if self.operation_id != AGENT_RUN_OPERATION {
            return Err(WorkerContractError::InvalidRelation(
                "Agent run operation_id 无效",
            ));
        }
        if self.lease.worker_id != snapshot.run.worker_id.as_deref().unwrap_or_default()
            || self.lease.fence_token != snapshot.lease_token
            || self.lease.epoch != u64::try_from(snapshot.run.attempt).unwrap_or_default()
        {
            return Err(WorkerContractError::InvalidRelation(
                "Agent run lease 与 worker 快照不一致",
            ));
        }
        if self.principal.subject != snapshot.thread.owner_user_id.to_string() {
            return Err(WorkerContractError::InvalidRelation(
                "Agent run principal 与 worker 快照不一致",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerRunWire {
    pub run: AgentWorkerRunSnapshotWire,
    pub thread: AgentWorkerThreadSnapshotWire,
    pub lease_token: String,
}

impl AgentWorkerRunWire {
    fn validate(&self, payload: &AgentRunPayload) -> Result<(), WorkerContractError> {
        if self.run.id != payload.run_id
            || self.run.thread_id != payload.thread_id
            || self.run.turn_id != payload.turn_id
            || self.thread.id != payload.thread_id
            || self.thread.profile_id != payload.profile_id
        {
            return Err(WorkerContractError::InvalidRelation(
                "worker_run 与 payload 标识不一致",
            ));
        }
        if self.run.lease_token.as_deref() != Some(self.lease_token.as_str()) {
            return Err(WorkerContractError::InvalidRelation(
                "worker_run lease_token 不一致",
            ));
        }
        if self.run.attempt <= 0 {
            return Err(WorkerContractError::InvalidValue {
                field: "worker_run.run.attempt",
                reason: "must be positive",
            });
        }
        validate_non_blank("worker_run.lease_token", &self.lease_token)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerRunSnapshotWire {
    pub id: i64,
    pub thread_id: i64,
    pub turn_id: i64,
    pub parent_run_id: Option<i64>,
    pub attempt: i32,
    pub status: String,
    pub budget_json: serde_json::Value,
    pub usage_json: serde_json::Value,
    pub worker_id: Option<String>,
    pub lease_token: Option<String>,
    pub lease_expires_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub started_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerThreadSnapshotWire {
    pub id: i64,
    pub owner_user_id: i64,
    pub thread_kind: String,
    pub profile_id: String,
    pub title: Option<String>,
    pub project_id: Option<i64>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub scope_json: serde_json::Value,
    pub default_model_config_id: Option<i64>,
    pub status: String,
    pub revision: i64,
    pub context_summary: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerRunContextWire {
    pub run: AgentWorkerRunSnapshotWire,
    pub thread: AgentWorkerThreadSnapshotWire,
    pub turn: AgentWorkerTurnSnapshotWire,
    pub messages: Vec<AgentWorkerMessageSnapshotWire>,
    pub delegations: Vec<AgentWorkerDelegationSnapshotWire>,
    pub executions: Vec<AgentWorkerExecutionSnapshotWire>,
    pub editor_source: Option<a4n_api_contract::agent::AgentEditorSource>,
    pub owner_user_id: i64,
    pub scope: AgentScope,
    pub profile_id: String,
    pub default_model_config_id: Option<i64>,
    pub execution_mode: String,
    pub goal: Option<String>,
    pub budget: serde_json::Value,
    pub usage: serde_json::Value,
    pub runtime_snapshot_required: bool,
    pub delegation_depth: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerTurnSnapshotWire {
    pub id: i64,
    pub thread_id: i64,
    pub input_message_id: i64,
    pub execution_mode: String,
    pub goal: Option<String>,
    pub model_override_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerMessageSnapshotWire {
    pub id: i64,
    pub thread_id: i64,
    pub turn_id: Option<i64>,
    pub run_id: Option<i64>,
    pub role: String,
    pub content: String,
    pub payload_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerExecutionSnapshotWire {
    pub id: i64,
    pub run_id: i64,
    pub execution_key: String,
    pub execution_kind: String,
    pub status: String,
    pub tool_name: Option<String>,
    pub effect: Option<String>,
    pub replay_policy: Option<String>,
    pub arguments_json: Option<serde_json::Value>,
    pub resource_preconditions_json: Option<serde_json::Value>,
    pub result_json: Option<serde_json::Value>,
    pub result_summary: Option<String>,
    pub result_refs_json: Option<serde_json::Value>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerDelegationSnapshotWire {
    pub id: i64,
    pub parent_run_id: i64,
    pub parent_execution_id: Option<i64>,
    pub child_thread_id: i64,
    pub child_turn_id: Option<i64>,
    pub child_run_id: Option<i64>,
    pub delegation_key: String,
    pub capability: String,
    pub objective: String,
    pub success_criteria_json: serde_json::Value,
    pub scope_json: serde_json::Value,
    pub budget_json: serde_json::Value,
    pub status: String,
    pub result_refs_json: Option<serde_json::Value>,
    pub result_summary: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentHostCommandPayload {
    pub run_id: i64,
    pub command: String,
    pub arguments: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_config_snapshot: Option<serde_json::Value>,
}

impl AgentHostCommandPayload {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        if self.run_id <= 0 {
            return Err(WorkerContractError::InvalidValue {
                field: "run_id",
                reason: "must be positive",
            });
        }
        validate_non_blank("command", &self.command)?;
        if !self.arguments.is_object() {
            return Err(WorkerContractError::InvalidValue {
                field: "arguments",
                reason: "must be an object",
            });
        }
        Ok(())
    }

    pub fn arguments_with_runtime_snapshot(&self) -> serde_json::Value {
        let mut arguments = self.arguments.clone();
        if let Some(snapshot) = &self.runtime_config_snapshot
            && let serde_json::Value::Object(fields) = &mut arguments
        {
            fields.insert("runtime_config_snapshot".into(), snapshot.clone());
        }
        arguments
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerWaitRequest {
    pub run_id: i64,
    pub owner_user_id: i64,
    pub scope: AgentScope,
    pub execution_id: i64,
    pub tool_name: Option<String>,
    pub arguments_json: Option<serde_json::Value>,
    pub duration_exhausted: bool,
    pub runtime_config_snapshot: Option<RuntimeConfigSnapshot>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerEditorProjectionRequest {
    pub plan: ChapterVisualPlanV2,
    pub view: VisualPlanReadView,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerCancelGenerationRequest {
    pub user_id: i64,
    pub task_id: String,
    pub attempt: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerCancelWorkflowRequest {
    pub workflow_run_id: i64,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerLlmCapabilitiesRequest {
    pub user_id: i64,
    pub snapshot: Option<RuntimeConfigSnapshot>,
    pub config_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerChatRequest {
    pub user_id: i64,
    pub snapshot: Option<RuntimeConfigSnapshot>,
    pub config_id: Option<i64>,
    pub request: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerPackageExecutionResponse {
    pub enabled: bool,
}

impl<T> WorkerRequest<T> {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        if self.schema_version != WORKER_SCHEMA_VERSION {
            return Err(WorkerContractError::SchemaVersion {
                expected: WORKER_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerPrincipal {
    pub kind: WorkerPrincipalKind,
    pub subject: String,
}

impl WorkerPrincipal {
    fn validate(&self) -> Result<(), WorkerContractError> {
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

fn validate_non_blank(field: &'static str, value: &str) -> Result<(), WorkerContractError> {
    if value.trim().is_empty() {
        return Err(WorkerContractError::BlankField { field });
    }
    Ok(())
}

fn validate_positive_optional_id(
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

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_request() -> WorkerRequest<serde_json::Value> {
        WorkerRequest {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request-1".into(),
            trace_id: "trace-1".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::Service,
                subject: "control".into(),
            },
            operation_id: "project.generate".into(),
            idempotency_key: "idem-1".into(),
            project_id: Some(7),
            root_project_id: Some(7),
            target_revision: Some(3),
            runtime_config_snapshot_ref: Some("snapshot-1".into()),
            lease: WorkerLease {
                worker_id: "worker-1".into(),
                epoch: 2,
                fence_token: "fence-2".into(),
                expires_at: "2030-01-01T00:00:00Z".into(),
            },
            payload: serde_json::json!({"command": "run"}),
        }
    }

    #[test]
    fn valid_request_round_trips_and_validates() {
        let request = valid_request();
        request.validate().expect("fixture must be valid");
        let encoded = serde_json::to_string(&request).expect("request should serialize");
        let decoded: WorkerRequest<serde_json::Value> =
            serde_json::from_str(&encoded).expect("request should deserialize");
        assert_eq!(decoded, request);
    }

    #[test]
    fn stale_schema_is_rejected_without_echoing_payload() {
        let mut request = valid_request();
        request.schema_version += 1;
        let error = request.validate().expect_err("schema drift must fail");
        assert_eq!(
            error,
            WorkerContractError::SchemaVersion {
                expected: WORKER_SCHEMA_VERSION,
                actual: WORKER_SCHEMA_VERSION + 1,
            }
        );
        assert!(!error.to_string().contains("command"));
    }

    #[test]
    fn lease_and_project_relations_are_checked_at_the_boundary() {
        let mut request = valid_request();
        request.lease.epoch = 0;
        assert!(matches!(
            request.validate(),
            Err(WorkerContractError::InvalidValue {
                field: "lease.epoch",
                ..
            })
        ));

        let mut request = valid_request();
        request.project_id = None;
        assert_eq!(
            request.validate().expect_err("root project needs project"),
            WorkerContractError::InvalidRelation("root_project_id requires project_id")
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut value = serde_json::to_value(valid_request()).expect("request should serialize");
        value["unexpected"] = true.into();
        let error = serde_json::from_value::<WorkerRequest<serde_json::Value>>(value)
            .expect_err("unknown fields must fail");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn host_command_arguments_require_an_object_for_snapshot_injection() {
        let payload = AgentHostCommandPayload {
            run_id: 7,
            command: "save_project".into(),
            arguments: serde_json::json!([]),
            runtime_config_snapshot: None,
        };
        assert!(matches!(
            payload.validate(),
            Err(WorkerContractError::InvalidValue {
                field: "arguments",
                ..
            })
        ));

        let payload = AgentHostCommandPayload {
            arguments: serde_json::json!({"projectId": 7}),
            runtime_config_snapshot: Some(serde_json::json!({"llm": {}})),
            ..payload
        };
        assert_eq!(
            payload.arguments_with_runtime_snapshot()["runtime_config_snapshot"],
            serde_json::json!({"llm": {}})
        );
    }

    #[test]
    fn shared_agent_host_dto_round_trips_and_rejects_unknown_fields() {
        let request = AgentWorkerCancelGenerationRequest {
            user_id: 7,
            task_id: "task-1".into(),
            attempt: 2,
        };
        let encoded = serde_json::to_string(&request).expect("DTO should serialize");
        let decoded: AgentWorkerCancelGenerationRequest =
            serde_json::from_str(&encoded).expect("DTO should deserialize");
        assert_eq!(decoded.user_id, request.user_id);
        assert_eq!(decoded.task_id, request.task_id);
        assert_eq!(decoded.attempt, request.attempt);

        let mut value = serde_json::to_value(request).expect("DTO should serialize");
        value["unexpected"] = true.into();
        assert!(serde_json::from_value::<AgentWorkerCancelGenerationRequest>(value).is_err());
    }

    #[test]
    fn usage_request_requires_an_object_and_round_trips() {
        let request = AgentWorkerUsageRequest {
            run_id: 7,
            lease: valid_request().lease,
            usage_json: serde_json::json!({"tokens_used": 3}),
        };
        request.validate().expect("usage request should be valid");
        let encoded = serde_json::to_string(&request).expect("usage request should serialize");
        let decoded: AgentWorkerUsageRequest =
            serde_json::from_str(&encoded).expect("usage request should deserialize");
        assert_eq!(decoded.run_id, request.run_id);

        let invalid = AgentWorkerUsageRequest {
            usage_json: serde_json::json!([]),
            ..request
        };
        assert!(matches!(
            invalid.validate(),
            Err(WorkerContractError::InvalidValue {
                field: "usage_json",
                ..
            })
        ));
    }

    #[test]
    fn run_wait_request_requires_a_reason() {
        let request = AgentWorkerRunWaitRequest {
            run_id: 7,
            lease: valid_request().lease,
            reason: "external".into(),
        };
        request.validate().expect("wait request should be valid");

        let invalid = AgentWorkerRunWaitRequest {
            reason: "  ".into(),
            ..request
        };
        assert!(matches!(
            invalid.validate(),
            Err(WorkerContractError::BlankField { field: "reason" })
        ));
    }

    #[test]
    fn context_summary_request_requires_an_object() {
        let request = AgentWorkerContextSummaryRequest {
            run_id: 7,
            lease: valid_request().lease,
            context_summary: serde_json::json!({"goal": "draft"}),
        };
        request
            .validate()
            .expect("context summary request should be valid");

        let invalid = AgentWorkerContextSummaryRequest {
            context_summary: serde_json::json!("not-an-object"),
            ..request
        };
        assert!(matches!(
            invalid.validate(),
            Err(WorkerContractError::InvalidValue {
                field: "context_summary",
                ..
            })
        ));
    }

    #[test]
    fn execution_requests_validate_their_lease_and_identifiers() {
        let lease = valid_request().lease;
        let lookup = AgentWorkerExecutionLookupRequest {
            run_id: 7,
            lease: lease.clone(),
            execution_key: "tool-call:1".into(),
        };
        lookup.validate().expect("lookup should be valid");

        let create = AgentWorkerExecutionCreateRequest {
            run_id: 7,
            lease: lease.clone(),
            execution_key: "tool-call:1".into(),
            execution_kind: "tool_call".into(),
            tool_name: Some("read".into()),
            effect: Some("read".into()),
            replay_policy: Some("safe".into()),
            arguments_json: Some(serde_json::json!({})),
            resource_preconditions_json: None,
        };
        create.validate().expect("create should be valid");

        let update = AgentWorkerExecutionUpdateRequest {
            run_id: 7,
            lease,
            execution_id: 2,
            status: "succeeded".into(),
            result_json: None,
            result_summary: None,
            result_refs_json: None,
            error_message: None,
        };
        update.validate().expect("update should be valid");
        assert!(matches!(
            AgentWorkerExecutionUpdateRequest {
                execution_id: 0,
                ..update
            }
            .validate(),
            Err(WorkerContractError::InvalidValue {
                field: "execution_id",
                ..
            })
        ));
    }

    #[test]
    fn run_payload_requires_a_consistent_worker_snapshot() {
        let timestamp = chrono::NaiveDate::from_ymd_opt(2030, 1, 1)
            .expect("date")
            .and_hms_opt(0, 0, 0)
            .expect("time");
        let payload = AgentRunPayload {
            run_id: 7,
            thread_id: 8,
            turn_id: 9,
            profile_id: "general".into(),
            worker_run: AgentWorkerRunWire {
                run: AgentWorkerRunSnapshotWire {
                    id: 7,
                    thread_id: 8,
                    turn_id: 9,
                    parent_run_id: None,
                    attempt: 1,
                    status: "running".into(),
                    budget_json: serde_json::json!({}),
                    usage_json: serde_json::json!({}),
                    worker_id: Some("worker-1".into()),
                    lease_token: Some("fence-1".into()),
                    lease_expires_at: Some(timestamp),
                    created_at: timestamp,
                    started_at: Some(timestamp),
                },
                thread: AgentWorkerThreadSnapshotWire {
                    id: 8,
                    owner_user_id: 11,
                    thread_kind: "main".into(),
                    profile_id: "general".into(),
                    title: None,
                    project_id: Some(12),
                    resource_type: None,
                    resource_id: None,
                    scope_json: serde_json::json!({}),
                    default_model_config_id: None,
                    status: "active".into(),
                    revision: 1,
                    context_summary: None,
                },
                lease_token: "fence-1".into(),
            },
            runtime_config_snapshot: None,
        };
        payload
            .validate()
            .expect("worker snapshot should match payload");

        let encoded = serde_json::to_string(&payload).expect("payload should serialize");
        let decoded: AgentRunPayload =
            serde_json::from_str(&encoded).expect("payload should deserialize");
        assert_eq!(decoded, payload);

        let mut inconsistent = payload;
        inconsistent.worker_run.thread.profile_id = "other".into();
        assert!(matches!(
            inconsistent.validate(),
            Err(WorkerContractError::InvalidRelation(_))
        ));
    }
}
