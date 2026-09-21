use a4n_api_contract::agent::{
    AgentDelegationView, AgentResourceRef, AgentScope, ReviewAgentDelegationInput,
};
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;
use serde::{Deserialize, Serialize};

use crate::agent_requests::AgentWorkerRunLeaseRequest;
use crate::agent_runtime::{
    AgentWorkerDelegationSnapshotWire, AgentWorkerMessageSnapshotWire, AgentWorkerRunSnapshotWire,
    AgentWorkerThreadSnapshotWire, AgentWorkerTurnSnapshotWire,
};
use crate::wire::{
    WorkerContractError, WorkerLease, validate_non_blank, validate_positive_id,
    validate_positive_optional_id,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentWorkerMessageOperation {
    AppendTool {
        execution_id: i64,
        tool_name: String,
        tool_call_id: Option<String>,
        result: serde_json::Value,
    },
    AppendAssistantTool {
        content: String,
        tool_calls: serde_json::Value,
    },
    UpsertAssistantStreaming {
        client_message_id: String,
        content: String,
        reasoning_content: Option<String>,
    },
    InterruptAssistantStreaming {
        client_message_id: String,
        reason: Option<String>,
    },
    CompleteAssistantStreaming {
        client_message_id: String,
        content: String,
        tool_calls: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerMessageRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub operation: AgentWorkerMessageOperation,
}

impl AgentWorkerMessageRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        match &self.operation {
            AgentWorkerMessageOperation::AppendTool {
                execution_id,
                tool_name,
                ..
            } => {
                if *execution_id <= 0 {
                    return Err(WorkerContractError::InvalidValue {
                        field: "operation.execution_id",
                        reason: "must be positive",
                    });
                }
                validate_non_blank("operation.tool_name", tool_name)
            }
            AgentWorkerMessageOperation::AppendAssistantTool { .. } => Ok(()),
            AgentWorkerMessageOperation::UpsertAssistantStreaming {
                client_message_id, ..
            }
            | AgentWorkerMessageOperation::InterruptAssistantStreaming {
                client_message_id, ..
            }
            | AgentWorkerMessageOperation::CompleteAssistantStreaming {
                client_message_id, ..
            } => validate_non_blank("operation.client_message_id", client_message_id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerBudgetRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
}

impl AgentWorkerBudgetRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerBudgetSnapshotWire {
    pub usage: serde_json::Value,
    pub max_tokens: Option<u64>,
    pub max_duration_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerFailureRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub error_message: String,
}

impl AgentWorkerFailureRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_non_blank("error_message", &self.error_message)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerResultRefsRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
}

impl AgentWorkerResultRefsRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerResultRefsSnapshotWire {
    pub result_refs_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerCompletionRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub content: String,
    pub result_summary: String,
    pub result_refs_json: Option<serde_json::Value>,
    pub observed_input_message_id: i64,
}

impl AgentWorkerCompletionRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_non_blank("content", &self.content)?;
        validate_non_blank("result_summary", &self.result_summary)?;
        if self.observed_input_message_id <= 0 {
            return Err(WorkerContractError::InvalidValue {
                field: "observed_input_message_id",
                reason: "must be positive",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerCompletionSnapshotWire {
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerApprovalCheckRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub execution_id: i64,
}

impl AgentWorkerApprovalCheckRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_positive_id("execution_id", self.execution_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerApprovalCheckSnapshotWire {
    pub approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerApprovalRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub execution_id: i64,
    pub operation_json: serde_json::Value,
    pub resource_preconditions_json: Option<serde_json::Value>,
    pub expires_at: Option<chrono::NaiveDateTime>,
}

impl AgentWorkerApprovalRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_positive_id("execution_id", self.execution_id)?;
        if !self.operation_json.is_object() {
            return Err(WorkerContractError::InvalidValue {
                field: "operation_json",
                reason: "must be an object",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerDelegationReviewRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub user_id: i64,
    pub parent_run_id: Option<i64>,
    pub input: ReviewAgentDelegationInput,
}

impl AgentWorkerDelegationReviewRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_positive_id("user_id", self.user_id)?;
        if let Some(parent_run_id) = self.parent_run_id {
            validate_positive_id("parent_run_id", parent_run_id)?;
        }
        Ok(())
    }
}

pub type AgentWorkerDelegationReviewSnapshotWire = AgentDelegationView;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerDelegationThreadRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub user_id: i64,
    pub canonical_binding_key: String,
    pub capability: String,
    pub title: Option<String>,
    pub scope: AgentScope,
    pub bound_resource: Option<AgentResourceRef>,
    pub default_model_config_id: Option<i64>,
}

impl AgentWorkerDelegationThreadRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_positive_id("user_id", self.user_id)?;
        validate_non_blank("canonical_binding_key", &self.canonical_binding_key)?;
        validate_non_blank("capability", &self.capability)?;
        validate_positive_optional_id("default_model_config_id", self.default_model_config_id)
    }
}

pub type AgentWorkerDelegationThreadSnapshotWire = AgentWorkerThreadSnapshotWire;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerDelegationScopeRequest {
    pub parent_scope: AgentScope,
    pub child_scope: AgentScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerDelegationScopeSnapshotWire {
    pub scope: AgentScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerDelegationStartRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub capability: String,
    pub objective: String,
    pub delegation_key: String,
    pub child_thread_id: i64,
    pub scope: AgentScope,
    pub success_criteria: Vec<String>,
    pub budget_json: serde_json::Value,
    pub parent_execution_id: Option<i64>,
    #[serde(default)]
    pub runtime_config_snapshot: Option<RuntimeConfigSnapshot>,
}

impl AgentWorkerDelegationStartRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_non_blank("capability", &self.capability)?;
        validate_non_blank("objective", &self.objective)?;
        validate_non_blank("delegation_key", &self.delegation_key)?;
        validate_positive_id("child_thread_id", self.child_thread_id)?;
        if !self.budget_json.is_object() {
            return Err(WorkerContractError::InvalidValue {
                field: "budget_json",
                reason: "must be an object",
            });
        }
        validate_positive_optional_id("parent_execution_id", self.parent_execution_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerDelegationStartSnapshotWire {
    pub delegation: AgentWorkerDelegationSnapshotWire,
    pub child_turn: AgentWorkerTurnSnapshotWire,
    pub child_run: AgentWorkerRunSnapshotWire,
    pub child_message: AgentWorkerMessageSnapshotWire,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerEventRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub kind: String,
    pub status: String,
    pub public_text: String,
    pub payload_json: Option<serde_json::Value>,
}

impl AgentWorkerEventRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_non_blank("kind", &self.kind)?;
        validate_non_blank("status", &self.status)?;
        validate_non_blank("public_text", &self.public_text)
    }
}
