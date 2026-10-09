# 冗余审计与清理记录（2026-10-10）

## 范围与方法

- 工作分支：`agent/redundancy-cleanup-20261010`，基线 `debug@3f6a2408c9e78c4719259e77d5443f2befdd8fda`。
- 建立分支本地 CodeGraph 索引后持续同步；最终索引覆盖 6,242 个文件、86,469 个符号、276,453 条边，状态为 up to date。
- 从 `a4n-server`、Tauri 本地 Server、`a4n-component-host`、Worker/Sidecar catalog、Axum route 注册、`a4n-control` 动态 command 分派和 `frontend/src/App.tsx` 路由入口追踪生产调用与注册点。前端索引包含 444 个 TS 和 318 个 TSX 文件。
- 基线 Cargo workspace 有 213 个 crate manifest；本次增加 runtime crate 后共 214 个。`cargo metadata` 核对了 Worker 对共享配置/runtime crate 的依赖。静态搜索补查脚本启动、环境变量、历史迁移、测试 fixture、生成合同脚本和运行时路由字符串。
- 检查了 `#[allow(dead_code)]`、兼容/legacy 命名、候选无入边符号、Cargo 直接依赖和可复用的前端格式工具。

CodeGraph 无法解析跨 crate 的 `a4n_config::worker_listen_addr`（29 处）、`a4n_worker_client::worker_client_connection_from_env`（28 处）、`a4n_worker_client` 错误映射 helpers（48 处）和 `a4n_worker_runtime` helpers（62 处）引用，记录在 `unresolved_refs` 中；这些调用逐处由源码搜索确认，并由对应消费者编译校验。图谱孤立符号也包含 trait 实现、路由 handler 和测试入口，未把“无入边”直接当作删除证据。

本记录说明实际检查的入口和结果，不声称仅凭静态分析证明了所有运行环境、外部调用者、反射或部署配置均无遗漏。未对所有产品模块做逐函数人工审阅，也未运行浏览器端到端流程。

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

### R-03：复用 Worker client 的安全 Debug 输出

- 类别：重复实现。
- 覆盖 30 个 Worker client：admin、agent、asset graph、canvas record、chapter content、chapter scene、config read、inspiration、manga record、blueprint、foreshadowing、outline、character mutation、object profile、portrait、profile material、scene profile、project access、project collaboration、project mutation、project read、project status、source snapshot、theme read、video record、workflow lifecycle、workflow read、novel planning、import review 和 artifact media。
- 这 30 个包装器都只含 `HttpWorkerClient` 一个字段；底层 `HttpWorkerClient` 已唯一实现 Debug 脱敏（地址显示为 `[REDACTED_URL]`，token 只暴露是否配置）。删除 30 份逐字相同的手写 fmt builder，改用 `#[derive(Clone, Debug)]` 委托给 transport；底层实现和脱敏规则不变。
- 保留所有现有 client URL/token 脱敏回归测试，并为此前没有该测试的 artifact media client 增加一项；不删除逐 crate 安全断言。
- 记录文件：30 个 client 的 `src/lib.rs` 和 `crates/media/a4n-artifact-media-client/src/lib.rs` 测试模块。

### R-04：统一 Worker client 环境读取规则

- 类别：重复配置解析。
- 覆盖 28 个默认式 Worker client `from_env`：读取 URL 和可选 token、忽略缺失/空白值、URL 缺失时采用 client 原有的 `DEFAULT_WORKER_URL`，最后调用各自 `new`。这些构造器在 `a4n-host-assembly` 中有生产调用，部署 URL/token 也由 process catalog 定义。
- 将读取与缺失/空白处理收敛到 `crates/process/a4n-worker-client/src/lib.rs::worker_client_connection_from_env`。各 client 继续传入自己的环境键和默认 URL，解析后交给原 `new`；保留现有值（包括非空值中的空白）和错误读取时的 `.ok()` 处理语义。
- 不迁移 `AgentWorkerClient::from_env`：它先读 catalog 使用的 `A4N_AGENT_WORKER_URL`，再从 catalog 的 `addr_env` `A4N_AGENT_WORKER_ADDR` 构造旧式 HTTP URL。也不迁移 `ArtifactMediaWorkerClient::from_env`：它在 URL 或 token 缺失/空白时返回 `None`，由 Host assembly 将未配置视为明确状态。这两条路径语义不同且有活跃装配调用，保持独立。
- 新增共享解析器的单测，覆盖 URL/token 缺失、空白和已配置值；28 个消费 crate 的检查通过。
- 记录文件：`crates/process/a4n-worker-client/src/lib.rs`、`src/tests.rs` 与上列 28 个 client 的 `src/lib.rs`。

