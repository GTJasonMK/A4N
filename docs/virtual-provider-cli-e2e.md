# 虚拟 Provider CLI E2E

这条 smoke 链路使用 `scripts/tests/mock_openai_provider.py`，但真实经过 CLI、Server、Runtime、Orchestrator、数据库和任务/工作流 API。

## 前置条件

Server 必须编译 runtime snapshot 使用的 HTTP Provider 实现：

```bash
cargo build -p a4n-server --features embedded-capabilities
cargo build -p a4n-cli
```

按正常数据库/JWT 配置启动该 Server。runner 需要已有用户，密码只从 `--password-env` 指定的环境变量读取。
完整的协作路径还需要第二个已有用户；通过 CLI 注册后，使用
`--collaborator-username` 和 `--collaborator-password-env` 传入，runner 会实际登录该账号并接受项目邀请。

Provider URL 由 **Server** 访问，不是由 CLI 访问。runner 默认启动的 loopback Provider 只适用于 Server 能访问同一 `127.0.0.1` 的部署。远程 Server 必须使用它可达的私有测试地址单独启动 Provider，再通过 `--provider-url http://<server-reachable-host>:18765/v1` 传入；CLI 本机的 `127.0.0.1` 对远程 Server 无效。

## 运行

```bash
python3 scripts/tests/test_mock_openai_provider.py

python3 scripts/tests/run_cli_contract_e2e.py \
  --a4n target/debug/a4n \
  --server http://127.0.0.1:8080/api \
  --username "$A4N_USERNAME" \
  --password-env A4N_PASSWORD \
  --ledger scripts/tests/cli_contract_scenarios.json \
  --report-dir data/cli-contract-e2e/smoke
```

核心真实业务链路：

```bash
python3 scripts/tests/run_cli_contract_e2e.py \
  --a4n target/debug/a4n \
  --server http://127.0.0.1:8080/api \
  --username "$A4N_USERNAME" \
  --password-env A4N_PASSWORD \
  --ledger scripts/tests/cli_contract_core_workflow.json \
  --report-dir data/cli-contract-e2e/core
```

常用业务路径 fixture（只执行真实写入，不再混入 manifest 探针）：

```bash
python3 scripts/tests/run_all_workflows_e2e.py \
  --a4n target/debug/a4n \
  --server http://127.0.0.1:8080/api \
  --username "$A4N_USERNAME" \
  --password-env A4N_PASSWORD \
  --collaborator-username "$A4N_COLLABORATOR_USERNAME" \
  --collaborator-password-env A4N_COLLABORATOR_PASSWORD \
  --execute-writes \
  --allow-destructive \
  --fixture-only \
  --report data/cli-contract-e2e/common-workflows/report.json
```

该模式只允许在本次运行专用的数据库、文件目录和 Provider 上使用。报告中的
`fixtures.paths` 会逐条列出小说创作、章节分析、RAG、素材、画布、派生项目、Galgame、漫画、视频和协作路径；
任一关键命令没有成功执行都会使测试失败。

全 manifest 调用验证：

```bash
python3 scripts/tests/run_all_workflows_e2e.py \
  --a4n target/debug/a4n \
  --server http://127.0.0.1:8080/api \
  --username "$A4N_USERNAME" \
  --password-env A4N_PASSWORD \
  --report data/cli-contract-e2e/manifest/report.json
```

该 runner 动态读取当前 manifest 并调用全部 descriptor。默认对 write 命令使用无效 probe，
所以不会在共享数据库中创建/修改业务记录；destructive 命令默认停在 CLI 授权门禁，
需要授权后路由时才在隔离 fixture 中添加 `--allow-destructive`。报告中的 `business_success` 和
`destructive_success` 是真实成功请求，`expected_state_rejection` 与 `permission_rejection` 是保留
退出码和错误合同的前置条件/权限拒绝，`real_failure` 或 `missing` 非零才表示测试失败。只有使用隔离 fixture 时才添加
`--execute-writes` 或 `--allow-destructive`。

历史管理员隔离运行结果为：`385/385` descriptor 已调用，`missing=0`、
`real_failure=0`，其中普通业务成功 `158`、破坏性成功 `9`、状态前置条件拒绝 `213`、
权限拒绝 `5`。这证明当时的调用链和错误合同完整，不表示 385 个命令都拥有成功业务 fixture；当前命令数以报告中的
`manifest_commands` 为准。

只做 manifest 分类、不登录也不启动 Provider 时，增加 `--inventory-only`。

core runner 输出 `coverage.json`、`coverage.md`、Provider 请求 JSONL 和进程日志；manifest
runner 输出一个 JSON 报告和 Provider 请求 JSONL。不要把小型 ledger 的 scenario/skip 分类
当成全量覆盖；全量调用以 manifest runner 的 `invoked == manifest_commands` 为准，业务成功
路径以核心 workflow 和 `--fixture-only` 的 `fixtures.paths` 为准。

核心 runner 的 `success` 只表示该 ledger 声明的场景全部通过；`classified_complete` 和
`tested_complete` 会继续明确报告它是否覆盖当前 manifest。核心账本是部分业务 fixture 时，
`success=true` 且 `complete=false` 是正常结果，不应把它误读为所有 descriptor 都已测试。
