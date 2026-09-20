//! Durable control plane for the shared main/sub-agent runtime.
//!
//! This application owns user authorization, command discovery and API
//! projections. Model/provider execution stays outside this module so the
//! runtime can drive queued runs without introducing a second workflow owner.

pub use a4n_agent_runtime_contract::{
    AgentApprovalExpiryApplicationPort, AgentContextSummary, AgentDelegationSnapshot,
    AgentEditorClaimApplicationPort, AgentEditorResponseApplicationPort,
    AgentEditorWaitApplicationPort, AgentExecutionSnapshot, AgentExternalTaskWaitApplicationPort,
    AgentExternalTaskWaitSnapshot, AgentGenerationCancelApplicationPort,
    AgentGenerationCancelSnapshot, AgentInboxSnapshot, AgentMessageSnapshot, AgentRunContext,
    AgentRunLeaseRecoveryApplicationPort, AgentRunSnapshot, AgentThreadSnapshot, AgentTurnSnapshot,
    AgentWorkerActivationApplicationPort, AgentWorkerApplicationPort,
    AgentWorkerApprovalApplicationPort, AgentWorkerBudgetApplicationPort,
    AgentWorkerBudgetSnapshot, AgentWorkerClaimApplicationPort,
    AgentWorkerCompletionApplicationPort, AgentWorkerContextApplicationPort,
    AgentWorkerDelegationApplicationPort, AgentWorkerExecutionApplicationPort,
    AgentWorkerFailureApplicationPort, AgentWorkerHistoryApplicationPort,
    AgentWorkerHostCommandApplicationPort, AgentWorkerInboxApplicationPort,
    AgentWorkerLeaseApplicationPort, AgentWorkerMessageApplicationPort,
    AgentWorkerReadApplicationPort, AgentWorkerRecoveryApplicationPort, AgentWorkerRun,
    AgentWorkerUsageApplicationPort, AgentWorkerWaitApplicationPort,
    AgentWorkflowBindingApplicationPort, AgentWorkflowCancelApplicationPort,
    AgentWorkflowCancelSnapshot, EnsureCanonicalSubagentThreadInput, WorkerDelegationInput,
    WorkerDelegationResult, WorkerExecutionInput, WorkerExecutionUpdate, WorkerUsage,
    child_budget_within_parent, enforce_delegation_limits, parse_agent_context_summary,
    validate_agent_budget,
};
use a4n_api_contract::agent::{
    AGENT_API_PROTOCOL_VERSION, AGENT_EVENTS_DEFAULT_LIMIT, AGENT_EVENTS_MAX_LIMIT,
    AGENT_THREADS_DEFAULT_LIMIT, AGENT_THREADS_MAX_LIMIT, AGENT_TOOLS_DEFAULT_LIMIT,
    AGENT_TOOLS_MAX_LIMIT, AgentApprovalDecision, AgentApprovalStatus, AgentApprovalView,
    AgentBudget, AgentControlAction, AgentDelegationStatus, AgentDelegationView, AgentDeliveryMode,
    AgentEvent, AgentEventsInput, AgentExecutionKind, AgentExecutionStatus, AgentExecutionView,
    AgentInboxItemView, AgentInboxKind, AgentInboxStatus, AgentMemoryInput, AgentMemoryScopeKind,
    AgentMemoryStatus, AgentMemoryView, AgentMessageAccepted, AgentMessageRole, AgentMessageStatus,
    AgentMessageView, AgentResourceRef, AgentRunStatus, AgentRunView, AgentScope, AgentThreadKind,
    AgentThreadSnapshot as AgentThreadHistorySnapshot, AgentThreadStatus, AgentThreadView,
    AgentToolDescriptor, AgentToolExposure, AgentToolReplayPolicy, AgentToolSearchResult,
    AgentTurnStatus, AgentTurnTrigger, AgentTurnView, AgentWaitingReason, ArchiveAgentThreadInput,
    ControlAgentRunInput, ControlChildAgentInput, CreateAgentThreadInput,
    EnsureMainAgentThreadInput, ForkAgentThreadInput, GENERAL_AGENT_CAPABILITY,
    ListAgentMessagesInput, ListAgentThreadsInput, MAIN_AGENT_CAPABILITY,
    ResolveAgentApprovalInput, RetractAgentMemoryInput, SearchAgentToolsInput,
    SendAgentMessageInput, SendAgentMessageToChildInput, SpawnAgentInput, UpdateAgentThreadInput,
};
use a4n_db::DbHandle;
use a4n_db::agent::{self, AgentDbError, AgentUserDelegationInput};
use a4n_db::collaboration::CollaborationDbError;
use a4n_db_models::models::agent::{
    approval, delegation, event, execution, inbox, memory, message, run, thread, turn,
};
use a4n_domain::agent::{
    AgentDelegationInput, AgentEventInput, AgentExecutionInput, AgentInboxInput, AgentMessageInput,
    AgentThreadCreateInput, AgentTurnInput,
};
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentHostCommandPayload, AgentWorkerApprovalCheckRequest, AgentWorkerApprovalCheckSnapshotWire,
    AgentWorkerApprovalRequest, AgentWorkerBudgetRequest, AgentWorkerCompletionRequest,
    AgentWorkerCompletionSnapshotWire, AgentWorkerContextSummaryRequest,
    AgentWorkerDelegationReviewRequest, AgentWorkerDelegationReviewSnapshotWire,
    AgentWorkerDelegationStartRequest, AgentWorkerDelegationStartSnapshotWire,
    AgentWorkerDelegationThreadRequest, AgentWorkerDelegationThreadSnapshotWire,
    AgentWorkerEditorWaitRequest, AgentWorkerEventRequest, AgentWorkerExecutionCreateRequest,
    AgentWorkerExecutionLookupRequest, AgentWorkerExecutionResultRequest,
    AgentWorkerExecutionUpdateRequest, AgentWorkerFailureRequest, AgentWorkerHistoryRequest,
    AgentWorkerHistorySnapshotWire, AgentWorkerInboxClaimRequest, AgentWorkerInboxConsumeRequest,
    AgentWorkerInboxMessageRequest, AgentWorkerInboxPersistRequest,
    AgentWorkerInboxPersistSnapshotWire, AgentWorkerInboxSnapshotWire, AgentWorkerMemoryRequest,
    AgentWorkerMessageOperation, AgentWorkerMessageRequest, AgentWorkerMessageSnapshotWire,
    AgentWorkerRecoveryRequest, AgentWorkerResultRefsRequest, AgentWorkerResultRefsSnapshotWire,
    AgentWorkerRunLeaseRequest, AgentWorkerRunStatusSnapshotWire, AgentWorkerRunWaitRequest,
    AgentWorkerUsageRequest, WorkerLease, WorkerRequest,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::collections::HashSet;