### R-05：统一子进程父进程退出信号

- 类别：跨进程管理模块的相同平台实现。
- 覆盖 3 个模块、6 个生产调用：Server 启动 Control、managed Worker、Edge；Tauri 启动本地 Server 和 Native Worker；ServiceManager 启动 sidecar。
- 三处原实现均在 Linux `pre_exec` 内调用 `prctl(PR_SET_PDEATHSIG, SIGTERM)`，非 Linux 为空操作。实现现收敛为 `a4n-worker-runtime::configure_parent_death_signal`；ServiceManager 将 Tokio 命令转为 std 命令调用同一 owner，其余调用位置不变。
- 保留平台差异和原有子进程终止行为；本次没有放宽启动错误，也没有吞掉 `prctl` 错误。
- 记录文件：`crates/process/a4n-worker-runtime/src/lib.rs`、`a4n-server/src/server/process.rs`、`a4n-tauri/src/server_process.rs`、`a4n-service-manager/src/process.rs` 及三个依赖 manifest。

### R-06：合并配置测试结果日期格式化

- 类别：重复日期解析与展示实现。
- `frontend/src/pages/settings/config-list/testResultUtils.ts::formatConfigTestTime` 单独创建 Date 并调用 `toLocaleString`；调用点为 Config Editor 的测试时间和配置列表侧栏 title。输入无效时原文回显、不显示年份、秒按调用选项显示，与共享 `formatDateTime` 的格式选项可表达。
- `formatDateTime` 增加 `includeYear` / `includeSeconds` 选项；配置测试时间 wrapper 改为调用共享 parser/formatter，并传原输入作为 invalid fallback。保留 locale、时区和 `withSeconds` 行为。
- 为该 wrapper 增加两项测试，覆盖 invalid 原文回显、无年份和有/无秒格式。
- 记录文件：`frontend/src/i18n/format.ts`、`frontend/src/pages/settings/config-list/testResultUtils.ts` 和对应 test 文件。

### R-07：统一数字 locale 格式化

- 类别：重复数字格式化。
- `AgentContextMeter` 自行对 token 数调用 `Number.toLocaleString`；`ModelUsageCostPanel::formatMicros` 也独立调用它并设置 2 到 6 位小数。共享 `i18n/format.ts::formatNumber` 已支持 locale 和 `Intl.NumberFormatOptions`。
- 两处改用 `formatNumber`：Agent token 仍先 round/clamp；微单位金额仍先转金额、非有限输入仍回显原文，小数位限制保持不变。i18n 测试覆盖 formatter 的精度选项，Agent meter 的既有渲染断言覆盖分组数字。
- 记录文件：`frontend/src/components/agent/AgentContextMeter.tsx`、`frontend/src/pages/novel-detail/ModelUsageCostPanel.tsx`、`frontend/src/i18n/i18n.test.ts`。

### R-08：集中 control-request WorkerLease 构造

- 类别：协议合同字段的重复初始化。
- 29 个 Worker client 都构造相同的 `WorkerLease`：epoch `1`、fence `control-request`、expiry `9999-12-31T23:59:59Z`，差异仅在 worker ID。该构造已收敛到 `a4n-worker-contract::WorkerLease::control_request(worker_id)`，协议值现在只有一份定义。
- 删除 29 处 client 私有 fence/expiry 常量和重复 struct 初始化；worker ID 继续由各自合同常量传入。Artifact Media 使用随机 fence 和五分钟过期时间，保留其真实短期 lease。
- 在 worker contract wire tests 增加 canonical lease 字段与合同校验用例。
- `control_request_lease_has_one_canonical_shape`：1/1 通过；all-targets workspace 检查编译所有 29 个构造器调用。
- 记录文件：`crates/process/a4n-worker-contract/src/wire.rs`、`wire_tests.rs` 和 29 个 Worker client 的 `src/lib.rs`。

### R-09：统一 Worker client 错误映射

