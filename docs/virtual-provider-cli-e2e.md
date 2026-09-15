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

## 导演时间线 V3

`run_director_timeline_cli_e2e.py` 验证图片点与镜头段独立、revision 冲突、图文候选生成与显式采用、参考图字节、取消、拍摄剧本投影、不同镜头时长的视频生成，以及包含图片与场景引用的 ZIP 往返。它只新建隔离数据，删除检查仅针对本次生成且仍被引用的资源。

沿用前面的隔离 Server 和账号；Provider 用 `--video-file` 指定 6 秒 MP4 fixture。执行：

```bash
python3 scripts/tests/run_director_timeline_cli_e2e.py \
  --a4n "${CARGO_TARGET_DIR:-target}/debug/a4n" \
  --server "$A4N_TEST_SERVER_URL" --username "$A4N_USERNAME" \
  --provider-url http://127.0.0.1:18765/v1 \
  --provider-log data/director-e2e/provider.jsonl \
  --report data/director-e2e/report.json
```

`frontend/e2e/director-timeline/check.mjs` 使用真实生产组件与 HTTP fixture，覆盖播放、拖动图片点／镜头边界、自动保存、整体伸缩、弹窗、窄屏与切换章节时的未完成请求。它不能替代上面的真实后端链路。

## LLM 调用生命周期回归

`run_llm_lifecycle_cli_e2e.py` 验证 Canvas 个人配置隔离、单模型并发为 1 时可完成、截断与缺失结束信号被拒绝、运行中取消，以及 Agent 的时长预算和时钟溢出失败状态。使用专用数据库与目录，并保持独立 mock Provider 运行：

```bash
python3 scripts/tests/mock_openai_provider.py --host 127.0.0.1 --port 18765 \
  --log data/cli-contract-e2e/llm-provider.jsonl

# 在另一终端运行；密码从环境变量读取。
python3 scripts/tests/run_llm_lifecycle_cli_e2e.py \
  --a4n "${CARGO_TARGET_DIR:-target}/debug/a4n" \
  --server "$A4N_TEST_SERVER_URL" --username "$A4N_USERNAME" \
  --password-env A4N_PASSWORD --provider-url http://127.0.0.1:18765/v1 \
  --provider-log data/cli-contract-e2e/llm-provider.jsonl \
  --report data/cli-contract-e2e/llm-lifecycle.json
```

## 工具驱动的导演 Agent 试点

`run_specialized_agent_cli_e2e.py` 会新建项目、章节、草稿和 Agent Thread，只在专用测试数据库与目录中运行。沿用上面的 Server/CLI 构建及账号配置：

```bash
python3 scripts/tests/run_specialized_agent_cli_e2e.py \
  --server "$A4N_TEST_SERVER_URL" \
  --username "$A4N_USERNAME" \
  --password-env A4N_PASSWORD \
  --report "$A4N_E2E_ARTIFACT_DIR/cli/report.json"
```

它验证原调用恢复、审批、局部修改、noop、版本冲突、按需工具发现、大结果读取、Thread 隔离、预算和编辑器离线；首模型请求检查未请求正文/方案/Memory 的哨兵内容。现在还包含一条从无草稿 starter 开始的导演 Agent 纵向流程：Agent 通过 typed operation 插入图片点、绑定参考图、两次切分镜头、配置场景转场并伸缩总时长；脚本随后以用户身份确认方案，并断言拍摄脚本保留三段镜头 ID 和时长。可选的 `--fault-database-url-env A4N_FAULT_TEST_DATABASE_URL --postgres-container a4n-postgres` 只接受名为 `a4n_agent_<hex>` 的隔离库，并仅修改本次新建 Run 的持久化记录，模拟消息或回执缺口。它不模拟真实进程终止。

UI E2E 复用成功报告中的资源。Provider 必须在整个 UI 测试期间运行；可以先单独启动 `mock_openai_provider.py --host 127.0.0.1 --port 18765 --log "$A4N_E2E_PROVIDER_LOG"`，并向 CLI runner 传入 `--provider-url http://127.0.0.1:18765/v1 --provider-log "$A4N_E2E_PROVIDER_LOG"`。测试 Host 的公共 LLM 模板也应指向这个隔离 Provider。

设置以下环境后，在 `frontend/` 运行已有 Chromium 配置：

```bash
export A4N_SPECIALIZED_FIXTURE="$A4N_E2E_ARTIFACT_DIR/cli/report.json"
export A4N_SPECIALIZED_SERVER="$A4N_TEST_SERVER_URL"
export A4N_BROWSER_E2E_BASE_URL="$A4N_TEST_WEB_URL"
export A4N_BROWSER_E2E_USERNAME="$A4N_USERNAME"
export A4N_BROWSER_E2E_PASSWORD="$A4N_PASSWORD"
export A4N_BROWSER_E2E_ARTIFACT_DIR="$A4N_E2E_ARTIFACT_DIR/browser"
node node_modules/@wdio/cli/bin/wdio.js run \
  ./e2e/full-project/wdio.conf.mjs \
  --spec ./e2e/specialized-agent/specialized-agent.e2e.mjs
```

原生 Tauri 验证复用同一 spec 的首个实际编辑器流程，观察状态通过原 typed API 的 native invoke 取得。先停止使用该隔离库的测试 Server，避免两个 Host 抢占同一 Run；为 Tauri 配置独立 data/cache/temp/log/portable 目录，并让 Vite 与 Tauri 的 devUrl 一致、二者均使用 `VITE_A4N_API_TRANSPORT=tauri`。编译 `a4n-tauri --features embedded-capabilities` 后，在同一隔离 Host 环境启动 tauri-driver：

```bash
tauri-driver --port 4444 --native-port 4445 --native-driver /usr/bin/WebKitWebDriver
```

另一个终端保留上述浏览器测试变量，再设置 `A4N_SPECIALIZED_NATIVE=1`、`A4N_WDIO_APP_BINARY` 为实际 Tauri 二进制绝对路径，以及 `A4N_TAURI_DRIVER_PORT=4444`，运行 `node node_modules/@wdio/cli/bin/wdio.js run ./e2e/specialized-agent/wdio.native.conf.mjs`。这里直接使用标准 WebDriver，不要求安装 WDIO Tauri 插件。Web 与 native 报告分别写入 `specialized-agent-report.json` 和 `specialized-agent-native-report.json`；运行失败不能当作通过。
