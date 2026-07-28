#!/usr/bin/env bash
# 模块化重构进度度量（可复现）。
#
# 主 KPI = 实际要处理的工作队列，不是 actions 总行（~10 万含测试/Host 壳，仅作噪声背景）。
#
# 用法：
#   scripts/checks/refactor-metrics.sh
#   scripts/checks/refactor-metrics.sh --label "批次 090 完成后" --append
#   scripts/checks/refactor-metrics.sh --label "批次 090 完成后" --prev-label "批次 089 完成后" --append
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

LABEL=""
PREV_LABEL=""
APPEND=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --label) LABEL="${2:-}"; shift 2 ;;
    --prev-label) PREV_LABEL="${2:-}"; shift 2 ;;
    --append) APPEND=1; shift ;;
    -h|--help) sed -n '1,20p' "$0"; exit 0 ;;
    *) echo "未知参数: $1" >&2; exit 2 ;;
  esac
done

count_rs() {
  local dir="$1" exclude_tests="${2:-0}"
  if [[ ! -d "$dir" ]]; then printf '0 0\n'; return 0; fi
  local list
  if [[ "$exclude_tests" == "1" ]]; then
    list=$(find "$dir" -name '*.rs' ! -path '*/tests/*' ! -name 'tests.rs' ! -name '*_tests.rs' 2>/dev/null || true)
  else
    list=$(find "$dir" -name '*.rs' 2>/dev/null || true)
  fi
  if [[ -z "$list" ]]; then printf '0 0\n'; return 0; fi
  local files lines
  files=$(printf '%s\n' "$list" | wc -l | tr -d ' ')
  lines=$(printf '%s\n' "$list" | tr '\n' '\0' | xargs -0 cat 2>/dev/null | wc -l | tr -d ' ')
  printf '%s %s\n' "$files" "$lines"
}

fmt_delta() {
  local d=$1
  if [[ $d -gt 0 ]]; then echo "+$d"; else echo "$d"; fi
}

# ── 实际工作队列（主 KPI）──────────────────────────────────
eval "$(python3 <<'PY'
from pathlib import Path
import re
from collections import defaultdict
import shlex

ROOT = Path("crates/a4n-runtime/src/actions")

def is_test(p: Path) -> bool:
    s = str(p)
    return "/tests/" in s or p.name == "tests.rs" or p.name.endswith("_tests.rs")

local_rules, adapter_rules, io_rules, orch_thin, struct_shell = [], [], [], [], []

for p in ROOT.rglob("*.rs"):
    if is_test(p):
        continue
    text = p.read_text(encoding="utf-8", errors="replace")
    n = len(text.splitlines())
    if p.name == "rules.rs":
        if re.search(r"AppState|\bstate\.", text):
            io_rules.append((str(p), n))
        elif "a4n_orchestrator" in text:
            adapter_rules.append((str(p), n))
        else:
            local_rules.append((str(p), n))
        continue
    code = re.sub(r"//.*?$|/\*.*?\*/", "", text, flags=re.M | re.S)
    code = re.sub(r"#!?\[.*?\]", "", code, flags=re.S)
    has_item = bool(re.search(r"\b(fn|struct|enum|trait|impl|const|static|type|macro_rules)\b", code))
    has_local_module = bool(
        re.search(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*;", code)
    )
    if has_item or not re.search(r"\b(use|mod)\b", code):
        continue
    is_orch_only_reexport = "a4n_orchestrator" in text and not has_local_module
    (orch_thin if is_orch_only_reexport else struct_shell).append((str(p), n))

by_domain = defaultdict(int)
for path, n in local_rules:
    rel = Path(path).relative_to(ROOT)
    d = rel.parts[0] if len(rel.parts) > 1 else "_root"
    by_domain[d] += n

def exp(name, val):
    print(f"{name}={shlex.quote(str(val))}")

exp("LOCAL_RULES_FILES", len(local_rules))
exp("LOCAL_RULES_LINES", sum(n for _, n in local_rules))
exp("ADAPTER_RULES_FILES", len(adapter_rules))
exp("ADAPTER_RULES_LINES", sum(n for _, n in adapter_rules))
exp("IO_RULES_FILES", len(io_rules))
exp("IO_RULES_LINES", sum(n for _, n in io_rules))
exp("ORCH_THIN_FILES", len(orch_thin))
exp("ORCH_THIN_LINES", sum(n for _, n in orch_thin))
exp("STRUCT_SHELL_FILES", len(struct_shell))
exp("STRUCT_SHELL_LINES", sum(n for _, n in struct_shell))
exp("QUEUE_DOMAIN", ",".join(f"{k}={v}" for k, v in sorted(by_domain.items(), key=lambda x: -x[1])))
exp("QUEUE_DETAIL", "|".join(f"{Path(p).relative_to(ROOT)}={n}" for p, n in sorted(local_rules, key=lambda x: -x[1])))
PY
)"