use a4n_db::collaboration::ProjectAccessRole;

const SNAPSHOT_LIMIT: u64 = 1_000;

struct RunLeaseFields<'a> {
    run_id: i64,
    thread_id: i64,
    turn_id: i64,
    lease_token: &'a str,
}

struct ApprovalRunFields<'a> {
    run_id: i64,
    thread_id: i64,
    lease_token: &'a str,
}

struct UserEventFields {
    thread_id: i64,
    turn_id: Option<i64>,
    run_id: Option<i64>,
    message_id: Option<i64>,
}

mod approval_expiry;
mod budget;
mod editor_claim;
mod editor_response;
mod editor_wait;
mod external_task_wait;
mod generation;
mod generation_cancel;
mod lease_recovery;
mod skills;
mod worker_activation;
mod worker_approval;
mod worker_claim;
mod worker_completion;
mod worker_context;
mod worker_delegation;
mod worker_execution;
mod worker_failure;
mod worker_history;
mod worker_inbox;
mod worker_lease;
mod worker_message;
mod worker_read;
mod worker_recovery;
mod worker_usage;
mod worker_wait;
mod workflow_binding;
mod workflow_cancel;
#[derive(Clone)]
pub struct AgentApplication {
    database: DbHandle,
}

mod core;
mod ports;
mod snapshots;
mod validation;
mod worker;

use snapshots::*;
use validation::*;

#[cfg(test)]
mod tests;
