#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

for core in \
    crates/config/a4n-config-read-core \
    crates/inspiration/a4n-inspiration-core \
    crates/manga/a4n-manga-core \
    crates/profiles/a4n-profile-material-core \
    crates/project/a4n-project-read-core \
    crates/theme/a4n-theme-core \
    crates/transfer/a4n-import-review-core; do
    if rg -n 'a4n_db_models|a4n-db-models' "$core/Cargo.toml" "$core/src"; then
        printf '%s must not depend on DB Models\n' "$core" >&2
        exit 1
    fi
done

for contract in \
    AgentThreadMutationApplicationPort \
    AgentThreadReadApplicationPort \
    AgentRunMutationApplicationPort \
    AgentRunReadApplicationPort \
    AgentDelegationApplicationPort \
    AgentEventReadApplicationPort \
    AgentToolReadApplicationPort \
    AgentMemoryReadApplicationPort \
    AgentMemoryMutationApplicationPort \
    AgentApprovalApplicationPort \
    AgentResourceAccessApplicationPort; do
    if ! rg -q "pub trait $contract" crates/host/a4n-api-contract/src/agent; then
        printf '%s contract is missing\n' "$contract" >&2
        exit 1
    fi
done

for accessor in \
    agent_thread_mutation_application \
    agent_thread_read_application \
    agent_run_mutation_application \
    agent_run_read_application \
    agent_delegation_application \
    agent_event_read_application \
    agent_tool_read_application \
    agent_memory_read_application \
    agent_memory_mutation_application \
    agent_approval_application \
    agent_resource_access_application; do
    if ! rg -q "$accessor\\(\\)" crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
        printf 'Runtime Agent actions must route through %s()\n' "$accessor" >&2
        exit 1
    fi
done

if rg -n 'AgentApplicationPort|agent_application\(\)' \
    crates/host/a4n-api-contract/src/agent \
    crates/agent/a4n-agent-application/src \
    crates/host/a4n-runtime/src; then
    printf 'The mixed AgentApplicationPort must remain removed\n' >&2
    exit 1
fi

if rg -n -U 'orchestrator\(\)\s*\.agent\(\)' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Runtime Agent worker must consume the injected Agent owner instead of the full OrchestratorContext\n' >&2
    exit 1
fi

if rg -n 'a4n_db_models::models::agent|a4n_db::models::agent' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Runtime Agent worker must consume orchestrator worker snapshots instead of Agent ORM models\n' >&2
    exit 1
fi

if rg -n 'state\.application\(\)\.(search_tools|describe_tool)' \
    crates/agent/a4n-agent-runtime/src/engine; then
    printf 'Agent worker tool discovery must consume AgentWorkerToolReadApplicationPort\n' >&2
    exit 1
fi
if ! rg -n 'search_tools_for_worker|describe_tool_for_worker' \
    crates/agent/a4n-agent-application/src/agent/worker/tool_read.rs \
    crates/agent/a4n-agent-runtime/src/engine; then
    printf 'Agent worker tool discovery read port is missing\n' >&2
    exit 1
fi
if rg -U -n 'ensure_argument_resource_scope\(\s*self\.state\.application\(\)' \
    crates/agent/a4n-agent-runtime/src/engine; then
    printf 'Agent worker resource scope checks must consume AgentResourceAccessApplicationPort\n' >&2
    exit 1
fi
if ! rg -U -q 'agent_resource_access_application\(\)\s*\.verify_chapters_in_project\(\s*request\.user_id,\s*request\.project_id,\s*&request\.chapter_ids,\s*&request\.source_chapter_ids' \
    crates/host/a4n-runtime/src/actions/agent_runtime/history.rs || \
   ! rg -U -q 'self\.application\s*\.verify_chapters_in_project\(\s*request\.user_id,\s*request\.project_id,\s*&request\.chapter_ids,\s*&request\.source_chapter_ids' \
    crates/agent/a4n-agent-worker/src/host.rs; then
    printf 'Agent worker chapter scope must validate chapter and source identities\n' >&2
    exit 1
