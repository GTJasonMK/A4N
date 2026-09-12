# A4N

A4N 是一个面向小说创作与多模态派生内容生产的本地应用。项目以文本项目为根，围绕灵感对话、故事蓝图、大纲、章节写作、结构化分析、人物/物体/场景档案、图片生成、漫画派生、视频派生和成片编辑等工作流组织功能。

当前代码库采用双 Host 架构：Tauri 桌面端和 axum Web Server 都通过
`a4n-runtime` 调用同一套 orchestrator 业务逻辑。内置 `a4n` CLI 连接
`a4n-server`，因此无前端运行、桌面端和网页端不会各自复制权限、事务、校验或状态机。

## 技术栈

- Rust workspace：核心模型、服务层、AI 引擎、迁移和 Tauri 命令层，承担后端业务逻辑。
- Tauri 2：桌面应用运行时与前后端桥接。
- React 18 + TypeScript + Vite：前端页面与交互。
- SeaORM + PostgreSQL：桌面与 Web 运行时共用的关系数据库。

## 目录结构

```text
crates/
  a4n-db-models               SeaORM 实体；纯领域值位于 a4n-domain
  a4n-config / a4n-storage     配置解析与文件生命周期边界
  a4n-domain                  纯领域值、蓝图合同与业务算法
  a4n-db                      SQL、Repository 与数据库一致性边界
  a4n-task-runtime            活跃任务句柄与进度运行态
  a4n-error / a4n-progress     共享错误与纯结构化进度算法
  a4n-queue                   内存队列实现；纯任务合同位于 a4n-domain
  a4n-prompt* / a4n-llm*      Prompt 与 LLM 核心、provider 和 sidecar
  a4n-embedding* / a4n-rag*   Embedding 与 RAG 核心、索引和 sidecar
  a4n-image* / a4n-video*     图片/视频合同、provider 和 sidecar
  a4n-api-contract            Server/Tauri 共享 Host API 合同与 TS 生成
  a4n-service-protocol        生产 sidecar 的协议、catalog 和发布合同
  a4n-*-contract              能力专用 wire / 执行合同
  a4n-orchestrator            业务用例、工作流和能力编排
  a4n-runtime                 provider、文件、进程和 Host API 适配
  a4n-tauri / a4n-server      桌面与 Web host 入口
  a4n-cli                     无前端 CLI、Agent 合同、Shell 和 local 生命周期
  a4n-migration               数据库迁移
frontend/
  src/           React 前端源码
  resources/     前端资源
prompts/         提示词模板与注册表
docs/            功能文档与架构设计
scripts/         开发、构建、测试脚本
```

前端 API 封装集中在 `frontend/src/api/`；Server 维护共享命令分派，Native Tauri 复用该入口。业务编排属于 `a4n-orchestrator`，SQL/Repository 属于 `a4n-db`，运行适配属于 `a4n-runtime`。Host API、通用 sidecar 协议和能力专用合同分别维护，详见[当前代码架构](docs/ARCHITECTURE.md)。完整资料从[文档导航](docs/README.md)进入。

## 本地开发

前置依赖：

- Rust toolchain（当前 CI 固定 Rust 1.95.0）
- Node.js 22 和 npm
- PostgreSQL 16+，或 Docker Compose
- Linux 桌面构建需要 WebKitGTK、GTK 和 Ayatana AppIndicator 开发库

启动本地 PostgreSQL：

```bash
./scripts/dev/dev-db.sh up
```

数据库测试使用独立的 `a4n_test` 库。具体连接方式和并行测试注意事项见[测试数据库说明](docs/TEST_DATABASE.md)。

安装前端依赖：

```bash
npm --prefix frontend ci
```

安装本地 embedding 使用的共享 ONNX Runtime（版本由 `tools/onnxruntime/VERSION` 固定）：

```bash
./scripts/dev/setup-onnxruntime.sh
```

运行库默认安装到 `~/.local/share/a4n/onnxruntime/<version>`，独立于 Cargo 的
`target/` 和构建缓存。项目构建脚本会动态链接该固定版本；缺失或版本不匹配时
会明确失败，不会在 Cargo 构建阶段自动下载。

部署目标机同样需要先运行该安装脚本，或通过 `A4N_ONNXRUNTIME_ROOT` 指向一套
版本匹配的完整 ONNX Runtime 动态库目录。

安装 RAG 使用的固定版本 Qdrant：

```bash
./scripts/dev/setup-qdrant.sh
```

`scripts/run.sh` 会复用健康的本地 Qdrant；默认地址不可达时，会自动启动上述固定版本。

启动开发环境：

```bash
A4N_JWT_SECRET=replace-with-a-long-random-secret \
./scripts/run.sh
```

`scripts/run.sh` 会在任何 Tauri/Server Debug 构建前检查 PostgreSQL、JWT、前端依赖；热重载模式还会先检查 `VITE_PORT`（默认 `5173`）是否被占用；
本地 managed-sidecar 模式还会检查 ONNX Runtime、Qdrant、sidecar catalog 和 Tauri
系统库。检查失败会直接停止，不进入 Cargo 或前端构建。
开发调试请使用该入口；直接执行 `cargo tauri dev` 不会经过这套依赖门禁。

构建前端并启动 Web 服务端：

```bash
A4N_JWT_SECRET=replace-with-a-long-random-secret \
A4N_BOOTSTRAP_ADMIN_USERNAME=admin \
A4N_BOOTSTRAP_ADMIN_EMAIL=admin@example.com \
A4N_BOOTSTRAP_ADMIN_PASSWORD=replace-with-an-initial-admin-password \
./scripts/run.sh --server
```