read -r ACTIONS_FILES ACTIONS_LINES < <(count_rs crates/a4n-runtime/src/actions 0)
read -r ACTIONS_PROD_FILES ACTIONS_PROD_LINES < <(count_rs crates/a4n-runtime/src/actions 1)
read -r ORCH_FILES ORCH_LINES < <(count_rs crates/a4n-orchestrator/src 0)
read -r RUNTIME_FILES RUNTIME_LINES < <(count_rs crates/a4n-runtime/src 0)

APPSTATE_FILES=$(grep -rl 'AppState' crates/a4n-runtime/src/actions --include='*.rs' 2>/dev/null | wc -l | tr -d ' ' || true)
APPSTATE_HITS=$(grep -rn 'AppState' crates/a4n-runtime/src/actions --include='*.rs' 2>/dev/null | wc -l | tr -d ' ' || true)
ORCH_CALL_ACTIONS=$(grep -rn '\.orchestrator()' crates/a4n-runtime/src/actions --include='*.rs' 2>/dev/null | wc -l | tr -d ' ' || true)
ORCH_CALL_RUNTIME=$(grep -rn '\.orchestrator()' crates/a4n-runtime/src --include='*.rs' 2>/dev/null \
  | grep -v 'convergence_tests' | grep -v '/tests/' | wc -l | tr -d ' ' || true)
APPSTATE_FILES=${APPSTATE_FILES:-0}
APPSTATE_HITS=${APPSTATE_HITS:-0}
ORCH_CALL_ACTIONS=${ORCH_CALL_ACTIONS:-0}
ORCH_CALL_RUNTIME=${ORCH_CALL_RUNTIME:-0}

# 一级域 Top 10（噪声背景）
TOP_DOMAINS=""
while IFS=$'\t' read -r lines files name; do
  [[ -n "$name" ]] || continue
  TOP_DOMAINS+="| ${name} | ${lines} | ${files} |"$'\n'
done < <(
  for d in crates/a4n-runtime/src/actions/*/; do
    [[ -d "$d" ]] || continue
    name=$(basename "$d")
    mapfile -t rs < <(find "$d" -name '*.rs' 2>/dev/null || true)
    files=${#rs[@]}
    [[ "$files" -eq 0 ]] && continue
    lines=$(cat "${rs[@]}" 2>/dev/null | wc -l | tr -d ' ')
    printf '%s\t%s\t%s\n' "$lines" "$files" "$name"
  done | sort -t$'\t' -nr -k1,1 | head -10
)

PLAN="$ROOT/docs/重构/模块化重构执行计划-v2.md"
V2_DONE=0; V2_TODO=0
if [[ -f "$PLAN" ]]; then
  V2_DONE=$(grep -c '状态：`完成`' "$PLAN" || true)
  V2_TODO=$(grep -c '状态：`未完成`' "$PLAN" || true)
fi
V2_TOTAL=$((V2_DONE + V2_TODO))

TS=$(date '+%Y-%m-%d %H:%M:%S %z')
GIT_HEAD=$(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')
GIT_DIRTY=$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ' || true)
[[ -z "$LABEL" ]] && LABEL="快照 ${TS}"

BASELINE_ACTIONS_LINES=106713
CUM_ACTIONS_DOWN=$((BASELINE_ACTIONS_LINES - ACTIONS_LINES))
PCT_DOWN=$(python3 -c "print(f'{100.0 * $CUM_ACTIONS_DOWN / $BASELINE_ACTIONS_LINES:.2f}')")

SNAP_FILE="$ROOT/docs/重构/模块化重构度量快照.md"

# 解析上批「实际工作队列」数字（新格式优先；旧格式回退纯 rules）
parse_prev_work() {
  python3 - "$SNAP_FILE" "$PREV_LABEL" "$LABEL" <<'PY'
import re, sys
from pathlib import Path
path, want, cur_label = Path(sys.argv[1]), sys.argv[2].strip(), sys.argv[3].strip()
if not path.is_file():
    print("0 0 0 0 -")
    raise SystemExit(0)
text = path.read_text(encoding="utf-8")
blocks = re.split(r"\n### 度量快照：", text)[1:]
cands = []
for b in blocks:
    lines = b.splitlines()
    if not lines: continue
    label = lines[0].strip()
    body = "\n".join(lines[1:])
    # 新表：| 队列项 | 目标 | 上批 | 本批 | 本批Δ | … |
    # 取「本批」列（第 4 个 文件/行 对，或最后一个 **N / M**）
    def current_pair(label_pat: str):
        row = re.search(
            rf"\|[^\n]*{label_pat}[^\n]*\|",
            body,
        )
        if not row:
            return 0, 0
        pairs = re.findall(r"(\d+)\s*/\s*(\d+)", row.group(0))
        # 目标, 上批, 本批, 本批Δ → 取本批（index 2）若不足则取最后一个非 0/0 的
        if len(pairs) >= 3:
            return int(pairs[2][0]), int(pairs[2][1])
        if pairs:
            return int(pairs[-1][0]), int(pairs[-1][1])
        return 0, 0

    lf, ll = current_pair(r"本地纯 rules")
    tf, tl = current_pair(r"(?:orch 透传壳|应删 orch)")
    # 旧格式回退
    if ll == 0:
        m = re.search(r"其中纯规则 rules 文件/行\s*\|\s*(\d+)\s*/\s*(\d+)", body)
        if m:
            lf, ll = int(m.group(1)), int(m.group(2))
    cands.append((label, lf, ll, tf, tl))
while cands and (cands[-1][0] == cur_label or cur_label in cands[-1][0]):
    cands.pop()
if want:
    for c in reversed(cands):
        if c[0] == want or want in c[0]:
            print(f'{c[1]} {c[2]} {c[3]} {c[4]} {c[0].replace(" ", "·")}')
            raise SystemExit(0)
if not cands:
    print("0 0 0 0 -")
else:
    c = cands[-1]
    print(f'{c[1]} {c[2]} {c[3]} {c[4]} {c[0].replace(" ", "·")}')
PY
}

