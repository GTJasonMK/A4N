# 冗余审计与清理记录（2026-10-10）

## 范围与方法

- 工作分支：`agent/redundancy-cleanup-20261010`，基线 `debug@3f6a2408c9e78c4719259e77d5443f2befdd8fda`。
- 建立分支本地 CodeGraph 索引时覆盖 6,240 个文件、86,648 个符号、277,041 条边；最终索引覆盖 6,241 个文件、86,569 个符号、276,858 条边，状态为 up to date。清理期间同步 34 个文件（新增 1、修改 33），重新解析 1,484 个符号。
- 从 `a4n-server`、Tauri 本地 Server、`a4n-component-host`、Worker/Sidecar catalog、Axum route 注册、`a4n-control` 动态 command 分派和 `frontend/src/App.tsx` 路由入口追踪生产调用与注册点。
- 基线 Cargo workspace 有 213 个 crate manifest；本次增加 runtime crate 后共 214 个。`cargo metadata` 核对了 Worker 对共享配置/runtime crate 的依赖。静态搜索补查脚本启动、环境变量、历史迁移、测试 fixture、生成合同脚本和运行时路由字符串。
- 检查了 `#[allow(dead_code)]`、兼容/legacy 命名、候选无入边符号、Cargo 直接依赖和可复用的前端格式工具。

CodeGraph 无法解析跨 crate 的 `a4n_config::worker_listen_addr` 和 `a4n_worker_runtime` 引用，记录在 `unresolved_refs` 中；这些调用逐处由源码搜索确认，并由全 workspace 编译校验。图谱孤立符号也包含 trait 实现、路由 handler 和测试入口，未把“无入边”直接当作删除证据。

本记录说明实际检查的入口和结果，不声称仅凭静态分析证明了所有运行环境、外部调用者、反射或部署配置均无遗漏。未对所有产品模块做逐函数人工审阅，也未运行前端构建或浏览器流程。

## 清理记录

### R-01：统一 Worker 监听地址读取与校验

- 类别：重复实现；并一并消除了读取环境变量失败时静默采用默认地址的路径。
- 覆盖：29 个 Worker 启动入口，分别位于 `a4n-admin-worker`、`a4n-asset-graph-worker`、`a4n-canvas-record-worker`、`a4n-chapter-content-worker`、`a4n-chapter-scene-worker`、`a4n-config-read-worker`、`a4n-inspiration-worker`、`a4n-manga-record-worker`、`a4n-artifact-media-worker`、`a4n-blueprint-worker`、`a4n-foreshadowing-worker`、`a4n-novel-planning-worker`、`a4n-outline-worker`、`a4n-character-mutation-worker`、`a4n-object-profile-worker`、`a4n-portrait-worker`、`a4n-profile-material-worker`、`a4n-scene-profile-worker`、`a4n-project-access-worker`、`a4n-project-collaboration-worker`、`a4n-project-mutation-worker`、`a4n-project-read-worker`、`a4n-project-status-worker`、`a4n-source-snapshot-worker`、`a4n-theme-read-worker`、`a4n-import-review-worker`、`a4n-video-record-worker`、`a4n-workflow-lifecycle-worker`、`a4n-workflow-read-worker`。
- 所有 Worker 已依赖 `a4n-config`。将地址读取、`SocketAddr` 解析、loopback 限制和非零端口校验收敛到 `crates/config/a4n-config/src/lib.rs::worker_listen_addr`；29 处 `run -> async_main -> listen address -> serve` 启动调用保留各自环境键和明确默认地址。
- 删除 29 个本地 `worker_addr()` 包装和重复校验，共移除约 290 行实现。旧实现对 `env::var` 的所有错误都使用 `unwrap_or_else`，因此非 Unicode 环境值也会静默落到默认地址；现在只有变量缺失时采用原有显式默认值，空值、无法读取、无效地址、非 loopback 或零端口均报错。
- `a4n-artifact-media-worker` 原来只解析地址，没有另外 28 个 Worker 共有的 loopback/非零端口限制；现已纳入同一校验。此行为变化只影响该内部 Worker：原本配置为公网地址或零端口的启动会失败。
- 记录文件：`crates/config/a4n-config/src/lib.rs` 与上列 29 个 Worker 的启动文件。

