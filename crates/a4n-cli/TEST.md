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
- `flow.rs`: dry-run, journal state, interruption, resume and secret redaction.
- `local.rs`: diagnostics, owned/external process state and safe shutdown decisions.

## Integration test inventory

- Mock HTTP server tests for auth, command routing, error status mapping, media download and SSE reconnect.
- CLI subprocess tests for `--help`, `capabilities`, `--agent`, non-interactive failure and destructive authorization.
- Real `a4n-server` tests against a migrated PostgreSQL test database.

## End-to-end workflows

- Account login and project creation.
- AI-assisted text project and chapter generation.
- Text import, review and reverse structure.
- Video, manga and GALGame derived-project workflows.
- Canvas execution, source synchronization, export and collaboration.
- Provider configuration, task/workflow watching and RAG maintenance.
- `doctor` and `local` startup with migration, managed sidecars and owned-process cleanup.

## Agent smoke scenario

1. List and describe capabilities as JSON.
2. Create a project and read its ID from structured output.
3. Start a task, consume JSONL events, reconnect and observe the terminal status.
4. Interrupt and resume a CLI flow.
5. Verify destructive operations fail without explicit authorization and succeed with it.
6. Download and verify an artifact.

## Output verification

- Parse every JSON/JSONL stdout payload programmatically.
- Verify failed terminal task events return a non-zero exit status.
- Verify exported/downloaded files by size, expected format signature and hash when available.
- Verify secrets never appear in stdout, stderr, history, context files or flow journals.
- Verify failed overwrites leave the prior file unchanged.

## Release gates

- `timeout 60s cargo test -p a4n-cli -- --test-threads=1`
- `cd frontend && npm run check:web-api-parity`
- `./scripts/build.sh --check`
- CLI subprocess smoke from outside the repository working directory.
- Platform build checks for Linux, macOS and Windows local-process code paths.

## Results

Last focused run: 2026-07-31

```text
$ CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 timeout 60s \
    cargo test -p a4n-cli --target-dir /tmp/a4n-cli-target -- --test-threads=1

running 41 tests
test client::tests::api_paths_cannot_escape_the_configured_api_root ... ok
test client::tests::concurrent_clients_reuse_the_single_rotated_token_pair ... ok
test client::tests::environment_access_tokens_cannot_report_a_fake_logout ... ok
test client::tests::extracts_both_api_error_contracts ... ok
test client::tests::failed_config_writes_do_not_mutate_client_state ... ok
test client::tests::logout_clears_local_tokens_even_when_server_rejects_it ... ok
test client::tests::redacts_known_secrets_from_api_errors ... ok
test client::tests::redacts_overlapping_secrets_longest_first ... ok
test client::tests::refreshes_once_retries_and_persists_tokens ... ok
test client::tests::reload_rejects_tokens_without_a_server_owner ... ok
test client::tests::sensitive_field_detection_does_not_hide_token_metrics ... ok
test client::tests::stale_clients_do_not_restore_cleared_tokens ... ok
test client::tests::successful_json_responses_are_redacted_with_request_secrets ... ok
test config::tests::normalizes_server_root_and_existing_api_path ... ok
test config::tests::rejects_credentials_and_non_http_urls ... ok
test context::tests::concurrent_context_updates_fail_instead_of_overwriting ... ok
test context::tests::context_clears_children_and_rejects_other_server ... ok
test context::tests::failed_persistence_does_not_mutate_in_memory_context ... ok
test flow::tests::flow_capture_limits_persisted_results ... ok
test flow::tests::flow_rejects_inline_secrets ... ok
test flow::tests::flow_validates_and_preserves_typed_references ... ok
test flow::tests::journal_preserves_completed_null_results ... ok
test flow::tests::journal_rejects_false_completed_state ... ok
test local::tests::blank_process_output_is_not_treated_as_a_live_process ... ok
test local::tests::bundled_onnxruntime_is_added_to_the_server_loader_path ... ok
test local::tests::installed_sidecars_support_portable_macos_and_system_layouts ... ok
test local::tests::local_endpoints_reject_remote_server_and_parse_postgres ... ok
test local::tests::log_tail_handles_utf8_truncation_and_logs_remain_private ... ok
test local::tests::owned_local_state_reports_stale_and_mismatched_processes ... ok
test local::tests::server_health_requires_the_a4n_meta_identity ... ok
test output::tests::agent_metadata_extracts_stable_references ... ok
test routing::tests::generation_start_adds_client_request_identity ... ok
test routing::tests::generic_commands_use_the_command_endpoint ... ok
test routing::tests::project_list_uses_the_same_query_mapping_as_web ... ok
test shell::tests::shell_tokenizes_without_shell_expansion_and_hides_secrets ... ok
test sse::tests::detects_both_sse_delimiters ... ok
test sse::tests::parses_generation_event_and_ignores_keep_alive ... ok
test tests::call_context_injects_user_and_admin_caller_ids ... ok
test tests::file_json_pointers_create_inputs_and_decode_outputs_without_overwrite ... ok
test tests::inline_json_and_query_values_are_parsed ... ok
test tests::profile_updates_preserve_missing_and_explicit_null ... ok

test result: ok. 41 passed; 0 failed; 0 ignored

running 7 tests
test agent_capabilities_and_safety_gates_work_outside_repository ... ok
test connection_failures_use_the_network_exit_contract ... ok
test failed_forced_download_preserves_existing_file ... ok
test flow_redacts_server_echoed_secrets_and_blocks_unsafe_resume ... ok
test flow_runs_captures_and_rejects_changed_source_on_resume ... ok
test health_rejects_a_non_a4n_server ... ok
test task_and_workflow_terminal_states_use_stable_exit_codes ... ok

test result: ok. 7 passed; 0 failed; 0 ignored
```

