# A4N CLI Test Plan

## Baseline

- Preserve every existing one-shot command and option.
- Preserve raw JSON output for existing commands.
- Keep Web/Tauri/CLI command parity at zero missing and zero stale entries.

## Unit test inventory

- `main.rs` / command parsing: output modes, agent mode, destructive gates, context precedence.
- `client.rs`: structured HTTP errors, refresh behavior, timeouts, response preservation.
- `a4n-api-contract/src/command_manifest.rs`: sorted/unique commands, capability filtering and command descriptions.
- `context.rs`: locked atomic persistence, server/user binding, hierarchical clearing.
- `output.rs`: human/json/jsonl/agent-json stdout and error envelopes.
- `shell.rs`: safe tokenization and shared command dispatch.
- `local.rs`: diagnostics, owned/external process state and safe shutdown decisions.

## Integration test inventory

- Mock HTTP server tests for auth, command routing, all documented HTTP exit classes,
  media download, raw requests (inline/file/stdin bodies, all unsafe methods and overwrite
  protection), file input/output and SSE reconnect/chunking/CRLF/multiline/malformed-frame
  handling.
- CLI subprocess tests for capabilities, project/chapter/task/workflow/Agent semantic commands,
  auth stdin flows, config/context persistence, output modes, non-interactive failure,
  destructive authorization, Agent lease tracking and secret redaction. Coverage also includes
  workflow event de-duplication, access-token refresh retry, Agent heartbeat failure headers,
  and empty local status/log/stop plus remote-start gating.
- Page CLI subprocess tests cover session listing, one-time-code attach from argument/stdin,
  status/context/command discovery, manifest availability and destructive gates, operation
  tracking with sensitive-target redaction, navigation, pause/resume/cancel/takeover/detach,
  credential cleanup, request ordering, header isolation and chunked CRLF SSE parsing.
- Manifest-wide CLI smoke invokes every current descriptor through the real Server. Default
  arguments are generated from each descriptor schema; writes and destructive calls therefore
  require an isolated database and `--execute-writes --allow-destructive`. The report records
  successful business calls separately from expected prerequisite/permission rejections and
  fails on missing commands, malformed envelopes, provider failures, or internal errors. The
  Rust suite still verifies destructive gates before network access.
- `run_all_workflows_e2e.py --fixture-only --execute-writes --allow-destructive` runs only the
  isolated business fixture and reports required user paths under `fixtures.paths`; it is the
  primary evidence for successful cross-layer workflows, while the manifest smoke remains the
  command-entry and error-contract probe.
- Real `a4n-server` tests against a migrated PostgreSQL test database.

## End-to-end workflows

- Account login and project creation.
- AI-assisted text project and chapter generation.
- Text import, review and reverse structure.
- Video, manga and GALGame derived-project workflows.
- Canvas execution, source synchronization, export and collaboration.
- Provider configuration, task/workflow watching and RAG maintenance.
- `doctor` and `local` startup with migration, managed sidecars and owned-process cleanup.
- Page CLI attach, current-page context/command execution, visible operation lifecycle,
  navigation and user takeover controls.

## Agent smoke scenario

1. List and describe capabilities as JSON.
2. Create a project and read its ID from structured output.
3. Start a task, consume JSONL events, reconnect and observe the terminal status.
4. Combine `call` with task/workflow watching in a shell script.
5. Verify destructive operations fail without explicit authorization and succeed with it.
6. Download and verify an artifact.

## Output verification

- Parse every JSON/JSONL stdout payload programmatically.
- Verify failed terminal task events return a non-zero exit status.
- Verify exported/downloaded files by size, expected format signature and hash when available.
- Verify secrets never appear in stdout, stderr, history or context files.
- Verify failed overwrites leave the prior file unchanged.

## Release gates

- `timeout 60s cargo test -p a4n-cli -- --test-threads=1`
- `python3 scripts/tests/test_cli_contract_runner.py`
- `python3 scripts/tests/test_mock_openai_provider.py`
- `python3 scripts/tests/run_all_workflows_e2e.py ...` with an isolated full Server and admin
  account.
- `cd frontend && npm run check:web-api-parity`
- `./scripts/build.sh --check`
- CLI subprocess smoke from outside the repository working directory.
- Platform build checks for Linux, macOS and Windows local-process code paths.

## Results

Last focused run: 2026-08-23

```text
$ cargo test -p a4n-cli --bin a4n -- --test-threads=1
test result: ok. 57 passed; 0 failed; 0 ignored

$ cargo test -p a4n-cli --test cli_e2e -- --test-threads=1
test result: ok. 49 passed; 0 failed; 0 ignored

$ cargo test -p a4n-server page_sessions --lib -- --test-threads=1
test result: ok. 7 passed; 0 failed; 0 ignored

$ cd frontend && npm run test:run
Test Files  70 passed (70)
Tests       245 passed (245)

$ cd frontend && npm run build
✓ built successfully

$ python3 scripts/tests/test_cli_contract_runner.py
Ran 23 tests - OK

$ python3 scripts/tests/test_mock_openai_provider.py
Ran 14 tests - OK
```

