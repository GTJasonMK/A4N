use a4n_api_contract::agent::{
    AgentEditorRequest, AgentScope, DescribeAgentToolInput, LoadAgentSkillInput,
    SearchAgentSkillsInput, SearchAgentToolsInput,
};
use serde::{Deserialize, Serialize};

use crate::wire::{WorkerContractError, WorkerLease, validate_non_blank, validate_positive_id};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerExecutionResultRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub execution_id: String,
    #[serde(default)]
    pub pointer: String,
    #[serde(default)]
    pub offset: u32,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerMemoryRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub query: Option<String>,
    pub limit: Option<u32>,
}

impl AgentWorkerMemoryRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        if self.limit == Some(0) {
            return Err(WorkerContractError::InvalidValue {
                field: "limit",
                reason: "must be positive",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerToolSearchRequest {
    pub input: SearchAgentToolsInput,
}

impl AgentWorkerToolSearchRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        if self.input.limit == Some(0) {
            return Err(WorkerContractError::InvalidValue {
                field: "input.limit",
                reason: "must be positive",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerToolDescribeRequest {
    pub input: DescribeAgentToolInput,
}

impl AgentWorkerToolDescribeRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_non_blank("input.name", &self.input.name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerSkillSearchRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub input: SearchAgentSkillsInput,
}

impl AgentWorkerSkillSearchRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        if self.input.limit == Some(0) {
            return Err(WorkerContractError::InvalidValue {
                field: "input.limit",
                reason: "must be positive",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerSkillLoadRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub input: LoadAgentSkillInput,
}

impl AgentWorkerSkillLoadRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_non_blank("input.id", &self.input.id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerChapterScopeRequest {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_ids: Vec<i64>,
}

impl AgentWorkerChapterScopeRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_positive_id("user_id", self.user_id)?;
        validate_positive_id("project_id", self.project_id)?;
        for chapter_id in &self.chapter_ids {
            validate_positive_id("chapter_ids[]", *chapter_id)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerHistoryRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    #[serde(default)]
    pub page: Option<a4n_api_contract::agent::ReadAgentHistoryInput>,
}

impl AgentWorkerHistoryRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        if let Some(page) = &self.page
            && (page.limit.is_some_and(|limit| limit == 0 || limit > 100)
                || page
                    .before
                    .as_ref()
                    .is_some_and(|id| id.parse::<i64>().ok().is_none_or(|id| id <= 0)))
        {
            return Err(WorkerContractError::InvalidValue {
                field: "page",
                reason: "invalid history cursor or limit",
            });
        }
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerRecoveryRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub reason: String,
}

impl AgentWorkerRecoveryRequest {
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
pub struct AgentWorkerEditorWaitRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub editor_request: AgentEditorRequest,
}

impl AgentWorkerEditorWaitRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_non_blank(
            "editor_request.execution_id",
            &self.editor_request.execution_id,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerEditorWaitExpiryRequest {
    pub limit: u64,
}

impl AgentWorkerEditorWaitExpiryRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        if self.limit == 0 {
            return Err(WorkerContractError::InvalidValue {
                field: "limit",
                reason: "must be positive",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerApprovalExpiryRequest {
    pub limit: u64,
}

impl AgentWorkerApprovalExpiryRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        if self.limit == 0 {
            return Err(WorkerContractError::InvalidValue {
                field: "limit",
                reason: "must be positive",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerExpiredRunRecoveryRequest {
    pub limit: u64,
    pub error_message: Option<String>,
}

impl AgentWorkerExpiredRunRecoveryRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        if self.limit == 0 {
            return Err(WorkerContractError::InvalidValue {
                field: "limit",
                reason: "must be positive",
            });
        }
        if self
            .error_message
            .as_deref()
            .is_some_and(|message| message.trim().is_empty())
        {
            return Err(WorkerContractError::BlankField {
                field: "error_message",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerGenerationCancelScanRequest {
    pub after_task_id: String,
}

impl AgentWorkerGenerationCancelScanRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerGenerationCancelSnapshotWire {
    pub task_id: String,
    pub attempt: u32,
    pub requested_by_user_id: i64,
    pub status: String,
    pub owner_stopped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerWorkflowCancelScanRequest {
    pub after_workflow_run_id: i64,
}

impl AgentWorkerWorkflowCancelScanRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        if self.after_workflow_run_id < 0 {
            return Err(WorkerContractError::InvalidValue {
                field: "after_workflow_run_id",
                reason: "must not be negative",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerWorkflowCancelSnapshotWire {
    pub workflow_run_id: i64,
    pub owner_stopped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerExternalTaskWaitScanRequest {
    pub after_run_id: i64,
}

impl AgentWorkerExternalTaskWaitScanRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        if self.after_run_id < 0 {
            return Err(WorkerContractError::InvalidValue {
                field: "after_run_id",
                reason: "must not be negative",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerExternalTaskWaitWakeRequest {
    pub run_id: i64,
    pub execution_id: i64,
}

impl AgentWorkerExternalTaskWaitWakeRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_positive_id("run_id", self.run_id)?;
        validate_positive_id("execution_id", self.execution_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerExternalTaskWaitSnapshotWire {
    pub run_id: i64,
    pub owner_user_id: i64,
    pub scope: AgentScope,
    pub execution_id: i64,
    pub tool_name: Option<String>,
    pub arguments_json: Option<serde_json::Value>,
    pub duration_exhausted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerInboxClaimRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub thread_id: i64,
}

impl AgentWorkerInboxClaimRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_positive_id("thread_id", self.thread_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerInboxConsumeRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub inbox_id: i64,
    pub claim_token: String,
}

impl AgentWorkerInboxConsumeRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_positive_id("inbox_id", self.inbox_id)?;
        validate_non_blank("claim_token", &self.claim_token)
    }
}

pub type AgentWorkerInboxPersistRequest = AgentWorkerInboxConsumeRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerInboxMessageRequest {
    pub run_id: i64,
    pub lease: WorkerLease,
    pub message_id: i64,
}

impl AgentWorkerInboxMessageRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_positive_id("message_id", self.message_id)
    }
}

impl AgentWorkerExecutionResultRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        AgentWorkerRunLeaseRequest {
            run_id: self.run_id,
            lease: self.lease.clone(),
        }
        .validate()?;
        validate_non_blank("execution_id", &self.execution_id)?;
        if self.limit == Some(0) {
            return Err(WorkerContractError::InvalidValue {
                field: "limit",
                reason: "must be positive",
            });
        }
        Ok(())
    }
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
