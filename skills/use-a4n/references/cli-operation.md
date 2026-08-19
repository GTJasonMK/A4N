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

```bash
a4n --agent health
a4n --agent capabilities check
```

These two checks apply to local and remote Servers. Use `doctor` and `local status` only for a local deployment:

```bash
a4n --agent doctor
a4n local status
```

`doctor` intentionally requires local PostgreSQL/JWT/runtime prerequisites, so do not use it as a gate for a healthy remote Server. For source-workspace local use, PostgreSQL must already be running and `A4N_DATABASE_URL` plus `A4N_JWT_SECRET` must be set. Empty Web databases also need the three `A4N_BOOTSTRAP_ADMIN_*` values. Managed/full AI profiles require the pinned ONNX Runtime. `a4n local start` runs migration and Server/sidecars but does not start PostgreSQL or Docker.

Useful lifecycle commands:

```bash
a4n local start
a4n local logs --lines 100
a4n local stop
```

## Authentication

```bash
printf '%s\n' "$A4N_PASSWORD" | a4n auth login --username USER --password-stdin
a4n --agent auth status
a4n auth sessions
a4n auth logout
```

Use stdin for passwords, password changes, and OAuth codes. `A4N_ACCESS_TOKEN` is a read-only environment authentication source; unset it before changing the stored session.

## Agent registration and heartbeats

Use the Agent commands when a CLI process is doing work that should be visible in the Tauri/Web Agent workbench. Registration requires an authenticated Server session. The CLI stores the returned Agent ID and private lease token in its local config; do not copy the token into scripts, prompts, or command arguments.

For normal Agent automation, register an Agent once, then prefer running work commands with `--agent`. The CLI infers a phase from commands such as `call`, `task watch`, and `workflow watch`, sends an internal heartbeat every 10 seconds, and marks the entry `completed` or `failed` when the command exits. This tracking adds HTTP calls only; it does not add heartbeat instructions or model tokens to the Agent prompt. Work commands with `--agent` require the saved Agent lease and report monitoring failures instead of running untracked.

Automatic tracking covers activity inside the current CLI process. Use the explicit commands below only when a separate long-lived worker needs to own and maintain its Agent lease.

```bash
a4n --agent agent register \
  --name '上一章漫画转换器' \
  --kind media \
  --client cli \
  --project-id 42 \
  --chapter-id 3

a4n --agent agent update \
  --status running \
  --phase '生成分镜' \
  --summary '正在把上一章拆成漫画镜头' \
  --progress 20

a4n --agent agent heartbeat \
  --status running \
  --phase '生成图片' \
  --progress 65
```

Send a heartbeat at least every 20 seconds during a long operation. The Server lease lasts 30 seconds; when it expires, the entry becomes `offline`. Valid statuses are `idle`, `running`, `paused`, `completed`, `failed`, and `offline`. Inspect all Agents with `a4n --agent agent status`; unregister the saved Agent after the work is no longer useful to the UI:

```bash
a4n --agent agent update --status completed --progress 100 --summary '漫画素材已生成'
a4n --agent agent unregister
```

Use `a4n --agent agent update --agent-id ID ...` only when operating the Agent ID already associated with the saved local lease token. `agent status` is read-only and does not require an Agent token.

## Context-efficient capability discovery

```bash
a4n --agent commands chapter
a4n --agent capabilities describe start_chapter_generation
a4n --agent call list_projects --args '{"page":1,"pageSize":50}'
a4n --agent call start_chapter_generation --args @start-chapter.json
```

Use `commands KEYWORD` to find candidate names without loading every schema. Then inspect only the chosen descriptor. Prefer its typed schema and check `effect`, `execution`, `required_capabilities`, `idempotency`, file hints, secret pointers, and result references.

Use filtered `capabilities list` only when several descriptors must be compared. Do not use `capabilities export` for ordinary task discovery.

## Project and chapter context

```bash
CTX="$HOME/.config/a4n/context.json"
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
a4n --agent task active --project-id 42
a4n --agent task history --project-id 42 --limit 50
a4n --output jsonl workflow watch WORKFLOW_ID --interval-ms 1000
a4n --agent workflow show WORKFLOW_ID
a4n --agent workflow resume WORKFLOW_ID
```

Cancel only with explicit approval:

```bash
a4n --agent task cancel TASK_ID --attempt 1 --yes
a4n --agent workflow cancel --workflow-run-id WORKFLOW_ID --yes
```

## File input and output

```bash
a4n --agent call import_project_text_upload \
  --args '{"input":{"project_id":42,"file_name":"book.epub"}}' \
  --set-base64 /input/data_base64=book.epub

a4n --agent call export_epub_data \
  --args @export.json \
  --save-base64 /data_base64=book.epub

a4n media download generated/video.mp4 --output ./video.mp4
```

Outputs are atomic and refuse overwrite by default. Do not use `--force` casually.

## Multi-step automation

Compose ordinary scripts from `--agent call`, task/workflow observation, and media download commands. Persist returned IDs and input files outside shell history. Before retrying an interrupted non-idempotent call, query Server state and inspect the descriptor's `idempotency` field.

Load [agent-recipes.md](agent-recipes.md) for the minimal project creation, generation-task, and artifact-export control flows.

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
