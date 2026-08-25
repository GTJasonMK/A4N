# A4N CLI 测试与诊断

只在用户要求验证 CLI、Server、Runtime 或 mock provider 链路时读取本节。测试必须使用当前源码构建的 CLI；不要把旧二进制、静态 manifest 或自然语言输出当作测试证据。

## 两层验证

- 真实业务 workflow 使用 `scripts/tests/run_cli_contract_e2e.py`。推荐先跑 `cli_contract_core_workflow.json`，它会登录、注入 runtime snapshot，执行项目/灵感/导入/蓝图/大纲/章节/导出，并观察 task/workflow。
- manifest 全量 probe 使用 `scripts/tests/run_all_workflows_e2e.py`。它动态导出当前 Server manifest，逐个调用所有 descriptor，并把结果分为 `business_success`、`destructive_success`、`expected_state_rejection`、`permission_rejection` 和 `real_failure`。`invoked == manifest_commands`、`missing == []`、`real_failure == 0` 才表示全量调用链通过；这不等于每个业务命令都拥有成功 fixture。

两条链路都经过真实 CLI 和 Server。provider URL 由 Server 访问；本机 runner 默认启动 loopback mock provider，远程 Server 必须传入 Server 可达的私有 provider URL。

核心 runner 的 `success` 表示账本中声明的场景全部通过；`classified_complete` 和
`tested_complete` 仍单独报告该账本是否覆盖当前 manifest。核心账本可以是有意的部分业务
fixture，因此 `complete=false` 不等于场景失败；需要全量 descriptor 调用时使用 manifest probe。

## 前置条件

```bash
cargo build -p a4n-server --features embedded-capabilities
cargo build -p a4n-cli
```

启动带 embedded provider 能力的 Server，准备隔离 PostgreSQL 测试库和已有管理员账号。密码只能通过环境变量再经 stdin 传给 runner；不要把密码写进 ledger、参数文件或报告。

## 命令

```bash
python3 scripts/tests/run_cli_contract_e2e.py \
  --a4n target/debug/a4n \
  --server http://127.0.0.1:8080/api \
  --username "$A4N_USERNAME" \
  --password-env A4N_PASSWORD \
  --ledger scripts/tests/cli_contract_core_workflow.json \
  --report-dir data/cli-contract-e2e/core

python3 scripts/tests/run_all_workflows_e2e.py \
  --a4n target/debug/a4n \
  --server http://127.0.0.1:8080/api \
  --username "$A4N_USERNAME" \
  --password-env A4N_PASSWORD \
  --report data/cli-contract-e2e/manifest/report.json
```

manifest probe 默认给 write 命令注入无效 probe，避免污染共享数据库；成功写入由核心 workflow 验证。只有数据库、文件和 provider 都是本次运行专用 fixture 时，才使用 `--execute-writes` 或 `--allow-destructive`。

检查 JSON 报告中的 manifest 数量、调用数量、缺失命令、真实失败和 provider 请求路径。默认
destructive 命令的 `route` 是 `cli_destructive_gate`，只验证 CLI 授权门禁；隔离数据库中
再加 `--allow-destructive`，才验证其授权后 Server 路由。`expected_state_rejection` 和
`permission_rejection` 需要保留错误合同和退出码，不能被改写为成功。

## Page CLI 覆盖边界

Page CLI 不能用“mock provider 能响应”代替“GUI Page Session 已绑定”：前者只验证 Server 到 Provider 的 Host API 路径，后者还需要真实的用户、账户开关、当前 Web tab、一次性授权码和 Agent lease。按边界分层验证：

- `crates/a4n-cli/src/page.rs` 单测覆盖授权码校验、Session path 编码、子命令 label 和 LF/CRLF SSE framing。
- `crates/a4n-cli/tests/cli_e2e.rs` 用当前构建的 CLI 和本地 mock HTTP Server 覆盖 `sessions`、`attach --code`/`--code-stdin`、`status`、`context`、`commands`、`call`、`navigate`、`pause`、`resume`、`cancel`、`takeover`、`detach`、`events`；同时检查请求顺序、UI/Agent headers、凭证不出现在普通输出、敏感 target 脱敏，以及 destructive gate 在 Operation/Host API 请求前生效。
- `crates/a4n-server/src/routes/page_sessions.rs` 单测覆盖用户隔离、按 tab 幂等、授权码一次性使用、Agent 绑定、Page CLI 开关、上下文 revision 冲突、页面命令可用性、管理员门禁、危险命令门禁、操作状态转换和 takeover/cancel。
- `frontend/src/api/pageSessions.test.ts` 覆盖请求路径、JSON body、UI/Agent headers、SSE 过滤和错误；`frontend/src/hooks/usePageSession.test.ts` 覆盖路由到页面上下文的推导；控制条测试覆盖等待绑定、运行锁定和 takeover 可见性；认证 store 测试覆盖账户开关更新。

真实 Page CLI smoke test 需要：当前源码构建的 `a4n`、已启用 Page CLI 的隔离管理员账号、正在运行的 Web Server，以及页面上显示的绑定码。测试时必须使用同一个 `--config` 完成 Agent register、attach 和后续 Page CLI 命令；不要把 UI token 或 Agent token 写入 fixture、ledger、报告或日志。完成标准不是 manifest descriptor 数量，而是所有 Page CLI 子命令都有成功或明确的预期失败路径，并且 real Server / Provider 请求链路另有证据。
