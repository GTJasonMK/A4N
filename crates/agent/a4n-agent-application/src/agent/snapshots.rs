//! Agent durable snapshot projection。

use super::*;

pub(super) fn run_snapshot(row: run::Model) -> AgentRunSnapshot {
    AgentRunSnapshot {
        id: row.id,
        thread_id: row.thread_id,
        turn_id: row.turn_id,
        parent_run_id: row.parent_run_id,
        attempt: row.attempt,
        status: row.status,
        budget_json: row.budget_json,
        usage_json: row.usage_json,
        worker_id: row.worker_id,
        lease_token: row.lease_token,
        lease_expires_at: row.lease_expires_at,
        created_at: row.created_at,
        started_at: row.started_at,
    }
}

pub(super) fn thread_snapshot(row: thread::Model) -> Result<AgentThreadSnapshot, AppError> {
    let context_summary = parse_agent_context_summary(row.context_summary.as_deref())?;
    Ok(AgentThreadSnapshot {
        id: row.id,
        owner_user_id: row.owner_user_id,
        thread_kind: row.thread_kind,
        profile_id: row.profile_id,
        title: row.title,
        project_id: row.project_id,
        resource_type: row.resource_type,
        resource_id: row.resource_id,
        scope_json: row.scope_json,
        default_model_config_id: row.default_model_config_id,
        status: row.status,
        revision: row.revision,
        context_summary,
    })
}

pub(super) fn turn_snapshot(row: turn::Model) -> AgentTurnSnapshot {
    AgentTurnSnapshot {
        id: row.id,
        thread_id: row.thread_id,
        input_message_id: row.input_message_id,
        execution_mode: row.execution_mode,
        goal: row.goal,
        model_override_json: row.model_override_json,
    }
}

pub(super) fn message_snapshot(row: message::Model) -> AgentMessageSnapshot {
    AgentMessageSnapshot {
        id: row.id,
        thread_id: row.thread_id,
        turn_id: row.turn_id,
        run_id: row.run_id,
        role: row.role,
        content: row.content,
        payload_json: row.payload_json,
    }
}

pub(super) fn execution_snapshot(row: execution::Model) -> AgentExecutionSnapshot {
    AgentExecutionSnapshot {
        id: row.id,
        run_id: row.run_id,
        execution_key: row.execution_key,
        execution_kind: row.execution_kind,
        status: row.status,
        tool_name: row.tool_name,
        effect: row.effect,
        replay_policy: row.replay_policy,
        arguments_json: row.arguments_json,
        resource_preconditions_json: row.resource_preconditions_json,
        result_json: row.result_json,
        result_summary: row.result_summary,
        result_refs_json: row.result_refs_json,
        error_message: row.error_message,
    }
}

pub(super) fn delegation_snapshot(row: delegation::Model) -> AgentDelegationSnapshot {
    AgentDelegationSnapshot {
        id: row.id,
        parent_run_id: row.parent_run_id,
        parent_execution_id: row.parent_execution_id,
        child_thread_id: row.child_thread_id,
        child_turn_id: row.child_turn_id,
        child_run_id: row.child_run_id,
        delegation_key: row.delegation_key,
        capability: row.capability,
        objective: row.objective,
        success_criteria_json: row.success_criteria_json,
        scope_json: row.scope_json,
        budget_json: row.budget_json,
        status: row.status,
        result_refs_json: row.result_refs_json,
        result_summary: row.result_summary,
        error_message: row.error_message,
    }
}

pub(super) fn inbox_snapshot(row: inbox::Model) -> AgentInboxSnapshot {
    AgentInboxSnapshot {
        id: row.id,
        thread_id: row.thread_id,
        turn_id: row.turn_id,
        run_id: row.run_id,
        source_kind: row.source_kind,
        source_thread_id: row.source_thread_id,
        inbox_kind: row.inbox_kind,
        delivery_mode: row.delivery_mode,
        content: row.content,
        payload_json: row.payload_json,
        status: row.status,
        claim_token: row.claim_token,
    }
}
