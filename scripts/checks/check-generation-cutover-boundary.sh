#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

for path in \
    crates/a4n-core/src/event/sse.rs \
    crates/a4n-core/src/event/host_task.rs \
    crates/a4n-llm-http/src/http/provider/chat.rs \
    crates/a4n-llm-gateway/src/handlers/chat.rs \
    crates/a4n-service-protocol/src/events/service.rs \
    crates/a4n-server/src/routes/events.rs \
    crates/a4n-server/src/routes/commands/task/events.rs \
    crates/a4n-tauri/src/events/bridge.rs; do
    if [[ -e "$path" ]]; then
        printf 'retired generation contract module exists: %s\n' "$path" >&2
        exit 1
    fi
done

if rg -n \
    '\b(ServiceTaskEventKind|ServiceTaskEvent|ProgressTaskKind|SseEvent|HostTaskEvent)\b|event::(sse|host_task)|sanitize_service_task_event|host-task:|emit_host_task_event|events::bridge::EventBridge' \
    crates frontend/src \
    --glob '*.rs' --glob '*.ts' --glob '*.tsx'; then
    printf 'retired generation contract references remain in production or test source\n' >&2
    exit 1
fi

if rg -n '/llm/chat|async fn chat\s*\(' \
    crates/a4n-capabilities/src/contracts/llm.rs \
    crates/a4n-capabilities/src/llm.rs \
    crates/a4n-llm/src/provider \
    crates/a4n-llm-gateway/src \
    crates/a4n-llm-http/src/http/provider \
    --glob '*.rs'; then
    printf 'retired full-response LLM entry remains\n' >&2
    exit 1
fi