read -r PREV_LF PREV_LL PREV_TF PREV_TL PREV_LBL <<<"$(parse_prev_work)"
PREV_LBL_DISP=${PREV_LBL//·/ }
[[ "$PREV_LBL_DISP" == "-" || -z "$PREV_LBL_DISP" ]] && PREV_LBL_DISP="（无上批工作队列快照）"

D_LF=$((LOCAL_RULES_FILES - PREV_LF))
D_LL=$((LOCAL_RULES_LINES - PREV_LL))
D_TF=$((ORCH_THIN_FILES - PREV_TF))
D_TL=$((ORCH_THIN_LINES - PREV_TL))

# 主队列域表
QUEUE_DOMAIN_TABLE=""
if [[ -n "${QUEUE_DOMAIN:-}" ]]; then
  IFS=',' read -ra parts <<<"$QUEUE_DOMAIN"
  for part in "${parts[@]}"; do
    [[ -z "$part" ]] && continue
    name=${part%%=*}
    lines=${part#*=}
    QUEUE_DOMAIN_TABLE+="| ${name} | ${lines} |"$'\n'
  done
fi

# 主队列文件明细（最多 20）；格式 path=lines|path=lines
QUEUE_FILE_TABLE=""
if [[ -n "${QUEUE_DETAIL:-}" ]]; then
  IFS='|' read -ra parts <<<"$QUEUE_DETAIL"
  i=0
  for part in "${parts[@]}"; do
    [[ -z "$part" ]] && continue
    path=${part%%=*}
    lines=${part#*=}
    QUEUE_FILE_TABLE+="| \`${path}\` | ${lines} |"$'\n'
    i=$((i + 1))
    [[ $i -ge 20 ]] && break
  done
fi

# 主队列目标：0 文件 / 0 行
TARGET_LOCAL_FILES=0
TARGET_LOCAL_LINES=0
REMAIN_LOCAL_LINES=$LOCAL_RULES_LINES
# 若有上批，本批清了多少
CLEARED_THIS=$((PREV_LL - LOCAL_RULES_LINES))

REPORT=$(
  cat <<EOF
### 度量快照：${LABEL}

| 元数据 | 值 |
|--------|----|
| 时间 | ${TS} |
| Git HEAD | \`${GIT_HEAD}\`（工作区未提交变更 ${GIT_DIRTY} 条） |
| V2 条目 完成/未完成/合计 | ${V2_DONE} / ${V2_TODO} / ${V2_TOTAL} |
| 对比上批 | ${PREV_LBL_DISP} |

> **主 KPI 说明**：不再用 actions 总行（~10 万，含测试与 Host 壳）吓人。
> **实际要处理** = 仍留在 runtime 的**本地纯规则** + 应删的 **orch 透传壳**。
> Host 适配 rules / IO rules / 结构 mod 壳 / AppState 编排 = 观察或可保留，不进主队列。

#### ① 实际工作队列（主 KPI）

| 队列项 | 目标 | 上批 | 本批 | 本批Δ | 剩余含义 |
|--------|------|------|------|-------|----------|
| **本地纯 rules（主队列）文件/行** | **${TARGET_LOCAL_FILES} / ${TARGET_LOCAL_LINES}** | ${PREV_LF} / ${PREV_LL} | **${LOCAL_RULES_FILES} / ${LOCAL_RULES_LINES}** | **$(fmt_delta "$D_LF") / $(fmt_delta "$D_LL")** | 未迁入 orchestrator 的纯规则，A2 优先清空 |
| **应删 orch 透传壳 文件/行** | **0 / 0** | ${PREV_TF} / ${PREV_TL} | **${ORCH_THIN_FILES} / ${ORCH_THIN_LINES}** | **$(fmt_delta "$D_TF") / $(fmt_delta "$D_TL")** | 无本地逻辑、仅 reexport orchestrator |
| 已委托 orch 的 Host 适配 rules | 可保留 | — | ${ADAPTER_RULES_FILES} / ${ADAPTER_RULES_LINES} | — | 半完成适配，非主队列 |
| 含 IO 的 rules（Host） | 可保留 | — | ${IO_RULES_FILES} / ${IO_RULES_LINES} | — | 读库/进度等适配 |
| 结构 mod 壳 | 非迁移目标 | — | ${STRUCT_SHELL_FILES} / ${STRUCT_SHELL_LINES} | — | 目录分层，不算债 |

**主队列合计（本地纯 rules + orch 透传壳）**：本批 **$((LOCAL_RULES_LINES + ORCH_THIN_LINES)) 行** / **$((LOCAL_RULES_FILES + ORCH_THIN_FILES)) 文件**；目标 **0**。

主队列按域（本地纯 rules 行）：

| 域 | 行 |
|----|----|
${QUEUE_DOMAIN_TABLE}
主队列文件明细（本地纯 rules，最多 20）：

| 路径 | 行 |
|------|----|
${QUEUE_FILE_TABLE}
#### ② 噪声背景（勿作主 KPI）

| 项 | 值 | 说明 |
|----|-----|------|
| runtime actions 总行（含测试） | ${ACTIONS_LINES} | 3.10 文档基线 ${BASELINE_ACTIONS_LINES}；累计已降 ${CUM_ACTIONS_DOWN}（${PCT_DOWN}%）— **含 Host 壳，关闭条件不是压到 0** |
| runtime actions 文件/行（排除 tests） | ${ACTIONS_PROD_FILES} / ${ACTIONS_PROD_LINES} | 生产体量背景 |
| 引用 AppState 的 actions 文件/命中 | ${APPSTATE_FILES} / ${APPSTATE_HITS} | Host 编排会长期存在 |
| orchestrator 文件/行 | ${ORCH_FILES} / ${ORCH_LINES} | 业务迁入后上升属预期 |
| actions \`.orchestrator()\` / 生产 runtime | ${ORCH_CALL_ACTIONS} / ${ORCH_CALL_RUNTIME} | 目标 0（除结构测试） |
| V2 完成 | ${V2_DONE} / ${V2_TOTAL} | 未完成 ${V2_TODO} |

actions 一级域 Top 10（总行，噪声）：

| 域 | 行数 | 文件数 |
|----|------|--------|
${TOP_DOMAINS}
生成命令：\`scripts/checks/refactor-metrics.sh --label "${LABEL}"\`
EOF
)

printf '%s\n' "$REPORT"

if [[ "$APPEND" -eq 1 ]]; then
  SNAP="$SNAP_FILE"
  if [[ ! -f "$SNAP" ]]; then
    cat >"$SNAP" <<'HDR'
# 模块化重构度量快照

本文件由 `scripts/checks/refactor-metrics.sh --append` 追加写入。

## 主 KPI（读这里）

**实际工作队列** = runtime 内仍本地的纯 `rules.rs` + 应删除的 orchestrator 透传壳。
**不是** actions 总行（约 10 万，含测试与 Host 适配，只作噪声背景）。

每批必须能直接读出：目标 0、上批、本批、本批Δ、剩余主队列行数。

```bash
scripts/checks/refactor-metrics.sh --label "批次 NNN 完成后" --prev-label "批次 MMM 完成后" --append
```

---

HDR
  fi
  {
    printf '\n'
    printf '%s\n' "$REPORT"
    printf '\n---\n'
  } >>"$SNAP"
  echo "已追加到 docs/重构/模块化重构度量快照.md" >&2
  echo "主队列: 本地纯 rules ${LOCAL_RULES_FILES}/${LOCAL_RULES_LINES}；orch 透传壳 ${ORCH_THIN_FILES}/${ORCH_THIN_LINES}" >&2
fi
