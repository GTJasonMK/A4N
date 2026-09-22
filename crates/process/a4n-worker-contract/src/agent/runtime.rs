use a4n_api_contract::agent::AgentScope;
use a4n_api_contract::command_manifest::{RuntimeConfigPolicy, command_runtime_config_policy};
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::AGENT_RUN_OPERATION;
use crate::wire::{
    WorkerContractError, WorkerEnvelope, validate_non_blank, validate_positive_optional_id,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerRunStatusSnapshotWire {
    pub terminal: bool,
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

impl WorkerEnvelope<AgentRunPayload> {
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
pub struct AgentWorkerHistorySnapshotWire {
    pub user_messages: Vec<AgentWorkerMessageSnapshotWire>,
    pub latest_model_input_message_id: Option<i64>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerInboxSnapshotWire {
    pub id: i64,
    pub thread_id: i64,
    pub turn_id: Option<i64>,
    pub run_id: Option<i64>,
    pub source_kind: String,
    pub source_thread_id: Option<i64>,
    pub inbox_kind: String,
    pub delivery_mode: String,
    pub content: String,
    pub payload_json: Option<serde_json::Value>,
    pub status: String,
    pub claim_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerInboxPersistSnapshotWire {
    pub inbox: AgentWorkerInboxSnapshotWire,
    pub message: AgentWorkerMessageSnapshotWire,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentHostCommandPayload {
    pub run_id: i64,
    pub turn_id: i64,
    pub scope: AgentScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<i64>,
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
        if self.turn_id <= 0 {
            return Err(WorkerContractError::InvalidValue {
                field: "turn_id",
                reason: "must be positive",
            });
        }
        validate_positive_optional_id("execution_id", self.execution_id)?;
        validate_non_blank("command", &self.command)?;
        if !self.arguments.is_object() {
            return Err(WorkerContractError::InvalidValue {
                field: "arguments",
                reason: "must be an object",
            });
        }
        Ok(())
    }

    pub fn arguments_with_runtime_snapshot(
        &self,
    ) -> Result<serde_json::Value, WorkerContractError> {
        let mut arguments = self.arguments.clone();
        let fields = arguments
            .as_object_mut()
            .ok_or(WorkerContractError::InvalidValue {
                field: "arguments",
                reason: "must be an object",
            })?;
        fields.remove("runtime_config_snapshot");
        let Some(snapshot) = &self.runtime_config_snapshot else {
            return Ok(arguments);
        };
        let snapshot = match command_runtime_config_policy(&self.command) {
            RuntimeConfigPolicy::None => return Ok(arguments),
            RuntimeConfigPolicy::WithoutSecrets => {
                let snapshot: RuntimeConfigSnapshot = serde_json::from_value(snapshot.clone())
                    .map_err(|_| WorkerContractError::InvalidValue {
                        field: "runtime_config_snapshot",
                        reason: "must be a valid runtime config snapshot",
                    })?;
                serde_json::to_value(snapshot.without_secrets())
                    .expect("runtime config snapshot must serialize")
            }
            RuntimeConfigPolicy::Full => snapshot.clone(),
        };
        fields.insert("runtime_config_snapshot".into(), snapshot);
        Ok(arguments)
    }
}
