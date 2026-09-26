//! worker inbox。

use super::super::{AgentApplication, AgentInboxSnapshot, AgentMessageSnapshot, AgentWorkerRun};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerInboxApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerInboxClaimRequest, AgentWorkerInboxConsumeRequest, AgentWorkerInboxMessageRequest,
    AgentWorkerInboxPersistRequest, AgentWorkerInboxPersistSnapshotWire,
    AgentWorkerInboxSnapshotWire,
};

impl AgentApplication {
    pub(crate) async fn claim_inbox_for_thread(
        &self,
        thread_id: i64,
    ) -> Result<Option<AgentInboxSnapshot>, AppError> {
        agent::claim_inbox(self.connection(), Some(thread_id))
            .await
            .map(|row| row.map(inbox_snapshot))
            .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn consume_claimed_inbox(
        &self,
        row: &AgentInboxSnapshot,
    ) -> Result<AgentInboxSnapshot, AppError> {
        let token = row
            .claim_token
            .as_deref()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("inbox {} 缺少 claim token", row.id),
                operation: "消费 Agent inbox".into(),
            })?;
        agent::consume_inbox(self.connection(), row.id, token)
            .await
            .map(inbox_snapshot)
            .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn persist_claimed_inbox(
        &self,
        claimed: &AgentWorkerRun,
        row: &AgentInboxSnapshot,
    ) -> Result<(AgentInboxSnapshot, AgentMessageSnapshot), AppError> {
        if row.thread_id != claimed.run.thread_id
            || row.turn_id != Some(claimed.run.turn_id)
            || row.run_id != Some(claimed.run.id)
        {
            return Err(AppError::InvalidArgument(
                "Agent inbox 不属于当前 worker run".into(),
            ));
        }
        let token = row
            .claim_token
            .as_deref()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("inbox {} 缺少 claim token", row.id),
                operation: "持久化 Agent inbox".into(),
            })?;
        agent::consume_inbox_with_message(
            self.connection(),
            row.id,
            token,
            &claimed.lease_token,
            AgentMessageInput {
                thread_id: row.thread_id,
                turn_id: row.turn_id,
                run_id: row.run_id,
                role: if row.source_kind == "user" { "user" } else { "assistant" }.into(),
                sender_kind: row.source_kind.clone(),
                sender_thread_id: row.source_thread_id,
                client_message_id: Some(format!("agent-inbox:{}", row.id)),
                content: if row.source_kind == "user" { row.content.clone() } else {
                    serde_json::to_string(&json!({"source_kind": row.source_kind, "inbox_kind": row.inbox_kind, "content": row.content, "payload": row.payload_json}))?
                },
                status: "complete".into(),
                payload_json: Some(json!({
                    "inbox_id": row.id.to_string(),
                    "inbox_kind": row.inbox_kind.clone(),
                    "delivery_mode": row.delivery_mode.clone(),
                    "payload": row.payload_json.clone(),
                })),
            },
        )
        .await
        .map(|(inbox, message)| (inbox_snapshot(inbox), message_snapshot(message)))
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn claim_inbox_request(
        &self,
        request: &AgentWorkerInboxClaimRequest,
    ) -> Result<Option<AgentWorkerInboxSnapshotWire>, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let claimed = self
            .worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
                run_id: request.run_id,
                lease: request.lease.clone(),
            })
            .await?;
        if request.thread_id != claimed.run.thread_id {
            return Err(AppError::InvalidArgument(
                "Agent inbox thread 不属于当前 worker run".into(),
            ));
        }
        self.claim_inbox_for_thread(request.thread_id)
            .await
            .map(|row| row.map(Into::into))
    }

    pub(crate) async fn consume_inbox_request(
        &self,
        request: &AgentWorkerInboxConsumeRequest,
    ) -> Result<AgentWorkerInboxSnapshotWire, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let claimed = self
            .worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
                run_id: request.run_id,
                lease: request.lease.clone(),
            })
            .await?;
        let row = agent::inbox_by_id(self.connection(), request.inbox_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Agent inbox 不存在: {}", request.inbox_id))
            })?;
        if row.thread_id != claimed.run.thread_id
            || row.claim_token.as_deref() != Some(request.claim_token.as_str())
        {
            return Err(AppError::InvalidState {
                current: format!("Agent inbox {} 不属于当前 claim", request.inbox_id),
                operation: "消费 Agent inbox".into(),
            });
        }
        self.consume_claimed_inbox(&inbox_snapshot(row))
            .await
            .map(Into::into)
    }

    pub(crate) async fn persist_inbox_request(
        &self,
        request: &AgentWorkerInboxPersistRequest,
    ) -> Result<AgentWorkerInboxPersistSnapshotWire, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let claimed = self
            .worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
                run_id: request.run_id,
                lease: request.lease.clone(),
            })
            .await?;
        let row = agent::inbox_by_id(self.connection(), request.inbox_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Agent inbox 不存在: {}", request.inbox_id))
            })?;
        if row.claim_token.as_deref() != Some(request.claim_token.as_str()) {
            return Err(AppError::InvalidState {
                current: format!("Agent inbox {} 不属于当前 claim", request.inbox_id),
                operation: "持久化 Agent inbox".into(),
            });
        }
        let (inbox, message) = self
            .persist_claimed_inbox(&claimed, &inbox_snapshot(row))
            .await?;
        Ok(AgentWorkerInboxPersistSnapshotWire {
            inbox: inbox.into(),
            message: message.into(),
        })
    }
}

#[async_trait::async_trait]
impl AgentWorkerInboxApplicationPort for AgentApplication {
    async fn claim_inbox_for_thread(
        &self,
        thread_id: i64,
    ) -> Result<Option<AgentInboxSnapshot>, AppError> {
        AgentApplication::claim_inbox_for_thread(self, thread_id).await
    }

    async fn consume_claimed_inbox(
        &self,
        row: &AgentInboxSnapshot,
    ) -> Result<AgentInboxSnapshot, AppError> {
        AgentApplication::consume_claimed_inbox(self, row).await
    }

    async fn persist_claimed_inbox(
        &self,
        claimed: &AgentWorkerRun,
        row: &AgentInboxSnapshot,
    ) -> Result<(AgentInboxSnapshot, AgentMessageSnapshot), AppError> {
        AgentApplication::persist_claimed_inbox(self, claimed, row).await
    }

    async fn load_message_for_worker(
        &self,
        thread_id: i64,
        message_id: i64,
    ) -> Result<AgentMessageSnapshot, AppError> {
        AgentApplication::load_message_for_worker(self, thread_id, message_id).await
    }

    async fn claim_inbox_request(
        &self,
        request: &AgentWorkerInboxClaimRequest,
    ) -> Result<Option<AgentWorkerInboxSnapshotWire>, AppError> {
        AgentApplication::claim_inbox_request(self, request).await
    }

    async fn consume_inbox_request(
        &self,
        request: &AgentWorkerInboxConsumeRequest,
    ) -> Result<AgentWorkerInboxSnapshotWire, AppError> {
        AgentApplication::consume_inbox_request(self, request).await
    }

    async fn persist_inbox_request(
        &self,
        request: &AgentWorkerInboxPersistRequest,
    ) -> Result<AgentWorkerInboxPersistSnapshotWire, AppError> {
        AgentApplication::persist_inbox_request(self, request).await
    }

    async fn load_message_request(
        &self,
        request: &AgentWorkerInboxMessageRequest,
    ) -> Result<a4n_worker_contract::AgentWorkerMessageSnapshotWire, AppError> {
        AgentApplication::load_message_request(self, request).await
    }
}