本轮额外通过的门禁：

- `cargo test -p a4n-api-contract --lib`: 78 passed.
- Linux 与 `x86_64-pc-windows-msvc` 的 `a4n-cli --all-targets --no-deps` clippy 均以
  `-D warnings` 通过；`cargo machete crates/a4n-cli` 未发现未使用依赖。
- `a4n-server --no-default-features` check 通过。
- `npm run check:web-api-parity`: 381 个共享 Host API 命令、33 个 Web/CLI 专用映射、
  13 个 Tauri-only 等价入口全部一致，Web/Desktop/CLI missing/stale 均为 0。
- Bash/Ruby 语法、catalog 驱动的 sidecar inventory、Linux/Windows/macOS release
  manifest 清单及篡改检测全部通过。

此前已完成且仍适用的真实环境验收：

- `a4n-service-protocol`: 69 passed；`a4n-service-manager --lib`: 36 passed。
- PostgreSQL local lifecycle 使用隔离端口、数据库和 CLI state 通过 core-only Server、
  catalog managed sidecar + 外部 Qdrant、CLI-owned Qdrant、embedded/full + 外部 Qdrant 四种模式。
- 完整源码构建在显式取消 `ORT_LIB_PATH` 后通过；`a4n local` 为 Cargo 链接和 Server
  启动同时注入了安装的 ONNX Runtime。
- 真实 PostgreSQL/Server 登录和媒体下载通过，access token 只位于 Authorization
  header，不出现在请求 URL 或 Server 日志中。
- 完整用户流程和最终媒体/包验证记录在
  [`CLI用户操作流程测试报告.md`](../../CLI用户操作流程测试报告.md)。
- `run_cli_contract_e2e.py --ledger cli_contract_core_workflow.json` 通过：13 个核心场景、
  12 个不同 descriptor，真实完成项目创建、LLM 任务、embedding、图片/视频 provider、
  task/workflow watch 和 Markdown 导出。
- 历史 `run_all_workflows_e2e.py` 通过：manifest `385/385` descriptor 已调用，`missing=0`、
  `real_failure=0`；本次 `business_success=158`、`destructive_success=9`，另有
  `expected_state_rejection=213` 和 `permission_rejection=5`。后两类保留了当前错误
  合同和退出码，不计入业务成功。全量授权写入必须使用隔离数据库并同时传入
  `--execute-writes --allow-destructive`。

Coverage gaps:

- GitHub/Google OAuth 仍需要真实第三方凭据和 callback 基础设施。
- Windows Rust cfg/clippy 已通过；PowerShell CIM、`taskkill` 和实际进程树生命周期仍以
  Windows CI 为权威。本 Linux 环境没有 `pwsh`。
- 当前未安装 macOS target；`.app` 的 MacOS/Resources 布局、codesign 与 loader path
  仍以 macOS CI 为权威。
- 本 Linux 主机没有重新生成完整三平台 release archive。
- 商业 provider 的可用性、额度和网络行为仍属于外部系统检查。

## GUI/CLI parity and Agent lifecycle refinement (2026-08-23)

- `cargo test -p a4n-api-contract --lib`: 78 passed.
- `cargo test -p a4n-cli --bin a4n`: 57 passed.
- `cargo test -p a4n-cli --test cli_e2e`: 49 passed.
- `cargo test -p a4n-server page_sessions --lib`: 7 passed.
- `cd frontend && npm run test:run`: 70 files / 245 tests passed.
- `cd frontend && npm run build`: passed.
- `npm run check:web-api-parity`: passed.
- Manifest v12 exposes 385 Host API commands and 13 desktop-runtime equivalents;
  opaque argument/result schemas are both zero.

## Coverage accounting

- The Rust subprocess suite covers the public semantic CLI surface with a deterministic mock
  Server, and its manifest-wide smoke invokes every descriptor reported by the current manifest against a real full
  Server with an administrator session and virtual Provider. The default run reaches the Server
  for non-destructive descriptors and checks the CLI gate for destructive descriptors; use the
  authorized mode only with isolated fixtures. It does not claim that every descriptor has a
  successful business fixture: the report keeps successful calls and expected state/permission
  errors separate.
- `scripts/tests/cli_contract_scenarios.json` remains a reusable virtual-Provider smoke ledger,
  not the manifest-wide test. Its classification is intentionally partial and should not be used
  as the all-command invocation count; use `run_all_workflows_e2e.py` for that count and
  `cli_contract_core_workflow.json` for successful end-to-end business paths.
- No line/branch coverage threshold is configured yet, so test counts must not be reported as a
  percentage coverage result.