fi
if rg -n 'AgentWorkerReadApplicationPort|verify_chapters_in_project_for_worker' \
    crates/agent crates/host/a4n-runtime; then
    printf 'The mixed Agent worker read port must remain removed\n' >&2
    exit 1
fi
if ! rg -q 'a4n_api_contract::agent::AgentResourceAccessApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs || \
   ! rg -U -q 'agent_resource_access_application\(\)\s*\.verify_chapters_in_project' \
    crates/host/a4n-runtime/src/actions/agent_runtime/history.rs || \
   ! rg -U -q 'self\.application\s*\.verify_chapters_in_project' \
    crates/agent/a4n-agent-worker/src/host.rs; then
    printf 'Agent worker chapter scope checks must reuse AgentResourceAccessApplicationPort\n' >&2
    exit 1
fi
if rg -n 'verify_chapters_in_project_for_worker' crates/agent crates/host/a4n-runtime; then
    printf 'Duplicate Agent worker chapter-scope port must be removed\n' >&2
    exit 1
fi
if ! rg -q 'pub trait AgentWorkerToolReadApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs || \
   ! rg -q 'impl AgentWorkerToolReadApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/tool_read.rs || \
   ! rg -q 'fn agent_worker_tool_read_application\(' \
    crates/host/a4n-runtime/src/state/app/agent.rs || \
   ! rg -q 'agent_worker_tool_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/agent_runtime; then
    printf 'Agent worker tool reads must use their dedicated application port\n' >&2
    exit 1
fi
if ! rg -q 'pub trait AgentWorkerSkillReadApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs || \
   ! rg -q 'impl AgentWorkerSkillReadApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/skill_read.rs || \
   ! rg -q 'fn agent_worker_skill_read_application\(' \
    crates/host/a4n-runtime/src/state/app/agent.rs || \
   ! rg -q 'agent_worker_skill_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/agent_runtime/commands/reads.rs; then
    printf 'Agent worker skill reads must use their dedicated application port\n' >&2
    exit 1
fi
if ! rg -q 'pub\(crate\) async fn (search_skills_for_worker|load_skill_for_worker)' \
    crates/agent/a4n-agent-application/src/agent/worker/skill_read.rs || \
   rg -n 'search_skills_for_worker|load_skill_for_worker' \
    crates/agent/a4n-agent-application/src/agent/skills.rs; then
    printf 'Agent worker skill request wrappers must live in skill_read.rs\n' >&2
    exit 1
fi

if ! rg -q 'pub trait AgentWorkerMemoryReadApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs || \
   ! rg -q 'impl AgentWorkerMemoryReadApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/memory_read.rs || \
   ! rg -q 'fn agent_worker_memory_read_application\(' \
    crates/host/a4n-runtime/src/state/app/agent.rs || \
   ! rg -q 'agent_worker_memory_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/agent_runtime; then
    printf 'Agent worker memory reads must use their dedicated application port\n' >&2
    exit 1
fi
if ! rg -q 'pub trait AgentWorkerExecutionResultReadApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs || \
   ! rg -q 'impl AgentWorkerExecutionResultReadApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/execution_result_read.rs || \
   ! rg -q 'fn agent_worker_execution_result_read_application\(' \
    crates/host/a4n-runtime/src/state/app/agent.rs || \
   ! rg -q 'agent_worker_execution_result_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/agent_runtime; then
    printf 'Agent worker execution-result reads must use their dedicated application port\n' >&2
    exit 1
fi
if ! rg -q 'pub\(crate\) async fn worker_execution_by_key' \
    crates/agent/a4n-agent-application/src/agent/worker/execution_ops.rs || \
   rg -n 'worker_execution_by_key' \
    crates/agent/a4n-agent-application/src/agent/generation.rs; then
    printf 'Agent worker execution lookup must live with worker execution operations\n' >&2
    exit 1
fi

