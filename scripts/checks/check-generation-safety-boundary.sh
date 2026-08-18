#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

for entity in \
    crates/a4n-core/src/models/project/novel_project.rs \
    crates/a4n-core/src/models/user/user.rs; do
    rg -U -q '#\[serde\(skip\)\][[:space:]]+pub lifecycle_gate: String' "$entity" || {
        printf 'generation lifecycle gate 缺失或可被序列化: %s\n' "$entity" >&2
        exit 1
    }
done

if rg -n \
    'Column::(Status|IsActive)|\.(lifecycle_blocked|deleting|resetting|disabling)[[:space:]]*(==|!=)' \
    crates/a4n-db/src/generation/lifecycle_gate.rs \
    || rg -n \
    '\bis_active\b|\bProjectStatus\b|Column::(Status|IsActive)|\.(lifecycle_blocked|deleting|resetting|disabling)\b' \
    crates/a4n-runtime/src/actions/generation_tasks/destructive_lifecycle.rs; then
    printf 'generation 破坏式操作绕过数据库 lifecycle gate\n' >&2
    exit 1
fi

if rg -n \
    'pub[[:space:]]+(crate\([^)]*\)[[:space:]]+)?async[[:space:]]+fn[[:space:]]+prepare_committed_events\b' \
    crates/a4n-orchestrator/src/generation/events.rs; then
    printf '持有 checkpoint 行锁时不得异步等待 committed event permit\n' >&2
    exit 1
fi

if rg -n \
    '(spawn|worker|drain)[^\n]*(generation[^\n]*cleanup|cleanup[^\n]*generation)|(generation[^\n]*cleanup|cleanup[^\n]*generation)[^\n]*(spawn|worker|drain)' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src crates/a4n-db/src \
    --glob '*.rs' \
    --glob '!crates/a4n-runtime/src/cleanup_outbox/worker.rs' \
    --glob '!**/*test*.rs' --glob '!**/tests/**' \
    || rg -n \
    '(VecDeque|mpsc::channel|broadcast::channel)[^\n]*(cleanup|compensation)|(cleanup|compensation)[^\n]*(VecDeque|mpsc::channel|broadcast::channel)' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src crates/a4n-db/src \
    --glob '*.rs' --glob '!**/*test*.rs' --glob '!**/tests/**'; then
    printf 'generation cleanup 必须只走持久化 cleanup outbox\n' >&2
    exit 1
fi

if rg -ni \
    'pub[[:space:]]+[a-zA-Z0-9_]*(local_?path|file_?path|file_?url|artifact_?url|download_?url|public_?url)[a-zA-Z0-9_]*[[:space:]]*:' \
    crates/a4n-service-protocol/src/events/generation.rs; then
    printf 'generation 公共事件不得暴露本地路径或 URL\n' >&2
    exit 1
fi

printf 'generation safety boundary is valid\n'
