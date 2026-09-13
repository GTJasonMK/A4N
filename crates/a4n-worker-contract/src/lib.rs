//! Transport-neutral contracts for stateful A4N workers.
//!
//! Capability sidecars use `a4n-service-protocol`. Stateful workers use this
//! smaller contract so task ownership, fencing, and business payloads do not
//! become part of the capability protocol.

use serde::{Deserialize, Serialize};

pub use a4n_api_contract::agent::AgentScope;
use a4n_api_contract::chapter_visual_plan::{ChapterVisualPlanV2, VisualPlanReadView};
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentRunPayload {
    pub run_id: i64,
    pub thread_id: i64,
    pub turn_id: i64,
    pub profile_id: String,
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
    }
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
    fn validate(&self) -> Result<(), WorkerContractError> {
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
}