if ! rg -q 'impl AgentWorkerUsageApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/usage.rs || \
   rg -n 'update_run_usage|AgentWorkerUsageApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/execution_ops.rs; then
    printf 'Agent worker usage persistence must live in usage.rs\n' >&2
    exit 1
fi

if ! rg -q 'pub trait AgentWorkerMessageRequestApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs || \
   ! rg -q 'impl AgentWorkerMessageRequestApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/message_request.rs || \
   ! rg -q 'fn agent_worker_message_request_application\(' \
    crates/host/a4n-runtime/src/state/app/agent.rs || \
   ! rg -q 'agent_worker_message_request_application\(\)' \
    crates/host/a4n-runtime/src/actions/agent_runtime/history.rs; then
    printf 'Agent worker message request dispatch must use its own application port\n' >&2
    exit 1
fi
if ! rg -q 'pub\(crate\) async fn apply_message_request' \
    crates/agent/a4n-agent-application/src/agent/worker/message_request.rs || \
   ! rg -q 'worker_message_request_for_claimed' \
    crates/agent/a4n-agent-application/src/agent/worker/message_request.rs || \
   rg -n 'pub\(crate\) async fn apply_message_request|pub\(super\) fn worker_message_request_for_claimed' \
    crates/agent/a4n-agent-application/src/agent/worker/messages.rs; then
    printf 'Agent message request construction/dispatch must live in message_request.rs\n' >&2
    exit 1
fi
if ! rg -q 'impl AgentWorkerHostCommandApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/host_command.rs || \
   ! rg -q 'authorize_worker_host_command\(request\)' \
    crates/host/a4n-runtime/src/state/app/agent.rs || \
   rg -q 'authorize_worker_host_command' \
    crates/agent/a4n-agent-application/src/agent/worker/lease.rs; then
    printf 'Agent worker Host command authorization must live outside lease.rs\n' >&2
    exit 1
fi
if ! rg -q 'impl AgentWorkerHistoryApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/history.rs || \
   ! rg -q 'pub\(crate\) async fn load_message_request' \
    crates/agent/a4n-agent-application/src/agent/worker/inbox_ops.rs || \
   ! rg -q 'impl AgentWorkerRecoveryApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/recovery.rs; then
    printf 'Agent worker history, inbox-message, and recovery implementations must live with their owners\n' >&2
    exit 1
fi
if ! rg -q 'pub async fn user_messages_for_run' \
    crates/agent/a4n-agent-application/src/agent/worker/history.rs || \
   ! rg -q 'pub async fn latest_model_input_message_id' \
    crates/agent/a4n-agent-application/src/agent/worker/history.rs || \
   rg -n 'user_messages_for_run|latest_model_input_message_id' \
    crates/agent/a4n-agent-application/src/agent/generation.rs; then
    printf 'Agent worker history queries must live with worker history owner\n' >&2
    exit 1
fi
if rg -n 'pub\(crate\) async fn (load_message_request|worker_history_request|mark_recovery_required)' \
    crates/agent/a4n-agent-application/src/agent/worker/approval_ops.rs; then
    printf 'Non-approval worker operations must not return to approval_ops.rs\n' >&2
    exit 1
fi
if ! rg -q 'pub\(crate\) async fn expire_pending_approvals' \
    crates/agent/a4n-agent-application/src/agent/worker/approval_ops.rs || \
   rg -q 'pub\(crate\) async fn expire_pending_approvals' \
    crates/agent/a4n-agent-application/src/agent/worker/lease.rs; then
    printf 'Agent approval expiry must live in the approval owner, not lease.rs\n' >&2
    exit 1
fi
if rg -U -n 'agent_worker_message_application\(\)\s*\.\s*apply_message_request' \
    crates/host/a4n-runtime/src/actions/agent_runtime; then
    printf 'Agent message request dispatch must not use the runtime write port\n' >&2
    exit 1
fi
if rg -n 'AgentWorkerReadApplicationPort|AgentWorkerRunReadApplicationPort|agent_worker_(read|run_read)_application\(' \
    crates/agent crates/host/a4n-runtime; then
    printf 'Agent worker reads must not return to broad read application ports\n' >&2
    exit 1
