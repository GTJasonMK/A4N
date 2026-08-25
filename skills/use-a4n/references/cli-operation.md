# A4N CLI operation

## Contents

- Startup and diagnosis
- Authentication
- Agent registration and heartbeats
- Context-efficient capability discovery
- Project and chapter context
- Tasks and workflows
- File input and output
- Multi-step automation
- Exit codes

## Startup and diagnosis

Default Server: `http://127.0.0.1:8080/api`.

Use `--server URL` or `A4N_SERVER_URL` for another Server. Reject URLs containing credentials, query parameters, or fragments.

The default CLI config is `${XDG_CONFIG_HOME:-$HOME/.config}/a4n/cli.json` on Linux,
`%APPDATA%/A4N/cli.json` on Windows, or the path supplied by `A4N_CLI_CONFIG`. An explicit
`--config PATH` selects the file for that invocation and should be reused for Agent registration
and tracked work.

```bash
a4n --output agent-json health
a4n --output agent-json capabilities check
```

These two checks apply to local and remote Servers. Use `doctor` and `local status` only for a local deployment:

```bash
a4n --output agent-json doctor
a4n local status
```

`doctor` intentionally requires local PostgreSQL/JWT/runtime prerequisites, so do not use it as a gate for a healthy remote Server. For source-workspace local use, PostgreSQL must already be running and `A4N_DATABASE_URL` plus `A4N_JWT_SECRET` must be set. Empty Web databases also need the three `A4N_BOOTSTRAP_ADMIN_*` values. Managed/full AI profiles require the pinned ONNX Runtime. `a4n local start` runs migration and Server/sidecars but does not start PostgreSQL or Docker.

Choose the local profile deliberately:

- Default `a4n local start` starts the managed sidecar profile and tries to use or start local Qdrant.
- `a4n local start --no-sidecars --no-qdrant` uses externally managed sidecars and Qdrant; set `A4N_SIDECAR_AUTH_TOKEN` and a reachable `A4N_QDRANT_URL` first. This does not disable sidecar capabilities.
- `a4n local start --no-qdrant` is valid when an external Qdrant is already reachable. Managed/full profiles require `A4N_QDRANT_URL` in this mode.
- `a4n local start --full` builds the embedded-capabilities Server from the A4N workspace. Use an ancestor workspace directory or set `A4N_SOURCE_ROOT`; it is not a release-package fallback and is mutually exclusive with `--no-sidecars`.
- When the CLI must build managed sidecars or the full profile from source, install the pinned ONNX Runtime first with `./scripts/dev/setup-onnxruntime.sh`.

The bare `a4n local` command starts the default profile and enters an interactive Shell, so do not use it with `--agent` or `--no-input`. `local stop` only stops processes owned by the current CLI state; it does not stop external Server, Qdrant, sidecars, or Cargo watchers.

Useful lifecycle commands:

```bash
a4n local start
a4n local logs --lines 100
a4n local stop
```

## Authentication

```bash
printf '%s\n' "$A4N_PASSWORD" | a4n auth login --username USER --password-stdin
a4n --output agent-json auth status
a4n auth sessions
a4n auth logout
```

Use stdin for passwords, password changes, and OAuth codes. `A4N_ACCESS_TOKEN` is a read-only environment authentication source; unset it before changing the stored session.

## Page CLI：跟随当前 GUI 页面

普通 CLI 适合不依赖浏览器页面的可复现 Server 工作流；Page CLI 适合 Agent 必须像用户一样操作当前 Web tab 的场景。Page CLI 读取当前路由、项目/章节、Tab、选择项和 dirty state，只暴露当前页面允许的 Host API 命令，并把运行、暂停、取消和导航状态显示在页面控制条中。它不是另一套业务 API，也不能绕过普通命令的权限或合同。

使用前在 Web 设置的账户区域启用“页面 CLI”。该开关默认关闭；关闭时 Server 禁止创建、授权、绑定、导航和 Page Operation/Host API 调用；已有 Session 仍可读取并执行 takeover/detach，便于安全清理。当前 Web tab 在 HTTP 模式会为“用户 + client + tab”复用一个 Page Session，因此 React 重渲染不会产生多个可见会话。

