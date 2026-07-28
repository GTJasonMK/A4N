# A4N

A4N 是一个面向小说创作与多模态派生内容生产的本地应用。项目以文本项目为根，围绕灵感对话、故事蓝图、大纲、章节写作、结构化分析、人物/物体/场景档案、图片生成、漫画派生、视频派生和成片编辑等工作流组织功能。

当前代码库采用前后端分层结构：前端是 Vite/React，后端是 Rust workspace，运行时由 Tauri 将两者打包为同一个桌面应用。它不是传统 Web 项目中“前端站点 + 独立后端服务”的分开部署形态，前端主要通过 Tauri command 调用后端能力。

## 技术栈

- Rust workspace：核心模型、服务层、AI 引擎、迁移和 Tauri 命令层，承担后端业务逻辑。
- Tauri 2：桌面应用运行时与前后端桥接。
- React 18 + TypeScript + Vite：前端页面与交互。
- SeaORM + PostgreSQL：桌面与 Web 运行时共用的关系数据库。

## 目录结构

```text
crates/
  a4n-core                     SeaORM entity、配置、事件和基础共享类型
  a4n-domain / a4n-db          领域值对象与数据库边界
  a4n-queue                    任务队列合同与实现
  a4n-prompt* / a4n-llm*      Prompt 与 LLM 核心、provider 和 sidecar
  a4n-embedding* / a4n-rag*   Embedding 与 RAG 核心、索引和 sidecar
  a4n-image* / a4n-video*     图片/视频合同、provider 和 sidecar
  a4n-api-contract            Server/Tauri 共享 Host API 合同与 TS 生成
  a4n-service-protocol        八个 sidecar 的协议、catalog 和发布合同
  a4n-db / a4n-domain         数据库边界与纯领域值对象
  a4n-orchestrator            业务规则、事务、工作流和能力编排唯一 owner
  a4n-runtime                 provider、文件、进程和 Host API 适配
  a4n-tauri / a4n-server      桌面与 Web host 入口
  a4n-migration               数据库迁移
frontend/
  src/           React 前端源码
  resources/     前端资源
prompts/         提示词模板与注册表
docs/            功能文档与架构设计
scripts/         开发、构建、测试脚本
```

前端 API 封装集中在 `frontend/src/api/`，后端命令入口集中在 `crates/a4n-tauri/src/commands/`。业务规则由 `a4n-orchestrator` 拥有；`a4n-runtime` 只实现数据库、provider、文件、进程与 Host transport 适配。Host API DTO 位于 `a4n-api-contract`，sidecar 合同位于 `a4n-service-protocol`，`a4n-core` 只保留实体、基础配置、基础事件和少量无依赖共享值。

## 本地开发

前置依赖：

- Rust toolchain
- Node.js 和 npm
- PostgreSQL 16+，或 Docker Compose
- Linux 桌面构建需要 WebKitGTK、GTK 和 Ayatana AppIndicator 开发库

启动本地 PostgreSQL：

```bash
./scripts/dev/dev-db.sh up
```

数据库测试使用独立的 `a4n_test` 库。具体连接方式和并行测试注意事项见[测试数据库说明](docs/TEST_DATABASE.md)。

安装前端依赖：

```bash
cd frontend
npm install
```

安装本地 embedding 使用的共享 ONNX Runtime 1.24.2：

```bash
./scripts/dev/setup-onnxruntime.sh
```

运行库默认安装到 `~/.local/share/a4n/onnxruntime/1.24.2`，独立于 Cargo 的
`target/` 和构建缓存。项目构建脚本会动态链接该固定版本；缺失或版本不匹配时
会明确失败，不会在 Cargo 构建阶段自动下载。

部署目标机同样需要先运行该安装脚本，或通过 `A4N_ONNXRUNTIME_ROOT` 指向一套
完整的 ONNX Runtime 1.24.2 动态库目录。

启动开发环境：

```bash
A4N_JWT_SECRET=replace-with-a-long-random-secret \
./scripts/run.sh
```

构建前端并启动 Web 服务端：

```bash
A4N_JWT_SECRET=replace-with-a-long-random-secret \
A4N_BOOTSTRAP_ADMIN_USERNAME=admin \
A4N_BOOTSTRAP_ADMIN_EMAIL=admin@example.com \
A4N_BOOTSTRAP_ADMIN_PASSWORD=replace-with-an-initial-admin-password \
./scripts/dev/run-web.sh
```