fi
if rg -U -n 'state\.application\(\)\s*\.\s*show_run' \
    crates/agent/a4n-agent-runtime/src/engine; then
    printf 'Agent worker terminal status must consume AgentWorkerCompletionApplicationPort\n' >&2
    exit 1
fi
if rg -n 'state\.application\(\)\.review_delegation' \
    crates/agent/a4n-agent-runtime/src/engine; then
    printf 'Agent worker delegation review must consume AgentWorkerDelegationReviewApplicationPort\n' >&2
    exit 1
fi
if ! rg -q 'pub trait AgentWorkerDelegationReviewApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs || \
   ! rg -q 'impl AgentWorkerDelegationReviewApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/delegation_review.rs || \
   ! rg -q 'fn agent_worker_delegation_review_application\(' \
    crates/host/a4n-runtime/src/state/app/agent.rs || \
   ! rg -q 'agent_worker_delegation_review_application\(\)' \
    crates/host/a4n-runtime/src/actions/agent_runtime; then
    printf 'Agent worker delegation review must use its dedicated application port\n' >&2
    exit 1
fi
if ! rg -q 'pub trait AgentWorkerEventWriteApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs || \
   ! rg -q 'impl AgentWorkerEventWriteApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/events.rs || \
   ! rg -q 'fn agent_worker_event_write_application\(' \
    crates/host/a4n-runtime/src/state/app/agent.rs || \
   ! rg -q 'agent_worker_event_write_application\(\)' \
    crates/host/a4n-runtime/src/actions/agent_runtime; then
    printf 'Agent worker event writes must use their dedicated application port\n' >&2
    exit 1
fi
if ! rg -q 'pub trait AgentWorkerSubagentThreadApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs || \
   ! rg -q 'impl AgentWorkerSubagentThreadApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/subagent_thread.rs || \
   ! rg -q 'fn agent_worker_subagent_thread_application\(' \
    crates/host/a4n-runtime/src/state/app/agent.rs || \
   ! rg -q 'agent_worker_subagent_thread_application\(\)' \
    crates/host/a4n-runtime/src/actions/agent_runtime; then
    printf 'Agent worker subagent Thread operations must use their dedicated application port\n' >&2
    exit 1
fi

if ! rg -q 'pub\(crate\) async fn ensure_canonical_subagent_thread_request' \
    crates/agent/a4n-agent-application/src/agent/worker/subagent_thread.rs || \
   rg -n 'ensure_canonical_subagent_thread_request' \
    crates/agent/a4n-agent-application/src/agent/core/threads.rs; then
    printf 'Agent worker canonical Thread request must live outside core thread reads\n' >&2
    exit 1
fi

if ! rg -q 'pub\(crate\) async fn load_message_for_worker' \
    crates/agent/a4n-agent-application/src/agent/worker/inbox_ops.rs || \
   rg -n 'load_message_for_worker' \
    crates/agent/a4n-agent-application/src/agent/core/threads.rs; then
    printf 'Agent worker inbox message loading must live in inbox_ops.rs\n' >&2
    exit 1
fi

if ! rg -q 'pub\(crate\) async fn review_delegation_request' \
    crates/agent/a4n-agent-application/src/agent/worker/delegation_review.rs || \
   rg -n 'review_delegation_request' \
    crates/agent/a4n-agent-application/src/agent/core/interactions.rs; then
    printf 'Agent worker delegation review request must live outside core interactions\n' >&2
    exit 1
fi

if ! rg -q 'impl AgentWorkerDelegationApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/delegation.rs || \
   rg -n 'create_child_run(_request)?' \
    crates/agent/a4n-agent-application/src/agent/worker/execution_ops.rs; then
    printf 'Agent worker child-run delegation must live outside execution_ops.rs\n' >&2
    exit 1
fi

