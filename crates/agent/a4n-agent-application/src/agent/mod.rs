//! Durable control plane for the shared main/sub-agent runtime.
//!
//! This application owns user authorization, command discovery and API
//! projections. Model/provider execution stays outside this module so the
//! runtime can drive queued runs without introducing a second workflow owner.

mod workflow;

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
    child_budget_for_delegation, enforce_delegation_limits, parse_agent_context_summary,
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
    AgentHostCommandPayload, AgentWorkerDelegationReviewRequest,
    AgentWorkerDelegationReviewSnapshotWire, AgentWorkerDelegationStartRequest,
    AgentWorkerDelegationStartSnapshotWire, AgentWorkerDelegationThreadRequest,
    AgentWorkerDelegationThreadSnapshotWire, AgentWorkerEventRequest,
    AgentWorkerExecutionResultRequest, AgentWorkerHistoryRequest, AgentWorkerHistorySnapshotWire,
    AgentWorkerInboxMessageRequest, AgentWorkerMemoryRequest, AgentWorkerMessageOperation,
    AgentWorkerMessageSnapshotWire, AgentWorkerResultRefsRequest,
    AgentWorkerResultRefsSnapshotWire, AgentWorkerRunLeaseRequest, WorkerEnvelope, WorkerLease,
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

mod external_task_wait;
mod generation;

mod skills;

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