Web 空库首次启动必须提供 `A4N_BOOTSTRAP_ADMIN_USERNAME`、`A4N_BOOTSTRAP_ADMIN_EMAIL`、`A4N_BOOTSTRAP_ADMIN_PASSWORD` 三个变量创建首个管理员；用户表已有账号时不会再次创建。

只做 Rust 编译检查：

```bash
./scripts/build.sh --check
```

构建前端和后端调试产物：

```bash
./scripts/build.sh
```

构建前端：

```bash
cd frontend
npm run build
```

运行 Rust 测试：

```bash
./scripts/test.sh
```

运行指定 crate 测试：

```bash
./scripts/test.sh --crate a4n-core --nocapture
```

## 配置与本地数据

运行时日志、缓存、临时文件和生成媒体属于本地数据，不提交到 Git。默认本地目录为 `data/`，已在 `.gitignore` 中忽略。数据库不再是 `data/` 下的本地 SQLite 文件，桌面和 Web 运行时都通过 `A4N_DATABASE_URL` 连接 PostgreSQL。

环境变量模板见 `.env.example`。桌面端和 Web 端启动时必须提供 PostgreSQL 连接串和 JWT secret，可通过环境变量或桌面端 `data/config.toml` 配置。真实 API Key、OAuth Client Secret、数据库连接串、日志和导出产物不要提交到仓库。

必需变量：

- `A4N_DATABASE_URL`
- `A4N_JWT_SECRET`

Web 运行常用变量：

- `A4N_WEB_ADDR`
- `A4N_FRONTEND_DIST`
- `A4N_BOOTSTRAP_ADMIN_USERNAME`
- `A4N_BOOTSTRAP_ADMIN_EMAIL`
- `A4N_BOOTSTRAP_ADMIN_PASSWORD`

其他常用变量：

- `A4N_DATA_DIR`
- `A4N_LOG_DIR`
- `A4N_OPENAI_API_KEY`
- `A4N_OPENAI_BASE_URL`
- `DATABASE_URL`
- `A4N_TEST_DATABASE_URL`
- `GITHUB_CLIENT_ID`
- `GITHUB_CLIENT_SECRET`
- `GOOGLE_CLIENT_ID`
- `GOOGLE_CLIENT_SECRET`
- `RUST_LOG`

大部分 AI 服务配置也可以通过应用内设置页面管理。

## Web 部署状态

Web 服务端是独立 crate `a4n-server`，使用显式 HTTP 路由，不暴露 `/api/commands/:cmd`。当前已实现：

- `GET /api/health`
- `POST /api/auth/login`
- `GET /api/auth/me`
- `POST /api/auth/logout`
- `POST /api/auth/refresh`
- `GET /api/admin/users`
- `POST /api/admin/users`
- `POST /api/admin/users/{user_id}/role`
- `POST /api/admin/users/{user_id}/disable`
- `GET /api/projects`
- `POST /api/projects`
- `POST /api/projects/derived`
- `GET /api/projects/{root_project_id}/derived`
- `GET /api/projects/{project_id}/source-snapshot-chapters`
- `GET /api/projects/{project_id}`
- `POST /api/source-sync/preview`
- `GET /api/source-sync/runs/{sync_run_id}/conflicts`
- `POST /api/source-sync/apply`
- `POST /api/source-sync/detach`

前端在 Tauri 内仍调用 Tauri command；在普通浏览器中会调用上述 REST 接口。尚未映射到 REST 的前端 API 会明确报错，避免隐式兼容层。

Web 运行时的管理员页面当前只开放用户管理；桌面运行时仍保留概览、项目、任务和用量趋势等完整管理员功能。

桌面端与 Web 端功能一致性必须通过显式命令映射校验：

```bash
cd frontend && npm run check:web-api-parity
```

该检查会对齐前端 `apiInvoke` 命令、桌面 Tauri 注册命令、Web `webInvoke` 映射命令，并标出页面/功能组件中直接导入的 Tauri API。只要仍有前端命令缺少 Web 映射，或仍有事件监听、系统文件选择器、媒体协议等桌面专属能力没有 Web 等价适配，就不能认为 Web 端功能已经与桌面端一致。

## 发布前检查

```bash
git diff --check
cd frontend && npm run check:web-api-parity
cd frontend && npm run build
./scripts/build.sh --check
```

源码推送、GitHub 仓库设置、正式签名 Release 所需的 secrets/variables 和打 tag 顺序见 [GitHub 发布说明](GITHUB_RELEASE.md)。参与开发前请阅读 [贡献指南](CONTRIBUTING.md)；安全问题按 [安全策略](SECURITY.md) 私下报告。

## 许可证

本项目使用 [Apache License 2.0](LICENSE)。