本轮额外通过的门禁：

- `a4n-api-contract`: 12 passed.
- Linux 与 `x86_64-pc-windows-msvc` 的 `a4n-cli --all-targets --no-deps` clippy 均以
  `-D warnings` 通过；`cargo machete crates/a4n-cli` 未发现未使用依赖。
- `a4n-server --no-default-features` check 通过。
- `npm run check:web-api-parity`: 381 个共享 Host API 命令、33 个 Web/CLI 专用映射、
  13 个 Tauri-only 等价入口全部一致，Web/Desktop/CLI missing/stale 均为 0。
- Bash/Ruby 语法、8 个 sidecar inventory、Linux/Windows/macOS release manifest 的
  `9 binaries + 3 tools + 1 runtime` 清单及篡改检测全部通过。

此前已完成且仍适用的真实环境验收：

- `a4n-service-protocol`: 69 passed；`a4n-service-manager --lib`: 36 passed。
- PostgreSQL local lifecycle 使用隔离端口、数据库和 CLI state 通过 core-only Server、
  8 managed sidecar + 外部 Qdrant、CLI-owned Qdrant、embedded/full + 外部 Qdrant 四种模式。
- 完整源码构建在显式取消 `ORT_LIB_PATH` 后通过；`a4n local` 为 Cargo 链接和 Server
  启动同时注入了安装的 ONNX Runtime。
- 真实 PostgreSQL/Server 登录和媒体下载通过，access token 只位于 Authorization
  header，不出现在请求 URL 或 Server 日志中。
- 完整用户流程和最终媒体/包验证记录在
  [`CLI用户操作流程测试报告.md`](../../CLI用户操作流程测试报告.md)。

Coverage gaps:

- GitHub/Google OAuth 仍需要真实第三方凭据和 callback 基础设施。
- Windows Rust cfg/clippy 已通过；PowerShell CIM、`taskkill` 和实际进程树生命周期仍以
  Windows CI 为权威。本 Linux 环境没有 `pwsh`。
- 当前未安装 macOS target；`.app` 的 MacOS/Resources 布局、codesign 与 loader path
  仍以 macOS CI 为权威。
- 本 Linux 主机没有重新生成完整三平台 release archive。
- 商业 provider 的可用性、额度和网络行为仍属于外部系统检查。