if ! rg -q 'impl AgentWorkerActivationApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/activation.rs || \
   rg -n 'mark_turn_active' \
    crates/agent/a4n-agent-application/src/agent/worker/wait.rs; then
    printf 'Agent worker activation must live outside wait.rs\n' >&2
    exit 1
fi

if ! rg -q 'impl AgentEditorResponseApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/editor.rs || \
   rg -n 'wait_for_editor|claim_editor_request|requeue_after_external_response' \
    crates/agent/a4n-agent-application/src/agent/worker/wait.rs; then
    printf 'Agent editor wait/response operations must live outside wait.rs\n' >&2
    exit 1
fi

if ! rg -q 'complete_child_run_fields' \
    crates/agent/a4n-agent-application/src/agent/worker/completion.rs || \
   rg -n 'complete_child_run_fields' \
    crates/agent/a4n-agent-application/src/agent/worker/execution_ops.rs; then
    printf 'Agent worker child completion must live with completion\n' >&2
    exit 1
fi

if ! rg -q 'impl AgentWorkerFailureApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/failure.rs || \
   rg -n 'fail_(claimed_run|child_run|worker_run)' \
    crates/agent/a4n-agent-application/src/agent/worker/completion.rs; then
    printf 'Agent worker failure persistence must live outside completion.rs\n' >&2
    exit 1
fi

for port in tool_read skill_read memory_read execution_result_read message_request delegation_review event_write subagent_thread; do
    if ! rg -q "fn agent_worker_${port}_application\(" \
        crates/host/a4n-runtime/src/state/app/agent.rs || \
       ! rg -q "agent_worker_${port}_application\(\)" \
        crates/host/a4n-runtime/src/actions/agent_runtime; then
        printf 'Agent worker %s must use its dedicated application port\n' "$port" >&2
        exit 1
    fi
done

if ! rg -n 'pub trait AgentWorkerBudgetApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent worker budget application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'AgentWorkerBudgetSnapshot' \
    crates/agent/a4n-agent-runtime-contract/src/lib.rs || \
   ! rg -n 'agent_worker_budget_application\(\)' \
    crates/host/a4n-runtime/src/state/app{.rs,} \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,} || \
   ! rg -n 'worker_budget_snapshot' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Agent worker budget must use its typed narrow accessor and snapshot\n' >&2
    exit 1
fi

if ! rg -q 'impl AgentWorkerBudgetApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/budget.rs || \
   ! rg -q 'pub async fn run_budget_snapshot' \
    crates/agent/a4n-agent-application/src/agent/budget.rs || \
   rg -n 'run_budget_snapshot|worker_budget_snapshot' \
    crates/agent/a4n-agent-application/src/agent/worker/lease.rs; then
    printf 'Agent budget aggregation and Worker budget adaptation must not live in lease.rs\n' >&2
    exit 1
fi

if ! rg -q 'impl AgentWorkerContextApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/context.rs || \
   rg -n 'load_run_context|update_context_summary' \
    crates/agent/a4n-agent-application/src/agent/worker/lease.rs; then
    printf 'Agent worker context operations must live outside lease.rs\n' >&2
    exit 1
fi

if ! rg -q 'impl AgentWorkerEventWriteApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/events.rs || \
   rg -n 'append_worker_update' \
    crates/agent/a4n-agent-application/src/agent/worker/wait.rs; then
    printf 'Agent worker event writes must live in events.rs\n' >&2
    exit 1
fi

if ! rg -q 'impl AgentRunLeaseRecoveryApplicationPort' \
    crates/agent/a4n-agent-application/src/agent/worker/recovery.rs || \
   rg -n 'recover_expired_runs' \
    crates/agent/a4n-agent-application/src/agent/worker/lease.rs; then
    printf 'Agent lease recovery must live in recovery.rs\n' >&2
    exit 1
fi

if ! rg -q 'result_refs_for_run_id' \
    crates/agent/a4n-agent-application/src/agent/worker/completion.rs || \
   rg -n 'result_refs_for_run' \
    crates/agent/a4n-agent-application/src/agent/worker/lease.rs; then
    printf 'Agent worker result references must live with completion\n' >&2
    exit 1
