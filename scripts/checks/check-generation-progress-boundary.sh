#!/usr/bin/env bash
#=============================================================================
# 生成进度通道边界检查
#
# 约束：generation 任务的进度与取消必须走 generationClient（A 路径），
# 旧 Host workflow 轮询通道（allocateHostTaskId + pollTaskProgress）只减不增。
#
# 迁移完成后（上限归零）应连同旧通道一起删除本检查。
# 详见 .claude/plan-generation-progress-unification.md
#=============================================================================
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FRONTEND_SRC="frontend/src"
HOOK="$FRONTEND_SRC/hooks/useProgressTask.tsx"

# 阶段 4 逐子系统迁移时同步下调；不得上调。
LEGACY_HOST_TASK_BUDGET=15

[[ -f "$HOOK" ]] || {
    printf '进度 hook 缺失: %s\n' "$HOOK" >&2
    exit 1
}

fail=0

# 1) 旧 Host workflow 任务身份数量只减不增
legacy_count=$(rg -c --no-filename 'allocateHostTaskId\(' "$FRONTEND_SRC" \
    --glob '!**/*.test.*' --glob "!**/hooks/useProgressTask.tsx" 2>/dev/null \
    | awk '{sum+=$1} END {print sum+0}')

if [[ "$legacy_count" -gt "$LEGACY_HOST_TASK_BUDGET" ]]; then
    printf '旧 Host workflow 任务调用点增加: %s > 上限 %s\n' \
        "$legacy_count" "$LEGACY_HOST_TASK_BUDGET" >&2
    printf '新增生成任务必须走 runGenerationProgressTaskThen，不得新增 allocateHostTaskId\n' >&2
    fail=1
elif [[ "$legacy_count" -lt "$LEGACY_HOST_TASK_BUDGET" ]]; then
    printf '提示: 旧 Host workflow 调用点已降至 %s，请同步下调本脚本的 LEGACY_HOST_TASK_BUDGET\n' \
        "$legacy_count" >&2
    fail=1
fi

# 2) 禁止隐式取消兜底：cancelWorkflow 不得再作为 useProgressTask 的默认取消
if rg -q 'cancelTaskRef\.current \?\? cancelWorkflow' "$HOOK"; then
    printf '禁止隐式取消兜底: %s 不得默认落回 cancelWorkflow\n' "$HOOK" >&2
    printf '调用点必须显式传 cancelTask（generationClient.cancel 或 cancelHostWorkflowTask）\n' >&2
    fail=1
fi

# 3) 轮询归属必须显式声明，不得由 cancelTask 的有无推断
if rg -q 'pollTaskProgress \?\? ' "$HOOK"; then
    printf '禁止推断轮询归属: %s 的 pollTaskProgress 必须显式判定\n' "$HOOK" >&2
    fail=1
fi

# 4) generation 路径不得开启 Host 进度轮询（显式关闭是允许且推荐的）
if rg -n 'pollTaskProgress:\s*true' "$FRONTEND_SRC/hooks/runGenerationProgressTask.ts" >/dev/null 2>&1; then
    printf 'generation 路径不得开启 Host 进度轮询: runGenerationProgressTask.ts\n' >&2
    printf 'generation 任务的进度必须来自事件订阅\n' >&2
    fail=1
fi

# 5) 流式显示必须自适应，不得由调用点静态声明
if rg -q '\bshowStream\b' "$FRONTEND_SRC" --glob '!**/*.test.*'; then
    printf '禁止静态流式开关: showStream 已由「首个非空 delta 才渲染」的自适应逻辑取代\n' >&2
    rg -n '\bshowStream\b' "$FRONTEND_SRC" --glob '!**/*.test.*' >&2
    fail=1
fi

# 6) 前端 stage_code 映射表必须覆盖后端全部公共 stage
STAGE_MAP="$FRONTEND_SRC/utils/generationProgressMeta.ts"
CONTRACT="crates/a4n-service-protocol/src/events/generation.rs"

if [[ -f "$STAGE_MAP" && -f "$CONTRACT" ]]; then
    # 从 public_progress_phase 的 match 分支提取全部公共 stage_code
    backend_stages=$(rg -A 16 'fn public_progress_phase' "$CONTRACT" \
        | rg -o '"[a-z_]+"' | tr -d '"' | sort -u)
    [[ -n "$backend_stages" ]] || {
        printf '未能从 %s 提取公共 stage_code；请检查 public_progress_phase 是否改名\n' \
            "$CONTRACT" >&2
        exit 1
    }
    missing=""
    for stage in $backend_stages; do
        rg -q "^[[:space:]]+${stage}:" "$STAGE_MAP" || missing="$missing $stage"
    done
    if [[ -n "$missing" ]]; then
        printf '前端 stage_code 映射缺少后端公共 stage:%s\n' "$missing" >&2
        printf '请同步更新 %s 的 STAGE_PHASE 与 STAGE_LABEL\n' "$STAGE_MAP" >&2
        fail=1
    fi
else
    printf 'stage_code 映射校验输入缺失: %s / %s\n' "$STAGE_MAP" "$CONTRACT" >&2
    fail=1
fi

[[ "$fail" -eq 0 ]] || exit 1

printf '生成进度通道边界有效（旧 Host workflow 调用点: %s/%s）\n' \
    "$legacy_count" "$LEGACY_HOST_TASK_BUDGET"