### R-02：统一 Worker tracing 初始化与 Ctrl+C 关闭

- 类别：同一进程能力的重复实现。
- 30 个 Worker 原本各自配置同义的 `tracing_subscriber` 默认过滤器；26 个 Worker 的 graceful shutdown 都只等待 Ctrl+C。新增 `crates/process/a4n-worker-runtime` 作为 process 层共享 owner，提供 `init_worker_tracing` 和 `wait_for_ctrl_c`。
- 30 个 Worker 都调用共享 tracing 初始化；26 个相同的关闭 handler 改为调用共享 Ctrl+C helper。移除 Worker 本地重复函数，并删除 30 个 Worker manifest 中仅供本地 tracing wrapper 使用的 `tracing-subscriber` 直接依赖；共享 crate 复用 workspace 已有 Tokio 与 tracing-subscriber，不增加第三方依赖。
- Agent、Artifact Media、Canvas Record、Novel Planning Worker 的关闭逻辑各有专用行为，保留本地实现；Agent 与 Novel Planning 的 Unix 关闭路径显式处理 SIGTERM。Host control/edge 与 Sidecar bootstrap 具有不同关闭/日志契约，也未合并。
- 记录文件：`Cargo.toml`、`crates/process/a4n-worker-runtime/`、30 个 Worker 的 `Cargo.toml` 和启动入口。

## 核验后保留

- `crates/host/a4n-api-contract/src/bin/export_generation_schema.rs` 的 `GenerationContractBundle` 由 schema derive 读取，且 `scripts/contracts/generate-generation-contracts.sh` 明确调用该二进制；它不是未使用模块。
- `crates/host/a4n-orchestrator/src/project/family_package/mod.rs` 的 `Model` 别名位于 `cfg(test)` 路径，项目包测试 fixture 使用这些类型；未移除测试支撑代码。
- `normalize_legacy_editor_source` 有 `parse_persisted_editor_source` 生产调用；`archive_legacy_source_sync_payload` 由媒体迁移调用；`maximum_manifest_id` 参与旧 family package 导入。它们仍覆盖输入或持久数据路径，不能按名称认定为伪安全回退。
- Sidecar bootstrap 的 tracing 初始化还负责文件日志保留、鉴权和关闭路由，不能替代普通 Worker runtime；`a4n-control` 与 `a4n-edge` 的日志、关闭行为也独立保留。
- `cargo-machete` 未发现未使用 Cargo 依赖。`cargo tree -d` 中的多个版本属于传递依赖组合，没有足够证据作为可移除的直接重复实现。
- 前端日期解析已由 `frontend/src/i18n/format.ts::parseDate` 共用；数值范围与 CSS RGB 解析已有共享 utility。本次未发现需要再拆分的新事实来源。

## 验证

- `cargo check --workspace --offline`：通过；添加工作区内 `a4n-worker-runtime` 后 Cargo lockfile 已更新。检查覆盖 30 个 Worker 消费者和 Host/Component Host 入口，无警告。
- `timeout 60s cargo test --locked --offline -p a4n-config worker_listen_addr_requires_loopback_and_nonzero_port --lib`：1/1 通过。
- `cargo fmt --all -- --check`：通过。
- `git diff --check`：通过。
- `cargo-machete`：未发现未使用依赖。
- agent 公共构建目录在构建锁内清理；未运行数据库、Server 或前端服务。

## 覆盖边界

- 前端纳入 CodeGraph 路由/API 关系检索；本次改动不触及前端，且新工作树没有 `frontend/node_modules`，所以未运行 TypeScript build、Vitest 或浏览器验收。
- Cargo 未使用依赖扫描不检查未使用的公开 API；Rust workspace 检查只编译当前平台的默认生产目标，不能证明跨平台运行或外部二进制调用不存在。
- 仓库中第三方传递依赖的重复版本、仍在使用的 legacy 输入/数据库迁移、4 个具有不同终止语义的 Worker 生命周期代码，本批保留并记录为后续人工架构审阅对象；本次没有对它们做广泛删除。