fi

if ! rg -q 'delegation_depth_for_run' \
    crates/agent/a4n-agent-application/src/agent/core/lineage.rs || \
   rg -n 'delegation_depth_for_run' \
    crates/agent/a4n-agent-application/src/agent/worker/lease.rs; then
    printf 'Agent delegation lineage must live outside worker lease lifecycle\n' >&2
    exit 1
fi

if rg -n 'run_budget_snapshot' crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Runtime Agent worker must not call the broad run_budget_snapshot owner method\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentRunLeaseRecoveryApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent run lease-recovery application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'agent_run_lease_recovery_application\(\)' \
    crates/host/a4n-runtime/src/state/app{.rs,} \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,} || \
   ! rg -U -n '(run_lease_recovery_application|agent_run_lease_recovery_application\(\))\s*\.\s*recover_expired_runs\((16|request\.limit)' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Agent expired-run recovery must use its typed narrow accessor\n' >&2
    exit 1
fi

if rg -n 'agent\.recover_expired_runs' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Agent expired-run recovery must not use the broad worker owner accessor\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentWorkerLeaseApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent worker-lease application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'agent_worker_lease_application\(\)' \
    crates/host/a4n-runtime/src/state/app{.rs,} \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,} || \
   ! rg -U -n '(renew_agent\s*\.\s*renew_worker_lease\(\s*&renew_claim\s*\)|agent_worker_lease_application\(\)\s*\.\s*renew_worker_lease_request\(\s*request)' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Agent worker lease renewal must use its typed narrow accessor\n' >&2
    exit 1
fi

if rg -n '(^|[^[:alnum:]_])agent\.renew_worker_lease' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Agent worker lease renewal must not use the broad worker owner accessor\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentWorkerClaimApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent worker-claim application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'agent_worker_claim_application\(\)' \
    crates/host/a4n-runtime/src/state/app{.rs,} \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,} || \
   ! rg -U -n '(claim_application\s*\.\s*claim_worker_run\(\s*&worker_id\s*\)|agent_worker_claim_application\(\)\s*\.\s*claim_worker_run\(\s*worker_id\s*\))' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Agent worker claim must use its typed narrow accessor\n' >&2
    exit 1
fi

if rg -n '(^|[^[:alnum:]_])agent\.claim_worker_run' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Agent worker claim must not use the broad worker owner accessor\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentWorkerFailureApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent worker-failure application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'agent_worker_failure_application\(\)' \
    crates/host/a4n-runtime/src/state/app{.rs,} \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,} || \
   ! rg -U -n 'agent_worker_failure_application\(\)\s*\.\s*fail_child_run\(\s*&claimed' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,} || \
   ! rg -U -n 'agent_worker_failure_application\(\)\s*\.\s*fail_worker_run\(\s*request' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Agent worker failure persistence must use its typed narrow accessor\n' >&2
    exit 1
fi

if rg -n '(^|[^[:alnum:]_])agent\.fail_child_run' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Agent worker failure persistence must not use the broad worker owner accessor\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentApprovalExpiryApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent approval-expiry application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'agent_approval_expiry_application\(\)' \
    crates/host/a4n-runtime/src/state/app{.rs,} \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,} || \
   ! rg -U -n 'agent_approval_expiry_application\(\)\s*\.\s*expire_pending_approvals\(\s*(32|request\.limit)\s*\)' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Agent approval expiry scan must use its typed narrow accessor\n' >&2
    exit 1
fi

if rg -n 'agent\.expire_pending_approvals' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Agent approval expiry scan must not use the broad worker owner accessor\n' >&2
    exit 1
fi

