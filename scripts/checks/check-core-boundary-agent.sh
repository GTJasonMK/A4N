#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

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
    printf 'Agent worker tool discovery must consume AgentWorkerReadApplicationPort\n' >&2
    exit 1
fi
if ! rg -n 'search_tools_for_worker|describe_tool_for_worker' \
    crates/agent/a4n-agent-application/src/agent/worker/ports.rs \
    crates/agent/a4n-agent-runtime/src/engine; then
    printf 'Agent worker tool discovery read port is missing\n' >&2
    exit 1
fi
if rg -U -n 'ensure_argument_resource_scope\(\s*self\.state\.application\(\)' \
    crates/agent/a4n-agent-runtime/src/engine; then
    printf 'Agent worker resource scope checks must consume AgentWorkerReadApplicationPort\n' >&2
    exit 1
fi
if ! rg -n 'verify_chapters_in_project_for_worker' \
    crates/agent/a4n-agent-application/src/agent/worker/ports.rs \
    crates/agent/a4n-agent-runtime/src/engine; then
    printf 'Agent worker resource scope read port is missing\n' >&2
    exit 1
fi
if rg -U -n 'state\.application\(\)\s*\.\s*show_run' \
    crates/agent/a4n-agent-runtime/src/engine; then
    printf 'Agent worker terminal status must consume AgentWorkerCompletionApplicationPort\n' >&2
    exit 1
fi
if rg -n 'state\.application\(\)\.review_delegation' \
    crates/agent/a4n-agent-runtime/src/engine; then
    printf 'Agent worker delegation review must consume AgentWorkerDelegationApplicationPort\n' >&2
    exit 1
fi
if ! rg -n 'review_delegation' \
    crates/agent/a4n-agent-application/src/agent/worker/ports.rs \
    crates/agent/a4n-agent-runtime/src/engine; then
    printf 'Agent worker delegation review port is missing\n' >&2
    exit 1
fi

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