Web 空库首次启动必须提供 `A4N_BOOTSTRAP_ADMIN_USERNAME`、`A4N_BOOTSTRAP_ADMIN_EMAIL`、`A4N_BOOTSTRAP_ADMIN_PASSWORD` 三个变量创建首个管理员；用户表已有账号时不会再次创建。

只做 Rust 编译检查：

```bash
./scripts/build.sh --backend --check
```

构建前端和后端调试产物：

```bash
./scripts/build.sh
```

构建前端：

```bash
npm --prefix frontend run build
```

运行 Rust 测试：

```bash
./scripts/test.sh
```

运行指定 crate 测试：

```bash
timeout 60s ./scripts/test.sh --crate a4n-domain --nocapture
```

## 无前端 CLI

`a4n` 通过 Server 调用共享 Host API manifest 中与桌面/Web 相同的全部业务命令，并提供稳定的
JSON/JSONL Agent 输出、危险操作授权、task/workflow 观察和本地
Server 生命周期管理。完整说明见 [A4N CLI 手册](docs/CLI.md)。

本地启动：

```bash
export A4N_DATABASE_URL='postgres://USER:PASSWORD@127.0.0.1:5432/a4n'
export A4N_JWT_SECRET='replace-with-a-long-random-secret'

cargo run -p a4n-cli -- local start
cargo run -p a4n-cli -- --output agent-json capabilities check
```

发布包中可直接使用：

```bash
a4n local start
a4n health
a4n --output agent-json capabilities export
a4n config runtime set --input @runtime-config.json
a4n local stop
```

`config runtime show/set/clear` 管理与 GUI 本机 Provider 设置相同的运行时配置。
`a4n call` 会按 manifest 的 `full`、`without_secrets` 或 `none` 策略自动注入；预览请求
不会携带 Provider 密钥。

发布包默认 sidecar profile 已包含完整 AI 能力。`a4n local start --full` 专用于源码
workspace，会重新构建带 `embedded-capabilities` feature 的 Server。

`a4n local` 会先运行独立 migration，再启动 Server；它不会自动启动 PostgreSQL
或 Docker。默认 profile 使用 catalog 声明的 managed sidecar；源码 workspace 可用
`--full` 运行 embedded capabilities。

## 配置与本地数据

运行时日志、缓存、临时文件和生成媒体属于本地数据，不提交到 Git。默认本地目录为 `data/`，已在 `.gitignore` 中忽略。数据库不再是 `data/` 下的本地 SQLite 文件，桌面和 Web 运行时都通过 `A4N_DATABASE_URL` 连接 PostgreSQL。

环境变量模板见 `.env.example`。桌面端和 Web 端启动时必须提供 PostgreSQL 连接串和 JWT secret，可通过环境变量或桌面端 `data/config.toml` 配置。真实 API Key、OAuth Client Secret、数据库连接串、日志和导出产物不要提交到仓库。

必需变量：

- `A4N_DATABASE_URL`
- `A4N_JWT_SECRET`

Web 运行常用变量：

- `A4N_WEB_ADDR`
- `A4N_CONTROL_ADDR`（Server 入口启动的内部 Control 地址；通常无需手动设置）
- `A4N_CONTROL_BINARY`（可选的独立 `a4n-control` 二进制路径）
- `A4N_FRONTEND_DIST`
- `A4N_BOOTSTRAP_ADMIN_USERNAME`
- `A4N_BOOTSTRAP_ADMIN_EMAIL`
- `A4N_BOOTSTRAP_ADMIN_PASSWORD`

其他常用变量：

- `A4N_DATA_DIR`
- `A4N_LOG_DIR`
- `A4N_OPENAI_API_KEY`
- `A4N_OPENAI_BASE_URL`
- `A4N_TEST_DATABASE_URL`
- `GITHUB_CLIENT_ID`
- `GITHUB_CLIENT_SECRET`
- `GOOGLE_CLIENT_ID`
- `GOOGLE_CLIENT_SECRET`
- `RUST_LOG`

大部分 AI 服务配置也可以通过应用内设置页面管理。

## Desktop / Web / CLI 一致性

桌面默认 `local-server` 模式与浏览器都通过 HTTP 调用 `a4n-server`。入口进程只负责 Edge
的监听、静态文件和 `/api` 代理，Control 在独立子进程中持有业务路由和 Runtime；设置
`A4N_CONTROL_BINARY` 后可直接替换为独立编译的 Control 进程。桌面 `native`
模式才使用 Tauri command。Server 提供显式 REST 路由和 `/api/commands/:cmd`。CLI 对 Web 专用 REST 路由使用相同映射，其余命令走
统一 command endpoint。共享 Host API manifest 位于 `a4n-api-contract`，是 CLI
能力发现和三端 parity 的单一命令目录。

一致性检查：

```bash
npm --prefix frontend run check:web-api-parity
```

该检查会对齐前端 `apiInvoke` 命令、桌面 Tauri 注册命令、Web 路由、CLI
专用映射、manifest 已知命令的通用 command endpoint 和 Tauri-only runtime 等价入口。页面
导航、系统文件选择器、原生窗口和媒体协议等壳层行为仍由各 Host 适配，不进入共享
业务命令目录。

## 发布前检查

```bash
git diff --check
npm --prefix frontend run check:web-api-parity
npm --prefix frontend run build
./scripts/build.sh --backend --check
```

源码检查、现有打包入口与正式发布的边界见 [发布说明](GITHUB_RELEASE.md)；当前没有自动签名 GitHub Release 流水线。参与开发前请阅读 [贡献指南](CONTRIBUTING.md)，构建空间管理见 [Rust 缓存说明](docs/rust-artifacts.md)；安全问题按 [安全策略](SECURITY.md) 私下报告。

## 许可证

本项目使用 [Apache License 2.0](LICENSE)。