- 类别：跨客户端重复的传输与基础协议错误映射。
- 29 个 Worker client 原本各自复制 `WorkerClientError -> AppError` 的相同 `match`；现在由 `a4n-worker-client::map_worker_client_error` 统一实现，`WorkerClientErrorPolicy` 明确保留 22 个配置错误策略和 7 个瞬态错误转 `Timeout` 策略。地址、状态码、传输错误和超时文案仍按原 Worker 名称输出。
- 19 个 client 的基础 `WorkerError -> AppError` 响应映射也改为调用 `map_worker_response_error`，只在本地保留 Worker 名称适配。Admin、Canvas Record、Foreshadowing、Novel Planning、Scene Profile、Video Record、Workflow Lifecycle、Workflow Read 以及 Inspiration/Source Snapshot 的映射包含重试、取消、内部错误、冲突文案或固定操作等差异，逐项核对后保留，不强行合并。
- 新增共享映射的超时策略、HTTP 状态边界和基础响应冲突/内部错误单测；all-targets 编译覆盖 29 个调用点。此次没有增加重试、默认值或错误吞并路径。
- 记录文件：`crates/process/a4n-worker-client/{Cargo.toml,src/lib.rs,src/tests.rs}` 及 29 个 Worker client 的 `src/lib.rs`。

## 核验后保留

- `crates/host/a4n-api-contract/src/bin/export_generation_schema.rs` 的 `GenerationContractBundle` 由 schema derive 读取，且 `scripts/contracts/generate-generation-contracts.sh` 明确调用该二进制；它不是未使用模块。
- `crates/host/a4n-orchestrator/src/project/family_package/mod.rs` 的 `Model` 别名位于 `cfg(test)` 路径，项目包测试 fixture 使用这些类型；未移除测试支撑代码。
- `normalize_legacy_editor_source` 有 `parse_persisted_editor_source` 生产调用；`archive_legacy_source_sync_payload` 由媒体迁移调用；`maximum_manifest_id` 参与旧 family package 导入。它们仍覆盖输入或持久数据路径，不能按名称认定为伪安全回退。
- Sidecar bootstrap 的 tracing 初始化还负责文件日志保留、鉴权和关闭路由，不能替代普通 Worker runtime；`a4n-control` 与 `a4n-edge` 的日志、关闭行为也独立保留。
- `cargo-machete` 未发现未使用 Cargo 依赖。`cargo tree -d` 中的多个版本属于传递依赖组合，没有足够证据作为可移除的直接重复实现。
- 前端日期解析已由 `frontend/src/i18n/format.ts::parseDate` 共用；数值范围与 CSS RGB 解析已有共享 utility。本次未发现需要再拆分的新事实来源。

## 验证

- `cargo check --workspace --locked --offline`：续作后全 workspace 通过，无警告。
- `cargo check --workspace --all-targets --locked --offline`：通过，编译全 workspace 的测试/示例目标，无警告。
- `timeout 60s cargo test --locked --offline -p a4n-worker-client --lib`：14/14 通过，覆盖共享传输与响应错误映射。
- `cargo check --locked --offline -p a4n-worker-runtime -p a4n-service-manager -p a4n-server -p a4n-tauri`：通过，验证共享父进程退出信号 helper 的三个集成点和更新后的 lockfile。
- `timeout 60s cargo test --locked --offline -p a4n-config worker_listen_addr_requires_loopback_and_nonzero_port --lib`：1/1 通过。
- Worker client Debug 脱敏定向测试：30 个 client 各 1 项通过；共享 URL/token 解析测试 1/1 通过；28 个环境读取消费者 `cargo check` 通过。
- `cargo fmt --all -- --check`：通过。
- `git diff --check`：通过。
- `cargo-machete`：未发现未使用依赖。
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`：未通过；命中基线 `a4n-domain` 的 5 个既有 lint（`large_enum_variant`、`collapsible_if`、`nonminimal_bool`），未涉及本批改动，未修改以掩盖该失败。
- `npm ci`：成功；Vitest 配置时间、Agent Context Meter 和 i18n 三份测试文件 10/10 通过；`npm run build` 通过。构建保留既有 >500 kB chunk 提示；npm audit 显示 37 项现有依赖漏洞（1 low、5 moderate、31 high），本次未修改锁定的三方依赖。
- agent 公共构建目录在构建锁内清理；未运行数据库、Server 或前端服务。

## 覆盖边界

- 前端检查覆盖路由/API 关系、共享日期/数字 formatter、配置测试时间和 Agent Context Meter 调用；生产 TypeScript/Vite build 与定向测试通过，未运行浏览器端到端流程。
- Cargo 未使用依赖扫描不检查未使用的公开 API；Rust workspace 检查只编译当前平台的默认生产目标，不能证明跨平台运行或外部二进制调用不存在。
- 仓库中第三方传递依赖的重复版本、仍在使用的 legacy 输入/数据库迁移、4 个具有不同终止语义的 Worker 生命周期代码，本批保留并记录为后续人工架构审阅对象；本次没有对它们做广泛删除。