if ! rg -n 'Agent(Execution|Inbox|Message)Snapshot' \
    crates/agent/a4n-agent-application/src/agent \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Agent worker snapshot contracts are missing\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentEditorWaitApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent editor-wait application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'agent_editor_wait_application\(\)' \
    crates/host/a4n-runtime/src/state/app{.rs,} \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,} || \
   ! rg -U -n 'agent_editor_wait_application\(\)\s*\.\s*expire_editor_requests\(\s*(32|request\.limit)\s*\)' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Agent editor-wait expiry scan must use its typed narrow accessor\n' >&2
    exit 1
fi

if rg -n 'agent\.expire_editor_requests' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Agent editor-wait expiry scan must not use the broad worker owner accessor\n' >&2
    exit 1
fi

if rg -n 'pub async fn expire_editor_requests' \
    crates/agent/a4n-agent-application/src/agent/mod.rs; then
    printf 'Agent editor-wait owner wrapper must live in editor_wait.rs\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentExternalTaskWaitApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent external-task wait port is missing\n' >&2
    exit 1
fi

if ! rg -n 'AgentExternalTaskWaitSnapshot' \
    crates/agent/a4n-agent-runtime-contract/src/lib.rs \
    crates/host/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent external-task wait snapshot contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentGenerationCancelApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent generation-cancel application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'AgentGenerationCancelSnapshot' \
    crates/agent/a4n-agent-runtime-contract/src/lib.rs \
    crates/host/a4n-runtime/src/actions/agent_runtime/task_wait.rs || \
   ! rg -n 'agent_generation_cancel_application\(\)' \
    crates/host/a4n-runtime/src/state/app{.rs,} \
    crates/host/a4n-runtime/src/actions/agent_runtime/task_wait.rs || \
   ! rg -U -n 'generation_cancel_application\s*\.\s*generation_cancel_candidates' \
    crates/host/a4n-runtime/src/actions/agent_runtime/task_wait.rs || \
   ! rg -n 'cancel_generation_task' \
    crates/host/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent generation cancellation must use its typed narrow accessor and snapshot\n' >&2
    exit 1
fi

if rg -n 'agent\.(active_owned_generation_tasks|generation_owner)\(' \
    crates/host/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent generation cancellation must not use broad owner queries from Runtime\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentWorkflowCancelApplicationPort' \
    crates/agent/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent Workflow-cancel application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'AgentWorkflowCancelSnapshot' \
    crates/agent/a4n-agent-runtime-contract/src/lib.rs \
    crates/host/a4n-runtime/src/actions/agent_runtime/task_wait.rs || \
   ! rg -n 'agent_workflow_cancel_application\(\)' \
    crates/host/a4n-runtime/src/state/app{.rs,} \
    crates/host/a4n-runtime/src/actions/agent_runtime/task_wait.rs || \
   ! rg -U -n 'workflow_cancel_application\s*\.\s*workflow_cancel_candidates' \
    crates/host/a4n-runtime/src/actions/agent_runtime/task_wait.rs || \
   ! rg -n 'workflow_cancellation_application\(\)' \
    crates/host/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent Workflow cancellation must use its typed narrow accessor and preserve the cancellation owner\n' >&2
    exit 1
fi

if rg -n 'active_owned_workflows|workflow_owner|generation_owner_stopped' \
    crates/host/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent Workflow cancellation must not use broad owner queries from Runtime\n' >&2
    exit 1
fi

if ! rg -n 'agent_external_task_wait_application\(\)' \
    crates/host/a4n-runtime/src/actions/agent_runtime{.rs,}; then
    printf 'Agent external-task wait recovery must use its narrow application port\n' >&2
    exit 1
fi

if rg -n 'AgentExecutionSnapshot|run_budget_snapshot' \
    crates/host/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent external-task wait recovery must not consume broad execution snapshots or budget owner methods\n' >&2
    exit 1
fi

if rg -n 'scope_json' crates/host/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent external-task wait recovery must consume parsed scope snapshots instead of persistence JSON\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)' crates/host/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent wait recovery must consume WorkflowCancellationPort instead of the full orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'workflow_cancellation_application\(\)' \
    crates/host/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent wait recovery must route workflow cancellation through its narrow port\n' >&2
    exit 1
fi