# generation 前端不得再订阅旧 workflow 通道或手写 WorkflowEvent。
if rg -n \
    '\b(WorkflowEvent|createProgressTaskId)\b|workflow:\$\{|listenAppEvent[^
]*workflow:|`workflow:\$\{' \
    frontend/src \
    --glob '*.ts' --glob '*.tsx'; then
    printf 'retired frontend workflow event references remain\n' >&2
    exit 1
fi

# 生产业务不得再调用完整响应 chat_llm。
if rg -n \
    '\bchat_llm\s*\(' \
    crates \
    --glob '*.rs' \
    --glob '!**/*test*.rs' \
    --glob '!**/tests/**'; then
    printf 'retired chat_llm production calls remain\n' >&2
    exit 1
fi

# 完整响应和隐藏 collector 的迁移期逃生口不得保留。
if rg -n \
    '\b(NullPublisher|collector_only)\b' \
    crates frontend/src \
    --glob '*.rs' --glob '*.ts' --glob '*.tsx' \
    --glob '!**/*test*.rs' --glob '!**/tests/**'; then
    printf 'retired generation publisher/collector escape hatch remains\n' >&2
    exit 1
fi

# 领域层不得重新手写 LLM stream 收集循环；共享 provider adapter 和 RAG ingest 各有独立 owner。
if rg -n \
    'while[[:space:]]+let[[:space:]]+Some\(.*\.next\(\)\.await|collect_stream' \
    crates/a4n-orchestrator/src \
    crates/a4n-runtime/src \
    --glob '*.rs' \
    --glob '!crates/a4n-runtime/src/rag_ingest/sync/batch.rs' \
    --glob '!**/*test*.rs' --glob '!**/tests/**'; then
    printf 'manual generation stream collector remains outside the canonical adapter\n' >&2
    exit 1
fi

# 已删除的 Host 后台事件通道不得复活。
if rg -n \
    'format!\("workflow:|workflow_channel\b|emit_workflow_event\b|/events/\{channel\}' \
    crates \
    --glob '*.rs'; then
    printf 'retired Host workflow channel helpers remain\n' >&2
    exit 1
fi

# Rust 生成物是唯一前端合同源；生产前端不得手写同名事件/快照类型。
if rg -n \
    'export[[:space:]]+(interface|type|enum)[[:space:]]+(GenerationTaskEvent|GenerationEvent|GenerationTaskSnapshot|GenerationTaskStatus|GenerationArtifactRef|GenerationError|TextDeltaEvent|CheckpointEvent|ArtifactEvent|CancelRequestedEvent)\b' \
    frontend/src \
    --glob '*.ts' --glob '*.tsx' \
    --glob '!frontend/src/api/generated/**'; then
    printf 'hand-written frontend generation contract remains\n' >&2
    exit 1
fi

# Artifact、checkpoint 和 terminal/status 事件只能由各自 canonical owner 构造。
if rg -n '\bArtifactEvent\b' crates/a4n-orchestrator/src \
    --glob '*.rs' \
    --glob '!crates/a4n-orchestrator/src/generation/artifacts.rs' \
    --glob '!crates/a4n-orchestrator/src/generation/events.rs'; then
    printf 'raw ArtifactEvent escaped the artifact coordinator/reducer boundary\n' >&2
    exit 1
fi
if rg -n '\bCheckpointEvent\b' crates/a4n-orchestrator/src \
    --glob '*.rs' \
    --glob '!crates/a4n-orchestrator/src/generation/checkpoint.rs' \
    --glob '!crates/a4n-orchestrator/src/generation/events.rs'; then
    printf 'CheckpointEvent escaped the checkpoint writer/reducer boundary\n' >&2
    exit 1
fi
if rg -n \
    '\b(CancelRequestedEvent|CompletedEvent|FailedEvent|CancelledEvent|InterruptedEvent|StatusChangedEvent)\b' \
    crates/a4n-orchestrator/src \
    --glob '*.rs' \
    --glob '!crates/a4n-orchestrator/src/generation/terminal.rs' \
    --glob '!crates/a4n-orchestrator/src/generation/events.rs'; then
    printf 'terminal/status event escaped the terminal coordinator/reducer boundary\n' >&2
    exit 1
fi
if rg -n 'GenerationEvent::Artifact[[:space:]]*\(' \
    crates/a4n-orchestrator/src crates/a4n-runtime/src \
    --glob '*.rs' \
    --glob '!crates/a4n-orchestrator/src/generation/terminal.rs' \
    --glob '!crates/a4n-orchestrator/src/generation/events.rs'; then
    printf 'artifact event is published outside the terminal coordinator\n' >&2
    exit 1
fi
if rg -n 'GenerationEvent::Checkpoint[[:space:]]*\(' \
    crates/a4n-orchestrator/src crates/a4n-runtime/src \
    --glob '*.rs' \
    --glob '!crates/a4n-orchestrator/src/generation/checkpoint.rs' \
    --glob '!crates/a4n-orchestrator/src/generation/events.rs'; then
    printf 'checkpoint event is published outside the checkpoint writer\n' >&2
    exit 1
fi
if rg -n \
    'GenerationEvent::(CancelRequested|Completed|Failed|Cancelled|Interrupted|StatusChanged)[[:space:]]*\(' \
    crates/a4n-orchestrator/src crates/a4n-runtime/src \
    --glob '*.rs' \
    --glob '!crates/a4n-orchestrator/src/generation/terminal.rs' \
    --glob '!crates/a4n-orchestrator/src/generation/events.rs'; then
    printf 'terminal/status event is published outside the terminal coordinator\n' >&2
    exit 1
fi

# completed 已持 checkpoint row lock；committed permit 必须同步准备，不能在事务内排队等待。
if rg -n \
    'pub[[:space:]]+(crate\([^)]*\)[[:space:]]+)?async[[:space:]]+fn[[:space:]]+prepare_committed_events\b' \
    crates/a4n-orchestrator/src/generation/events.rs; then
    printf 'generation committed permit can wait while a database row lock is held\n' >&2
    exit 1
fi

# 数据库 lifecycle gate 必须进入 canonical entity，且不得退回业务 status/is_active 或内存 flag。
for entity in \
    crates/a4n-core/src/models/project/novel_project.rs \
    crates/a4n-core/src/models/user/user.rs; do
    if ! rg -U -q '#\[serde\(skip\)\][[:space:]]+pub lifecycle_gate: String' "$entity"; then
        printf 'generation lifecycle gate is missing from canonical entity: %s\n' "$entity" >&2
        exit 1
    fi
done
if rg -n \
    'Column::(Status|IsActive)|\.(lifecycle_blocked|deleting|resetting|disabling)[[:space:]]*(==|!=)' \
    crates/a4n-db/src/generation/lifecycle_gate.rs; then
    printf 'project status, user is_active, or an in-memory flag is used as generation lifecycle gate\n' >&2
    exit 1
fi
if rg -n \
    '\bis_active\b|\bProjectStatus\b|Column::(Status|IsActive)|\.(lifecycle_blocked|deleting|resetting|disabling)\b' \
    crates/a4n-runtime/src/actions/generation_tasks/destructive_lifecycle.rs; then
    printf 'Host destructive coordinator bypasses the database lifecycle gate\n' >&2
    exit 1
fi

# 破坏式 cutover 不保留兼容 feature，也不允许 Host/领域重新生成规范 task id。
if rg -n \
    '^[[:space:]]*((legacy|compat)[_-]?(generation|event|stream)|(generation|event|stream)[_-]?(legacy|compat))[[:space:]]*=' \
    Cargo.toml crates/a4n-*/Cargo.toml \
    || rg -n \
    '#\[cfg\([^]]*feature[[:space:]]*=[[:space:]]*"[^"\n]*((legacy|compat)[_-]?(generation|event|stream)|(generation|event|stream)[_-]?(legacy|compat))[^"\n]*"' \
    crates/a4n-*/src frontend/src \
    --glob '*.rs' --glob '*.ts' --glob '*.tsx'; then
    printf 'generation compatibility feature flag remains\n' >&2
    exit 1
fi
if rg -n \
    '\b(create|make|build|new|generate)_(generation_)?task_id\b|\b(generation_)?task_id_(create|make|build|new|generate)\b' \
    crates/a4n-orchestrator/src \
    crates/a4n-runtime/src \
    crates/a4n-server/src \
    crates/a4n-tauri/src \
    frontend/src \
    --glob '*.rs' --glob '*.ts' --glob '*.tsx' \
    --glob '!**/*test*.rs' --glob '!**/tests/**'; then
    printf 'retired generation task id helper remains outside database reservation\n' >&2
    exit 1
fi

# Generation cleanup 只能复用现有 outbox worker；禁止第二 worker 或内存补偿队列。
if rg -n \
    '(spawn|worker|drain)[^\n]*(generation[^\n]*cleanup|cleanup[^\n]*generation)|(generation[^\n]*cleanup|cleanup[^\n]*generation)[^\n]*(spawn|worker|drain)' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src crates/a4n-db/src \
    --glob '*.rs' \
    --glob '!crates/a4n-runtime/src/cleanup_outbox/worker.rs' \
    --glob '!**/*test*.rs' --glob '!**/tests/**'; then
    printf 'second generation cleanup worker remains outside the canonical outbox worker\n' >&2
    exit 1
fi
if rg -n \
    '(VecDeque|mpsc::channel|broadcast::channel)[^\n]*(cleanup|compensation)|(cleanup|compensation)[^\n]*(VecDeque|mpsc::channel|broadcast::channel)' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src crates/a4n-db/src \
    --glob '*.rs' \
    --glob '!**/*test*.rs' --glob '!**/tests/**'; then
    printf 'in-memory cleanup compensation queue remains\n' >&2
    exit 1
fi

# 禁止第二 cleanup owner、generation broadcast hub、位置泄漏和已删除的用户删除入口。
if rg -n \
    '\bgeneration_cleanup_outbox\b|broadcast(::|[^\n]*)(channel|Sender|Receiver)[^\n]*Generation(Task)?Event|Generation(Task)?Event[^\n]*broadcast(::|[^\n]*)|\b(delete_user|delete_user_for_admin|deleteUser)\b' \
    crates frontend/src \
    --glob '*.rs' --glob '*.ts' --glob '*.tsx' \
    --glob '!**/*test*.rs' --glob '!**/tests/**'; then
    printf 'forbidden generation cleanup/broadcast/user-delete boundary remains\n' >&2
    exit 1
fi
if rg -ni \
    'pub[[:space:]]+[a-zA-Z0-9_]*(local_?path|file_?path|file_?url|artifact_?url|download_?url|public_?url)[a-zA-Z0-9_]*[[:space:]]*:' \
    crates/a4n-service-protocol/src/events/generation.rs; then
    printf 'generation event contract exposes a local path or URL\n' >&2
    exit 1
fi

"$ROOT/scripts/checks/inventory-generation-paths.sh" --check

printf 'retired generation contract boundary is clean\n'
