#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

if ! rg -n 'pub trait WorkflowTaskLookupApplicationPort|pub trait WorkflowCancellationPort' \
    crates/a4n-api-contract/src/workflow.rs; then
    printf 'Workflow runtime ports must live in the API contract\n' >&2
    exit 1
fi

if rg -n 'pub trait WorkflowTaskLookupApplicationPort|pub trait WorkflowCancellationPort' \
    crates/a4n-orchestrator/src/workflow.rs; then
    printf 'Workflow runtime ports must not be redefined in the orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'pub trait TaskCancellationPort' crates/a4n-api-contract/src/task.rs; then
    printf 'TaskCancellationPort must live in the API contract\n' >&2
    exit 1
fi

if rg -n 'pub trait TaskCancellationPort' crates/a4n-orchestrator/src/task_queue.rs; then
    printf 'TaskCancellationPort must not be redefined in the orchestrator queue module\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)\.(register_task_handle|remove_task_handle|task_handle|cancel_task_handle|cancel_owned_task_handle|cancel_or_remember_task_handle|active_task_snapshots)' \
    crates/a4n-runtime/src/actions/task_records; then
    printf 'Runtime task handle actions must use the a4n-task-runtime owner directly\n' >&2
    exit 1
fi

if ! rg -n 'state\.task_manager\(\)' \
    crates/a4n-runtime/src/actions/task_records/handles.rs \
    crates/a4n-runtime/src/actions/task_records/access/task.rs \
    crates/a4n-runtime/src/actions/task_records/queries/workflows \
    crates/a4n-runtime/src/actions/task_records/workflows/cancellation.rs; then
    printf 'Runtime task handle actions must route through AppState task manager\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)' crates/a4n-runtime/src/actions/storage_paths/paths.rs; then
    printf 'Runtime storage path display must consume AppState configuration and a4n-storage paths directly\n' >&2
    exit 1
fi

if rg -n 'existing_project_ids\(state\.orchestrator\(\)\)|project_read:\s*&a4n_orchestrator::OrchestratorContext' \
    crates/a4n-runtime/src/actions/storage_paths/orphans.rs; then
    printf 'Runtime orphan scans must use ProjectReadApplicationPort for project IDs\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*project_by_id' \
    crates/a4n-runtime/src/actions/storage_paths/orphans.rs; then
    printf 'Runtime orphan scans must use the project read port for existence checks\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*(project_ids_with_incomplete_cleanup|media_path_prefix_has_persisted_reference)' \
    crates/a4n-runtime/src/actions/storage_paths/orphans.rs; then
    printf 'Runtime orphan cleanup must use cleanup and image application ports\n' >&2
    exit 1
fi

if ! rg -n 'cleanup_outbox_application\(\)' \
    crates/a4n-runtime/src/actions/storage_paths/orphans.rs || \
   ! rg -n 'image_media_reference_application\(\)|persisted_media_path_prefix_has_reference\(' \
    crates/a4n-runtime/src/actions/storage_paths/orphans.rs; then
    printf 'Runtime orphan cleanup must route cleanup and media reference checks through application ports\n' >&2
    exit 1
fi

printf 'pure crate dependency boundaries are valid\n'
