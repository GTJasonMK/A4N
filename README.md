<p align="center">
  <img src="docs/readme/hero-banner.svg" alt="A4N — 从一句灵感到成片，本地优先的一体化全模态小说创作平台" width="880">
</p>

<p align="center">
  <img alt="License" src="https://img.shields.io/badge/license-Apache--2.0-blue.svg">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-1.95-orange.svg">
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2-24C8DB.svg">
  <img alt="React" src="https://img.shields.io/badge/React-18-61DAFB.svg">
</p>

A4N 是一个面向小说创作与多模态派生内容生产的本地应用。项目以文本项目为根，围绕灵感对话、故事蓝图、大纲、章节写作、结构化分析、人物/物体/场景档案、图片生成、漫画派生、视频派生和成片编辑等工作流组织功能。

当前代码库采用双 Host 架构：Tauri 桌面端和 axum Web Server 都通过
`a4n-runtime` 调用同一套 orchestrator 业务逻辑。内置 `a4n` CLI 连接
`a4n-server`，因此无前端运行、桌面端和网页端不会各自复制权限、事务、校验或状态机。

## 核心亮点

### 分析驱动的自反馈闭环

<div align="center"><img src="docs/readme/self-feedback-loop.svg" alt="写作→分析→自动沉淀→检索反哺的自反馈闭环，叠加内容优化 Agent 的人工在环修订" width="880"></div>

写完章节一键四阶段结构化分析（事实/摘要/角色状态/伏笔），结果在同一分析任务内自动沉淀为回顾性章纲、伏笔索引与角色/场景/物体档案并向量化入库；下一章生成前自动检索反哺。内容优化 Agent 再跨文档审查、生成补丁，经你确认后回写正文——让设定、伏笔、前文始终在场，而非“一键 AI 代写”。

### 全模态创作台

<div align="center"><img src="docs/readme/full-modality-pipeline.svg" alt="单一项目根贯穿文本创作、分析档案、视觉生成、视频成片四阶段九环节" width="880"></div>

以同一个项目根为主轴，从灵感、蓝图/大纲、章节写作、结构化分析、人物/物体/场景/立绘档案，一路派生到图片、漫画、视频与节点式成片——九个环节都有真实后端与独立前端工作台。多模态成品依赖你自备的模型/API 与本地服务（图片 ComfyUI/SD/OpenAI；视频 Gemini-Veo/Seedance；成片本地 ffmpeg 与 sidecar）。

### 商业级完成度

<div align="center"><img src="docs/readme/architecture-overview.svg" alt="双 Host 加 CLI 三端收敛到单一业务门面 host_api 与单一分发表" width="880"></div>

双 Host（Tauri 桌面、axum Web）与无前端 CLI 三端都不各自实现业务规则，全部收敛到唯一业务门面 `host_api` 与单一命令分发表；runtime 经 ApplicationPorts 依赖倒置，仅在单一装配点与 orchestrator 组合。

<div align="center"><img src="docs/readme/stats-completeness.svg" alt="201 crate、400+ 业务命令、114 迁移、4200+ 测试断言、26 门禁脚本" width="880"></div>

201 crate / 24 领域、400+ 业务命令（收敛到单一命令目录）、114 数据库迁移、4200+ 测试断言、26 门禁脚本；三端一致性由合同测试 + 代码生成 + `check:web-api-parity` 强制，外部依赖锁版本并校验 SHA256（ONNX 1.24.2 · Qdrant v1.18.3）。

## 技术栈

- Rust workspace：核心模型、服务层、AI 引擎、迁移和 Tauri 命令层，承担后端业务逻辑。
- Tauri 2：桌面应用运行时与前后端桥接。
- React 18 + TypeScript + Vite：前端页面与交互。
- SeaORM + PostgreSQL：桌面与 Web 运行时共用的关系数据库。

## 目录结构

