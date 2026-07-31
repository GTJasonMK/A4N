# 贡献指南

## 开发环境

需要 Rust、Node.js 22、npm、PostgreSQL 16+，Linux 桌面构建还需要 WebKitGTK、GTK 和 Ayatana AppIndicator 开发库。

```bash
./scripts/dev/dev-db.sh up
./scripts/dev/setup-onnxruntime.sh
cd frontend && npm ci
```

本地启动、环境变量和完整构建命令见 [README](README.md)。数据库测试必须使用独立的 `a4n_test`，具体规则见 [测试数据库说明](docs/TEST_DATABASE.md)。不要对开发数据库运行会重建或清理 schema 的测试。

## 架构边界

- 业务规则、事务和工作流由 `a4n-orchestrator` 拥有。
- 数据库、provider、文件、进程和 Host transport 适配留在 `a4n-runtime`。
- Host API 合同属于 `a4n-api-contract`，sidecar 合同属于 `a4n-service-protocol`。
- 不在 Runtime、Orchestrator、数据库或 sidecar 复制第二套业务规则、权限规则、事件合同或失败补偿。
- 架构边界变化必须在同一 PR 更新对应合同、边界检查和测试。

## 提交前检查

按修改范围选择最小验证；后端测试默认设置 60 秒超时。提交前至少执行：

```bash
git diff --check
./scripts/build.sh --check
cd frontend && npm run check:web-api-parity && npm run build
```

数据库、sidecar process、发布或供应链变化还应运行对应的 `scripts/checks/` 门禁。不能把超时、跳过或仅编译成功写成测试通过。

## Pull Request

PR 应说明修改范围、新旧 owner、数据库或 API/协议变化、执行过的验证和未验证风险。不要提交 `.env`、数据库、日志、PID、构建目录、生成媒体、签名证书或本机运行数据。

除非明确书面声明为“Not a Contribution”，提交到本仓库的贡献按项目 [Apache License 2.0](LICENSE) 提供。