先用同一个 CLI 配置注册 Agent，再从当前页面控制条读取一次性绑定码。绑定码只用于一次绑定，不能写进 prompt、日志、普通输出或 Agent envelope：

```bash
CFG=/path/to/a4n-cli.json
a4n --config "$CFG" --output agent-json agent register \
  --name '当前页面操作员' --kind automation --client cli
printf '%s\n' "$A4N_PAGE_CODE" | \
  a4n --config "$CFG" --output agent-json page attach --code-stdin
```

优先使用 `--code-stdin`，避免绑定码进入 shell history；小规模本地交互也可以使用 `page attach --code CODE`。Attach 的普通输出只返回 Session 摘要，不返回 UI Session token；Agent token 和 UI Session token 只保存在必要的 CLI 配置中。

绑定后先读取页面事实，再调用命令：

```bash
a4n --config "$CFG" --output agent-json page status
a4n --config "$CFG" --output agent-json page context
a4n --config "$CFG" --output agent-json page commands
a4n --config "$CFG" --output agent-json page call list_projects --args '{}'
```

`page call` 会重新读取 Session 和当前命令 manifest，拒绝页面未暴露或暂不可用的命令，检查当前上下文/权限，并先创建可见的 Page Operation，再调用 Host API，最后回写成功或失败状态。参数必须是 JSON object；危险命令必须显式传 `--allow-destructive`，且仍会经过 Server 二次门禁。命令参数中的 secret target 会在 Page Operation 展示值中脱敏，真实 secret 只进入必要的 Host API 请求。

Agent 可以请求当前页面导航到站内路由；页面收到 SSE 后同步路由和上下文：

```bash
a4n --config "$CFG" --output agent-json page navigate /novel/42/overview \
  --page-kind novel-detail --reason '读取项目概览'
```

页面协作控制命令如下：

- `page pause` / `page resume`：暂停或恢复当前 Page Agent。
- `page cancel`：取消当前操作，但保留 Agent 绑定。
- `page takeover`：用户接管页面，取消活动操作并解除 Agent 绑定；GUI 控制条仍可直接使用。
- `page detach`：解除绑定并清理本地 Page Session 凭证。
- `page events`：以 `jsonl` 或其他机器可读输出监听当前用户的 Page Session SSE 事件。

Page CLI 的 token、绑定码和普通 Agent lease 都是不同凭证：不要互相替代，也不要复制到业务参数、日志、提示词或普通输出。需要用户继续协作时，优先让用户点击页面控制条的“用户接管”；需要完全自动化时才持续使用 Page CLI。

## Agent registration and heartbeats

Use the Agent commands when a CLI process is doing work that should be visible in the Tauri/Web Agent workbench. Registration requires an authenticated Server session. The CLI stores the returned Agent ID and private lease token in its local CLI config. If registration uses `--config PATH`, every later tracked command must use the same `--config PATH`, or all commands must use the same `A4N_CLI_CONFIG`; `--context PATH` does not carry the Agent lease. Do not copy the token into scripts, prompts, or command arguments.

For ordinary machine-readable automation, use `--output agent-json` (or `--output jsonl` for event streams). `--agent` is more than an output format: it also disables input and, for commands such as `call`, `task watch`, and `workflow watch`, starts automatic Agent tracking. Register an Agent first when that visibility is wanted; those tracked commands require the saved Agent lease and report monitoring failures instead of running untracked.

Automatic tracking covers activity inside the current CLI process. Use the explicit commands below only when a separate long-lived worker needs to own and maintain its Agent lease.

```bash
a4n --config /path/to/a4n-cli.json --output agent-json agent register \
  --name '上一章漫画转换器' \
  --kind media \
  --client cli \
  --project-id 42 \
  --chapter-id 3

a4n --config /path/to/a4n-cli.json --output agent-json agent update \
  --status running \
  --phase '生成分镜' \
  --summary '正在把上一章拆成漫画镜头' \
  --progress 20

a4n --config /path/to/a4n-cli.json --output agent-json agent heartbeat \
  --status running \
  --phase '生成图片' \
  --progress 65
```

Send a heartbeat at least every 20 seconds during a long operation. The Server lease lasts 30 seconds; when it expires, the entry becomes `offline`. Valid statuses are `idle`, `running`, `paused`, `completed`, `failed`, and `offline`. Inspect all Agents with `a4n --output agent-json agent status`; unregister the saved Agent after the work is no longer useful to the UI:

```bash
a4n --config /path/to/a4n-cli.json --output agent-json agent update --status completed --progress 100 --summary '漫画素材已生成'
a4n --config /path/to/a4n-cli.json --output agent-json agent unregister
```

Use `a4n --config /path/to/a4n-cli.json --output agent-json agent update --agent-id ID ...` only when operating the Agent ID already associated with the saved local lease token. `agent status` is read-only and does not require an Agent token.

## Context-efficient capability discovery

```bash
a4n --output agent-json commands chapter
a4n --output agent-json capabilities describe start_chapter_generation
a4n --output agent-json call list_projects --args '{"page":1,"pageSize":50}'
a4n --output agent-json call start_chapter_generation --args @start-chapter.json
```

Use `commands KEYWORD` to find candidate names without loading every schema. Then inspect only the chosen descriptor. Prefer its typed schema and check `effect`, `execution`, `required_capabilities`, `idempotency`, file hints, secret pointers, and result references.

Use filtered `capabilities list` only when several descriptors must be compared. Do not use `capabilities export` for ordinary task discovery.

## Project and chapter context

```bash
CTX=/path/to/a4n-context.json
a4n --context "$CTX" project list
a4n --context "$CTX" project create --title '项目名' --project-type novel_long --use-context
a4n --context "$CTX" project use 42
a4n --context "$CTX" chapter list
a4n --context "$CTX" chapter import --number 1 --title '第一章' --content @chapter.txt
a4n --context "$CTX" chapter use 100
a4n --context "$CTX" context show
```

Valid text project types are `novel_long`, `novel_short`, `script`, and `prose`. Switch project context before chapter context; switching either clears narrower stale context.

## Tasks and workflows

Use these observers only when the initiating response is non-terminal. A command may record a workflow internally while still waiting and returning its final proposal/result synchronously.

```bash
a4n --output jsonl task watch TASK_ID --attempt 1
a4n --output agent-json task active --project-id 42
a4n --output agent-json task history --project-id 42 --limit 50
a4n --output jsonl workflow watch WORKFLOW_ID --interval-ms 1000
a4n --output agent-json workflow show WORKFLOW_ID
a4n --output agent-json workflow resume WORKFLOW_ID
```

Cancel only with explicit approval:

```bash
a4n --output agent-json task cancel TASK_ID --attempt 1 --yes
a4n --output agent-json workflow cancel --workflow-run-id WORKFLOW_ID --yes
```

## File input and output

```bash
a4n --output agent-json call import_project_text_upload \
  --args @import-project.json \
  --set-base64 /input/data_base64=book.epub

a4n --output agent-json call export_epub_data \
  --args @export.json \
  --save-base64 /data_base64=book.epub

a4n media download generated/video.mp4 --output ./video.mp4
```

Outputs are atomic and refuse overwrite by default. Do not use `--force` casually.

## Multi-step automation

Compose ordinary scripts from `--output agent-json call`, task/workflow observation, and media download commands. Use `--agent` only after registering an Agent for workbench tracking. Persist returned IDs and input files outside shell history. Before retrying an interrupted non-idempotent call, query Server state and inspect the descriptor's `idempotency` field.

Load [agent-recipes.md](agent-recipes.md) for the minimal project creation, generation-task, and artifact-export control flows.

For CLI/Server/provider coverage and manifest-wide probes, load [cli-testing.md](cli-testing.md). Keep its distinction between successful business workflows and expected prerequisite errors.

## Exit codes

| Code | Meaning |
| ---: | --- |
| 0 | success |
| 2 | invalid local input |
| 3 | unauthenticated |
| 4 | forbidden |
| 5 | not found |
| 6 | conflict/stale resource |
| 7 | network, timeout, rate limit, temporary unavailable |
| 8 | business error or failed task/workflow |
| 9 | cancelled/interrupted |
| 10 | destructive confirmation missing |
| 11 | CLI/Server capability manifest incompatible |

If the Agent envelope reports `invalid_server_error_contract`, stop instead of inferring missing fields or retrying: the Server returned neither the shared Host API error contract nor the Generation error contract.