```text
crates/
  host/          Server、Tauri、CLI、Runtime、Orchestrator、Host 装配与 API 合同
  platform/      领域公共值、数据库、迁移、存储、错误、事件、队列与测试支持
  process/       通用 service/worker 协议、客户端、进程管理与发布 catalog
  config/        配置解析与配置读取 worker
  provider/      跨能力 Provider 合同与模型发现
  llm/           LLM 客户端、上下文、网关与 Provider 合同
  prompt/        Prompt 渲染、Git、客户端与服务
  embedding/     Embedding 合同、Provider、客户端与服务
  rag/           RAG 合同、索引、客户端与服务
  agent/         Agent 应用、运行时、worker 与客户端
  project/       项目读取、修改、权限、协作、状态与源快照
  planning/      小说规划、蓝图、大纲与伏笔
  chapter/       章节正文、分析、生成与场景
  profiles/      人物、物体、场景、立绘与档案素材
  inspiration/   灵感记录与应用合同
  image/         图片合同、Provider、客户端与服务
  video/         视频合同、Provider、客户端、服务与记录 worker
  media/         媒体处理、渲染与生成产物
  manga/         漫画核心与记录 worker
  canvas/        画布合同与资产图 worker
  galgame/       Galgame 应用合同
  transfer/      导入、导出、归档与导入审查
  workflow/      工作流读取、生成任务与优化应用合同
  theme/         主题核心与读取 worker
frontend/
  src/           React 前端源码
  resources/     前端资源
prompts/         提示词模板与注册表
work/docs/       本机文档、草稿和历史资料（不提交）
scripts/         开发、构建、测试脚本
```

每个领域目录下保留现有 `a4n-*` crate 名称，例如 `crates/llm/a4n-llm-client`。
同一领域的 `core`、`contract`、`client` 和 `worker` 放在一起；目录分组不改变包和进程边界。
根 `Cargo.toml` 是唯一 workspace，本地依赖路径统一声明在 `[workspace.dependencies]`，成员通过 `workspace = true` 引用。

前端 API 封装集中在 `frontend/src/api/`；Server 维护共享命令分派，Native Tauri 复用该入口。业务编排属于 `a4n-orchestrator`，SQL/Repository 属于 `a4n-db`，运行适配属于 `a4n-runtime`。Host API、通用 sidecar 协议和能力专用合同分别维护，详见各 crate 的 manifest、`command_manifest.rs` 和 `frontend/src/api/`。

## 本地开发

前置依赖：

- Rust toolchain（当前 CI 固定 Rust 1.95.0）
- Node.js 22 和 npm
- PostgreSQL 16+，或 Docker Compose
- Linux 桌面构建需要 WebKitGTK、GTK 和 Ayatana AppIndicator 开发库

Linux 桌面端默认设置 `WEBKIT_GST_DISABLE_GL_SINK=1` 和
`WEBKIT_DISABLE_DMABUF_RENDERER=1`，规避 NVIDIA/Wayland 下拖动闪黑及
暂停后缩放丢帧。内存拷贝可能增加 CPU 开销；已有环境值保持优先。
确认系统播放栈修复后，可把对应变量设为 `0` 恢复相应渲染路径。

启动本地 PostgreSQL：

```bash
./scripts/dev/dev-db.sh up
```

数据库测试使用独立的 `a4n_test` 库，并通过 `A4N_TEST_DATABASE_URL` 指定隔离连接；具体边界以测试脚本和测试代码为准。

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

桌面 `local-server` 模式未设置 `A4N_WEB_ADDR` 时，优先监听 `127.0.0.1:8080`；
端口被占用则由系统分配空闲端口，实际 API 地址在窗口加载前传给前端。
设置 `A4N_WEB_ADDR` 会固定监听地址，占用时明确失败；设置 `127.0.0.1:0` 可每次由系统分配。
桌面本地地址必须为 loopback。HTTP 入口热重启会沿用本次启动已发布的地址。

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
Server 生命周期管理。命令参数和语义以 `a4n --help`、`crates/host/a4n-cli/src/main.rs` 及 Host manifest 为准。

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

桌面默认 `local-server` 模式与浏览器都通过 HTTP 调用 `a4n-server`；桌面 `native`
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

源码检查、现有打包入口与正式发布的边界见 [发布说明](GITHUB_RELEASE.md)；当前没有自动签名 GitHub Release 流水线。参与开发前请阅读 [贡献指南](CONTRIBUTING.md)，构建空间管理以 `rust-artifacts.json` 和 `cargo artifacts inspect` 为准；安全问题按 [安全策略](SECURITY.md) 私下报告。

## 许可证

本项目使用 [Apache License 2.0](LICENSE)。
