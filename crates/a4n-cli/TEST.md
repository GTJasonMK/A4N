# CLI 验证指南

本文件说明当前测试入口与验证边界。历史测试数量、旧 manifest 版本和单次运行结果不作为当前通过保证；实际结果记录在本次 PR、CI 或本地工作记录中。

## 单元与子进程测试

从仓库根目录运行：

```bash
timeout 60s ./scripts/cargo.sh test --locked -p a4n-cli --bin a4n -- --test-threads=1
timeout 60s ./scripts/cargo.sh test --locked -p a4n-cli --test cli_e2e -- --test-threads=1
```

源文件中的测试覆盖参数解析、输出模式、权限门禁、配置与上下文持久化、HTTP/SSE 错误和本地生命周期。`tests/cli_e2e.rs` 通过 mock Server 验证真实 CLI 子进程，包括鉴权、语义命令、文件保护、Page CLI 和 Agent 租约行为。

关注失败路径：无凭据或未授权操作不得访问写接口；失败和取消必须保留非零退出码；覆盖写入失败时保留原文件；凭据不得进入 URL、日志或上下文文件。命令目录数量不能当作业务覆盖率。

## Server 与 Provider 集成

这些测试会访问真实服务或数据库，先通过独立的 `A4N_TEST_DATABASE_URL` 分配隔离环境：

```bash
python3 scripts/tests/run_cli_contract_e2e.py --help
python3 scripts/tests/run_all_workflows_e2e.py --help
python3 scripts/tests/run_agent_cli_e2e.py --help
python3 scripts/tests/run_specialized_agent_cli_e2e.py --help
```

- contract runner 使用 ledger 验证指定场景；全量 workflow runner 是 manifest 入口探针，两者用途不同。
- 将成功业务调用、权限/前置状态拒绝、缺失 fixture 和真实失败分开记录。
- 写入及破坏性 fixture 只有在明确授权的隔离环境中运行，不能因为示例带有相关选项就直接复用。
- 完整准备步骤由对应的 `scripts/tests/run_*e2e.py` 入口负责；浏览器联动使用 `scripts/tests/run_browser_full_project_e2e.py`。

runner 自身的检查入口：

```bash
timeout 60s python3 scripts/tests/test_cli_contract_runner.py
timeout 60s python3 scripts/tests/test_mock_openai_provider.py
```

## 合同与交付检查

```bash
npm --prefix frontend run check:web-api-parity
git diff --check
```

参数与语义命令以当前 `main.rs` 的 Clap 定义及同版本 `a4n --help` 为准；服务端命令和 payload 以能力 manifest/descriptor 为准。

额外检查 CLI 在仓库之外的运行、发布包内的路径解析和对应平台的本地进程生命周期。Linux 检查不能替代 Windows/macOS 实机结果；OAuth 与商业 Provider 也需要真实外部环境。未配置的行/分支覆盖率不能用测试数量代替。
