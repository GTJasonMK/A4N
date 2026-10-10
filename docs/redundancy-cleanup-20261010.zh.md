# 冗余审计与清理记录（2026-10-10）

## 范围与方法

- 工作分支：`agent/redundancy-cleanup-20261010`，基线 `debug@3f6a2408c9e78c4719259e77d5443f2befdd8fda`。
- 建立分支本地 CodeGraph 索引后持续同步；当前索引覆盖 6,245 个文件、86,293 个符号、275,099 条边，状态为 up to date。
- 从 `a4n-server`、Tauri 本地 Server、`a4n-component-host`、Worker/Sidecar catalog、Axum route 注册、`a4n-control` 动态 command 分派和 `frontend/src/App.tsx` 路由入口追踪生产调用与注册点。前端索引包含 444 个 TS 和 318 个 TSX 文件。
- 基线 Cargo workspace 有 213 个 crate manifest；本次增加 Worker runtime 与 DB adapter 后共 215 个。`cargo metadata` 核对了 Worker 对共享配置/runtime crate 的依赖。静态搜索补查脚本启动、环境变量、历史迁移、测试 fixture、生成合同脚本和运行时路由字符串。
- 检查了 `#[allow(dead_code)]`、兼容/legacy 命名、候选无入边符号、Cargo 直接依赖和可复用的前端格式工具。

R-74 的 CodeGraph sync 分两轮同步了 17 和 6 个文件；首轮报告 52 个 unresolved references，第二轮解析 25 个待处理引用。R-75 同步 6 个文件并解析 2 个待处理引用。R-76 首轮同步 27 个 Worker/helper 文件时报告 83 个 unresolved references；最终 helper/test 同步后报告 10 个。R-77 更新后 sync 报告 11 个 unresolved references；R-78 更新后报告 48 个；R-79 更新后报告 25 个。R-80 最新 sync 解析了 6 个 pending references；R-81 更新并同步 1 个文件。当前索引 up to date，但 CLI 不显示剩余 unresolved references 的总数或符号。跨 crate helper 导入和部分动态/宏式 route 注册仍可能无法形成完整调用边，例如共享 runtime helper 的包外调用需要用静态搜索复核。R-74—R-81 的消费者均另经全仓静态搜索确认，并由受影响 workspace package 编译覆盖。图谱孤立符号也包含 trait 实现、路由 handler 和测试入口，未把“无入边”直接当作删除证据。

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

### R-10：收敛 Worker 失败值与 wire 转换

- 类别：重复协议实现。
- 基线共有 29 个 Worker crate 各自定义 `WorkerFailure { code, message, retryable }` 和重复构造器。BFS 从 Worker catalog/路由注册进入 command handler，沿 `Result<_, WorkerFailure>` 追到数据库错误映射、principal/scope 校验和 failed response builder；逐处确认生产调用最终只生成同一个 `WorkerError` wire 结构。覆盖 admin、agent、asset graph、canvas record、chapter content/scene、config read、inspiration、manga record、blueprint、foreshadowing、novel planning、outline、character mutation、object profile、portrait、profile material、scene profile、project access/collaboration/mutation/read/status、source snapshot、theme read、import review、video record、workflow lifecycle/read worker。
- 唯一值类型与构造规则现位于 `a4n-worker-contract::WorkerFailure`，字段封装，提供唯一的 invalid request、unauthorized、not found、conflict、unavailable 和 internal 构造入口，以及消息只读访问器；通过 `From<WorkerFailure> for WorkerError` 转换 wire 字段。29 个本地 struct/impl 已移除；失败响应保留原 request/trace/operation identity 和失败状态，改为 `failure.into()`。CodeGraph 将其解析为一个结构及 503 个依赖关系；路由调用与构造器消费另经逐 crate 静态引用搜索核对。
- 逐项保留语义差异：Admin 的 `forbidden` 原本就是 `Unauthorized`，现改用该唯一构造器；Blueprint `database` 原本就是 `unavailable(error.to_string())`，现直接调用后者；Agent/Foreshadowing/Canvas 的 Internal 保持 `Internal`；Canvas 和其他 Worker 的 `Internal`/`InvalidState` 分类没有统一重映射。Artifact Media 直接按动态结果构造 `WorkerError`，保留其 retryable 策略，不纳入静态构造器。
- 新增 contract 单测覆盖六种错误码、retryable 和消息转换；contract 46 项、worker client 14 项通过；workspace all-targets check 通过，无告警，覆盖全部 Worker producer 与 Control/Host consumer。
- 记录文件：`crates/process/a4n-worker-contract/{src/wire.rs,src/wire_tests.rs}`、29 个 Worker crate 的失败类型/响应模块及相应专属调用点。

### R-11：集中 Worker 完成/失败响应元数据装配

- 类别：重复协议响应实现。
- BFS 从 Worker catalog 和 Axum 路由注册进入 29 个 Worker command handler，沿 `completed`/`failed` helper、`WorkerResponse` 构造和 Host/Control 消费者追踪 request、trace、operation identity；逐处搜索 `WorkerResponse` 直接字面量、`WorkerOperationRef` 装配、导入链和测试入口，确认重复只存在于响应元数据装配，不改变各 Worker 的业务 payload、错误分类或 HTTP 未授权路径。
- `a4n-worker-contract::WorkerResponse` 新增 `completed_for` 与 `failed_for`，统一 schema version、request/trace ID、operation ID、空 task ID、状态、payload/error 和其余可选字段的初始化。29 个 Worker 的本地 builder 改为薄转发，共删除约 1,033 行重复 wire 初始化；Canvas Record 为保留 `RequestMeta` 的所有权边界使用 `completed_for_parts`/`failed_for_parts`，避免重新借用或复制完整请求 payload。
- 保留所有本地 helper 名称和调用关系，避免扩大路由/响应转换改动；共享入口只负责协议字段，业务错误仍由各 Worker 先映射为 `WorkerFailure`，再通过既有 `From<WorkerFailure> for WorkerError` 转换。未新增默认值、兼容分支、异常吞并或重试路径。
- 新增合同测试覆盖成功/失败响应的 request、trace、operation identity、状态、payload/error 和 parts 构造校验；静态搜索确认 29 个生产 helper 都只转发到共享入口，生产侧唯一直接使用 parts 的路径是 Canvas Record，其他路径不再自行装配 `WorkerOperationRef`。
- 记录文件：`crates/process/a4n-worker-contract/src/wire.rs`、`wire_tests.rs`，以及 29 个 Worker 的响应 helper 文件。

### R-12：统一 Worker command envelope 基础校验

- 类别：重复请求校验实现及无效转发包装。
- BFS 从 `WorkerEnvelope` schema 合同进入 Worker catalog/Axum route，沿 command handler、`validate_request`/`request_validate`、principal/scope 校验和错误响应 builder 追踪；CodeGraph 探查 `validate_worker_command` 的全部生产调用与调用路径，文本搜索复核所有 `operation_id`/`lease.worker_id` 校验分支。29 个通用 `WorkerEnvelope` Worker 的基础校验均收敛到 `a4n-worker-contract::WorkerEnvelope::validate_worker_command`。
- 共享 helper 按原顺序执行 `WorkerEnvelope::validate`、operation ID 相等校验、lease worker ID 相等校验；保留原错误码、retryable 标志和逐 Worker 错误消息。29 个 Worker 仍传入自己的 operation、worker ID 和标签；User/Service/Admin principal、用户身份、项目范围、无 scope 规则等后续专属校验保持原位置。
- Import Review 的 `validate_mutation_request` 只有两个调用且只转发到 `validate_request`；BFS 确认没有独立变换或副作用，调用点改为直接使用完整校验函数并删除转发层。Artifact Media 使用独立请求协议，不在通用 `WorkerEnvelope` 调用集合内，保留其独立校验。
- contract 测试覆盖有效身份、错误 operation、错误 worker、schema 错误优先级及 InvalidRequest/不可重试语义。此项未新增回退、默认、宽松校验或吞错路径。
- 记录文件：`crates/process/a4n-worker-contract/{src/wire.rs,src/wire_tests.rs}`、29 个通用 Worker 的基础 request validator，以及 `crates/transfer/a4n-import-review-worker/src/mutations.rs`。

### R-13：集中 Worker HTTP command response 流程

- 类别：跨 Worker 重复的 HTTP command orchestration。
- BFS 从 23 个 Worker 的 Axum `Router`/route handler 进入 `command_response`，追踪每条 handler 到授权判断、业务 `Result`、成功/失败响应与 HTTP `Json` 返回；CodeGraph 探查本地包装器、其全部生产调用和共享 runtime helper，静态搜索复核所有 23 个包装器。22 个包装器的响应流程逐项相同；Portrait 的闭包额外接收 `WorkerState`，业务结果和响应合同相同。
- 将授权后 command 调用、保留原请求元数据、成功/失败响应构造及 JSON 返回收敛到 `a4n-worker-runtime::worker_command_response`。各 Worker 保留本地 `authorized` 实现和 `unauthorized` 响应，因此 token 比较规则、401 status/body 和“未授权不调用业务命令”行为不变；共享 helper 的测试覆盖未授权、完成、失败三条路径。
- 23 个本地 `command_response` 现在仅向共享 helper 转发 Worker-specific 授权结果、命令闭包和 401 构造器。Portrait 将 `WorkerState` clone 放在共享 helper 调用的命令闭包内，保证授权失败时不克隆 state，也不触发命令。删除了 22 组仅被旧响应 wrapper 使用的本地 `completed`/`failed` 转发函数；仍有其他活跃调用的 response helper 保留。
- 30 个 Worker manifest 原已依赖 `a4n-worker-runtime`，无需新增消费者依赖；runtime crate 新增 Axum、serde 和 worker contract 依赖，Cargo.lock 仅增加这些本地 package dependency 边，不变更三方版本。未改业务错误映射、重试策略、成功/失败 HTTP status 或鉴权凭据语义。
- 记录文件：`crates/process/a4n-worker-runtime/{Cargo.toml,src/lib.rs}`、23 个 Worker 的 command response 调用入口，以及 `Cargo.lock`。

### R-14：统一 Worker token raw-byte constant-time 比较

- 类别：重复安全边界实现及重复直接依赖。
- BFS 从 Worker health/capability 和 command 路由入口追踪 `WorkerState::authorized` 到 header extraction、token 比较及未授权响应；CodeGraph 探查所有 authorizer 调用路径，静态搜索确认 27 个 Worker 都使用相同 `HeaderValue::as_bytes()` 与 token bytes 的 constant-time compare。Agent 使用独立 `AGENT_WORKER_AUTH_HEADER`，传入共享 helper 的 header 参数后比较策略相同。
- 将 raw-byte 比较收敛到 `a4n-worker-runtime::worker_token_authorized(headers, header_name, token)`。27 个本地 authorizer 保留，作为传入 Worker-specific header name/token 的薄转发；共享 helper 对 header 缺失返回 false，比较仍使用 `subtle::ConstantTimeEq`，并保留非 UTF-8 header bytes 与 token bytes 相等时的原始匹配语义。
- 从 27 个 Worker Cargo manifest 移除仅用于该比较的直接 `subtle` 依赖，转由所有 Worker 已依赖的 runtime crate 管理；lockfile 删除对应依赖边并在 runtime package 中登记一条。Admin 的 `to_str()` 校验、Workflow Lifecycle 的可见文本校验，以及 Artifact Media 独立协议的精确字符串比较均不具备相同边界语义，逐项核对后保留。
- runtime 单测覆盖缺失 header、匹配/不匹配，以及有效 header 的非 ASCII token bytes；Worker/runtimes 的 all-targets 编译和 cargo-machete 校验依赖清理。
- 记录文件：`crates/process/a4n-worker-runtime/{Cargo.toml,src/lib.rs}`、27 个 raw-byte Worker authorizer/Cargo manifest，以及 `Cargo.lock`。

### R-15：集中 Worker user principal ID 解析

- 类别：重复身份解析与 principal 类型检查。
- BFS 从 26 个 Worker 的 route/command 校验进入本地 `user_id` helper，追踪每条路径的 principal kind 限制、subject 解析和后续权限查询；CodeGraph 探查 `WorkerEnvelope`/`WorkerPrincipal` 关系，静态搜索复核所有 helper 调用。重复实现分成两种已验证语义：21 个 Worker helper 同时要求 `User` kind 并解析正整数 subject；5 个 helper 仅解析 subject，因为上游 `validate_user_request`、`validate_request` 或固定 User operation 已先校验 kind。Agent user command 另有一条内联的相同强校验。
- `WorkerPrincipal::require_user_id` 集中 User kind 与正整数检查；`WorkerPrincipal::parse_user_id` 只负责正整数 subject 解析，供已完成 kind 校验的路径调用。21 个 helper 与 Agent command 改为前者的薄转发，5 个 parse-only helper 改为后者的薄转发；Unauthorized 错误码、不可重试标志和原消息保持一致。
- Admin caller 还需绑定 payload caller ID 并输出专属错误；Artifact Media 使用独立协议与错误字符串；Agent run result 的 `id.parse::<i64>()` 解析运行时记录 ID，不是 principal。三项均保留。
- 新增 contract 单测覆盖 User、Service、正整数 subject 和零 ID 边界；contract 49 项通过，受影响 Worker/runtime 的 all-targets 检查通过，无警告。
- 记录文件：`crates/process/a4n-worker-contract/{src/wire.rs,src/wire_tests.rs}`、26 个 Worker 的 `user_id` helper 和 Agent user-command 校验。

### R-16：统一 Worker 必选项目范围校验

- 类别：重复协议校验。
- BFS 覆盖：从 Worker catalog 与 Axum route 注册进入 command handler，追踪 `validate_request`、读写操作中的 payload project ID 校验、权限查询与后续数据库调用；复用 R-12/R-13 已核对的 Worker route/handler 注册关系。CodeGraph 同步后探查 `WorkerEnvelope::require_project_scope` 的调用关系；静态搜索复核全部引用和原有谓词，覆盖 `a4n-asset-graph-worker`、`a4n-canvas-record-worker`、`a4n-inspiration-worker`、`a4n-project-access-worker`、`a4n-project-read-worker`、`a4n-project-status-worker`、`a4n-project-mutation-worker`、`a4n-project-collaboration-worker`、`a4n-source-snapshot-worker`、`a4n-import-review-worker`、`a4n-portrait-worker`、`a4n-profile-material-worker`、`a4n-scene-profile-worker`、`a4n-object-profile-worker`、`a4n-blueprint-worker`、`a4n-foreshadowing-worker` 和 `a4n-video-record-worker` 共 17 个 Worker crate。
- 同一必选 scope 规则收敛到 `a4n-worker-contract::WorkerEnvelope::require_project_scope`：payload project ID 必须为正数，并与 envelope 的 `project_id` 完全相等。删除重复的本地比较 helper 与内联分支；项目 mutation/collaboration 的“缺少 project scope”专属错误仍在本地先行返回，然后将非空 scope 的比较委托给合同方法。
- Video Record 保留资源范围错误文案，由本地 `validate_required_project_scope` 映射到 `require_project_scope_with_message`；该函数不再自行比较或验证 ID。场景资料与 Video Record 的可选 envelope scope、Outline/Novel Planning 的 scope/payload 诊断信息、Object Profile 的资源归属授权、AgentScope 和 video-service 协议仍是不同合同，逐项确认后保留。
- 新增合同测试覆盖匹配、缺失 scope、零/负 ID 和不匹配 ID；原有 Asset Graph、Canvas Record、Video Record scope 用例改为验证共享入口及 Video 专属错误消息。未增加默认值、兼容转换、回退或吞错路径。
- 记录文件：`crates/process/a4n-worker-contract/{src/wire.rs,src/wire_tests.rs}`，以及上述 17 个 Worker 的 project-scope 调用点。

### R-17：合并 Worker 可选资源项目范围校验

- 类别：重复协议校验。
- BFS 覆盖：CodeGraph 追踪 Scene Profile 的 create/update/delete 三个调用和 Video Record 的 playback/update 两个调用；继续沿 Worker command method 追到资源读取、范围校验和后续权限查询。静态搜索遍历 `crates/` 中所有同类 `is_some_and` 条件，确认这两处原实现的条件、错误码和错误文案完全一致；Object Profile 的档案归属检查及 video-service 的 request envelope 合同不同，保留。
- 将可选 scope 规则收敛到 `a4n-worker-contract::validate_optional_project_scope(scope_project_id, resource_project_id)`：envelope scope 缺失时允许访问，资源 project ID 必须为正数，scope 存在时必须与资源 ID 相等。删除 Scene Profile 与 Video Record 两处本地分支；Video Record update 继续在移动 payload 前复制 `request.project_id`，调用顺序和部分移动边界不变。
- `WorkerFailure::InvalidRequest`、不可重试属性和“worker scope project_id 与资源 project_id 不一致或无效”错误文案保持一致。新增 contract 测试覆盖 scope 缺失/匹配/不匹配以及零、负资源 ID。
- 记录文件：`crates/process/a4n-worker-contract/{src/wire.rs,src/wire_tests.rs}`、`crates/profiles/a4n-scene-profile-worker/{src/lib.rs,src/mutations.rs}`、`crates/video/a4n-video-record-worker/src/lib.rs`。

### R-18：统一 Worker client User principal 构造

- 类别：跨 client 重复身份构造和边界校验。
- BFS 覆盖：从 `WorkerPrincipal` wire 合同及 Project Access/Project Read/Workflow Read 三处重复 `user_principal` helper 进入，追踪其 7 个端口操作调用；CodeGraph 无法完整解析跨 crate 引用，静态搜索遍历全部 `*-worker-client/src`，核对 53 个共享 helper 调用和所有 `WorkerPrincipalKind::User` 构造。覆盖 `a4n-agent-worker-client`、`a4n-asset-graph-worker-client`、`a4n-canvas-record-worker-client`、`a4n-chapter-content-worker-client`、`a4n-chapter-scene-worker-client`、`a4n-config-read-worker-client`、`a4n-inspiration-worker-client`、`a4n-manga-record-worker-client`、`a4n-blueprint-worker-client`、`a4n-foreshadowing-worker-client`、`a4n-novel-planning-worker-client`、`a4n-outline-worker-client`、`a4n-character-mutation-worker-client`、`a4n-object-profile-worker-client`、`a4n-portrait-worker-client`、`a4n-profile-material-worker-client`、`a4n-scene-profile-worker-client`、`a4n-project-access-worker-client`、`a4n-project-collaboration-worker-client`、`a4n-project-mutation-worker-client`、`a4n-project-read-worker-client`、`a4n-project-status-worker-client`、`a4n-source-snapshot-worker-client`、`a4n-theme-read-worker-client`、`a4n-import-review-worker-client`、`a4n-video-record-worker-client` 和 `a4n-workflow-read-worker-client` 共 27 个 Worker client crate。
- `a4n-worker-client::user_principal` 统一 user-only 正数校验和 User principal 构造；`user_principal_with_scope` 统一 required/optional project scope 场景，调用方传入原错误文案以保留各 client 的既有合同。删除 3 个本地 principal factory、Config Read 的本地 `user` helper，以及所有标准 client 中内联的 User principal 构造和重复 `user_id` 分支。
- Admin client 的 caller principal 与 payload caller ID 绑定由 Admin Worker 的服务端校验并返回 Unauthorized，保留其专属路径；Service principal、合作邀请中的 `member_user_id` 验证和 transport 测试 fixture 也不属于此 User principal factory。
- 新增 worker-client 测试覆盖 User principal 构造、零/负 ID、scope 缺失/匹配/不匹配及原错误文案。第一次 60 秒冷构建在编译期超时；`cargo test --no-run` 完成后，限时测试 16/16 通过。
- 记录文件：`crates/process/a4n-worker-client/{src/lib.rs,src/tests.rs}` 与上述 27 个 Worker client 的 command principal 调用点。

### R-19：统一 Worker client 正数 ID 校验

- 类别：重复输入校验。
- BFS 覆盖：从 Worker client 的 project ID validation 进入 Project Read/Access/Collaboration/Mutation operation，追踪其 `AppError::InvalidArgument` 返回及后续 envelope/payload 构造；CodeGraph 从 `ProjectCollaborationApplicationPort` 的 10 个业务操作追踪到 reads/mutations，静态搜索复核所有 Worker client 的 project ID 谓词及字段错误字符串，共 24 个共享调用、11 个源文件，覆盖 `a4n-chapter-content-worker-client`、`a4n-character-mutation-worker-client`、`a4n-import-review-worker-client`、`a4n-inspiration-worker-client`、`a4n-manga-record-worker-client`、`a4n-project-access-worker-client`、`a4n-project-collaboration-worker-client`、`a4n-project-mutation-worker-client`、`a4n-project-read-worker-client` 和 `a4n-workflow-read-worker-client`。
- `a4n-worker-client::positive_id(field, value)` 统一必选正数检查，`optional_positive_id` 统一可选 ID 检查并保留 `None`；`ProjectCollaborationWorkerClient` 的 project/invite/member helper 和 Project Mutation client 的 `positive_id` wrapper 已删除。所有直接 `project_id <= 0` 分支改用共享函数，字段名驱动的错误文案保持不变。`user_principal_with_scope` 与这些通用检查共用同一内部正数谓词，但仍保留 user/project 联合校验所需的专属错误策略。
- CodeGraph 中 `a4n-domain` 的 `positive_id` 是 Strong ID 构造器并返回 `DomainError`，不属于 Worker client 的 `AppError` 边界，保持独立；server-side command 校验也继续保留。
- 新增 contract-client 测试覆盖正数、零/负数、字段错误名及可选 scope 的 `None`。静态搜索确认 Worker client crate 内不再保留第二份 project ID 正数比较。
- 记录文件：`crates/process/a4n-worker-client/{src/lib.rs,src/tests.rs}`、上述 10 个 Worker client crate 的 ID validation 调用点。

### R-20：统一 Worker 服务端正数 ID 校验

- 类别：重复输入校验。
- BFS 覆盖：从 6 个 Worker 的 catalog/route handler 进入 command 方法，沿 `positive`/`positive_id` helper 追到 payload 构造、数据库查询和权限校验；CodeGraph 与静态搜索确认这些 helper 共有 30 余个生产调用，且错误类型、错误码、不可重试标志和文案完全一致。覆盖 `a4n-chapter-content-worker`、`a4n-manga-record-worker`、`a4n-outline-worker`、`a4n-character-mutation-worker`、`a4n-video-record-worker` 和 `a4n-project-collaboration-worker`。
- 将规则收敛到 `a4n-worker-contract::WorkerFailure::require_positive_id(value, field)`，统一返回原值或 `InvalidRequest` 的 `{field} 必须大于 0`。删除 6 个 Worker 的本地 helper；所有调用继续在原数据库/权限操作之前执行。配置读取 Worker 的专属“配置 ID 必须为正整数”文案、章节序号/范围校验及其他业务数值规则保持独立。
- 合同测试覆盖正数、零、负数、错误码和不可重试属性；未增加默认值、兼容分支或吞错路径。
- 记录文件：`crates/process/a4n-worker-contract/src/wire.rs`、`wire_tests.rs`，以及上述 6 个 Worker 的 validation/protocol/command 调用点。

### R-21：复用 Chapter client 正数 ID 校验

- 类别：重复输入校验。
- BFS 覆盖：从 `ChapterContentWorkerClient` 和 `ChapterSceneWorkerClient` 的 application port 实现进入 reads/mutations，追踪每个 `Self::positive` 调用到 envelope/payload 构造；CodeGraph 与静态搜索确认两处 helper 都只服务于本 client，且与 `a4n-worker-client::positive_id` 的 `AppError::InvalidArgument`、正数边界和错误文案完全一致。内容 client 覆盖 6 个调用，场景 client 覆盖 3 个调用。
- 删除两个 client 的本地 helper，全部调用改用现有共享 `a4n-worker-client::positive_id`；移除场景 client 中随旧 helper 遗留且无引用的 `WorkerPrincipal`/`WorkerPrincipalKind` 导入。没有改变 optional project scope、User principal 或 transport 错误策略。
- 记录文件：`crates/chapter/a4n-chapter-content-worker-client/{src/lib.rs,src/reads.rs,src/mutations.rs}`、`crates/chapter/a4n-chapter-scene-worker-client/{src/lib.rs,src/reads.rs,src/mutations.rs}`。

### R-22：收敛剩余 Worker 正数 ID 边界分支

- 类别：重复输入校验。
- BFS 覆盖：从 Portrait、Workflow Read、Import Review、Theme Read 的 route handler 进入具体 command，沿 payload ID 校验追到数据库读取、项目权限和 service principal 分支；静态搜索复核所有同文案 `<= 0` 分支。确认 7 个边界分支都返回 `WorkerFailure::InvalidRequest`、不可重试且文案分别为 `{field} 必须大于 0`。
- 将 Portrait 的 profile/portrait ID、Workflow Read 的 workflow/project ID、Import Review 的 review ID、Theme Read 的 theme ID 改用 `WorkerFailure::require_positive_id`。保留 Workflow 的 `limit` 校验、章节号/范围专属文案、配置 ID 专属文案及 Artifact Media 的独立字符串协议。
- 调用顺序仍在任何数据库查询或权限检查前执行；可选 project scope 仍允许 `None`。未增加默认值、兼容转换或回退路径。
- 记录文件：`crates/profiles/a4n-portrait-worker/{src/reads.rs,src/mutations.rs}`、`crates/workflow/a4n-workflow-read-worker/src/reads.rs`、`crates/transfer/a4n-import-review-worker/src/lib.rs`、`crates/theme/a4n-theme-read-worker/src/lib.rs`。

### R-23：删除 Worker user principal 直通包装

- 类别：重复身份访问包装和无效转发层。
- BFS 覆盖：从 27 个 Worker route/command 入口沿 `user_id` 调用追到 `WorkerPrincipal::{require_user_id,parse_user_id}`，再到权限查询或 Admin/Service 资格判断；静态搜索确认 27 个本地函数均只有单条直通表达式，生产调用合计 145 处。保留 Artifact Media 的 `actor_user_id`，其解析结果使用字符串错误，并同时验证 user/project pair 和 request kind，不是直通包装。
- 删除 27 个 `user_id` wrapper，调用端直接使用 principal 合同方法；保留 5 个 parse-only Worker 对 Service numeric subject 的原有接受行为，其余仍要求 User principal。删除随 Chapter Content helper 删除后无用的导入；Manga access 模块仍保留真实的权限 helper。
- 首轮 workspace all-targets 检查捕获 4 个残留 import；移除后重新对 27 个消费 Worker 执行 all-targets 检查通过。身份语义、检查顺序、错误码和消息均不变。
- 记录文件：27 个 Worker 内部 `user_id` wrapper/调用点；不包含拥有独立行为的 Artifact Media `actor_user_id`。

### R-24：清理 Worker client 身份 helper 遗留导入

- 类别：显性死代码。
- BFS 覆盖：从 R-18 的 `a4n-worker-client::user_principal` 集中构造进入 27 个 Worker client crate，静态遍历 `WorkerPrincipal`/`WorkerPrincipalKind` 所有引用；其中 14 个 client 仅在 `use` 中保留旧类型，没有任何生产、fixture 或测试引用。全 workspace all-targets 检查也对这些导入发出 unused-import warning。
- 删除 `a4n-agent-worker-client`、`a4n-asset-graph-worker-client`、`a4n-canvas-record-worker-client`、`a4n-blueprint-worker-client`、`a4n-foreshadowing-worker-client`、`a4n-novel-planning-worker-client`、`a4n-outline-worker-client`、`a4n-object-profile-worker-client`、`a4n-portrait-worker-client`、`a4n-profile-material-worker-client`、`a4n-scene-profile-worker-client`、`a4n-project-status-worker-client`、`a4n-source-snapshot-worker-client` 和 `a4n-theme-read-worker-client` 共 28 个无用类型导入（14 个文件）。保留仍自行构造 Service/User principal 的 Admin、Config Read、Project Access/Read、Video Record、Workflow Read/Lifecycle clients。
- 记录文件：上述 14 个 client 的 `src/lib.rs`。

### R-25：统一 Collaboration DB 错误映射

- 类别：跨 Worker 重复错误适配。
- BFS 覆盖：从 24 个 Worker 的 route/command 调用进入本地 `collaboration_error`，追踪 `CollaborationDbError` 的五种变体到 `WorkerFailure` wire code/message/retryable；CodeGraph 加静态搜索核对全部 24 个生产映射及调用点。22 个映射的变体分类完全相同；Canvas Record 和 Foreshadowing 对 `Internal` 保持 `Internal`，其余 22 个把 `Internal` 与 `InvalidState` 同映射为 Conflict。`Database` 都以底层 `DbErr::to_string()` 映射为 retryable `Unavailable`；Video Record 原本转入本地 `database_error`，经逐项核验行为相同。
- 新增 `crates/process/a4n-worker-db-adapter`，将 DB 错误到 Worker wire failure 的边界放在 process adapter 层，不让 `a4n-db` 或通用 worker contract 依赖彼此。共享 match 只有一份；默认入口保留 Conflict 规则，第二入口明确保留 Internal。R-25 当时保留 24 个本地 `collaboration_error` 薄委托；这些转发层已由 R-38 删除，调用点现直接选择对应 adapter mapper。其他 DB 错误类型和 worker-specific 错误映射未迁入。
- adapter 回归测试覆盖 InvalidArgument、NotFound、Forbidden、InvalidState、Internal 两种策略、Database message、wire code 和 retryable。adapter 测试 2/2 通过；24 个 Worker 与 adapter 的 all-targets 检查通过且无警告。
- 记录文件：`Cargo.toml`、`Cargo.lock`、新 adapter crate、上述 24 个 Worker 的 manifest 与错误 mapper。

### R-26：统一 Worker principal kind 校验

- 类别：重复身份边界校验。
- BFS 覆盖：从 Worker route/command validators 进入 principal kind 条件，追踪 Unauthorized 返回到 HTTP Worker response；静态搜索复核 `require_user_id`、Admin caller、Config Read、Inspiration、Import Review、Project Access/Read、Workflow Read/Lifecycle 的所有严格 kind guard。其谓词都只是枚举相等判断，差异仅为所需 kind 和本地错误文案。
- `WorkerPrincipal::require_kind(kind, error_message)` 成为唯一 kind 比较入口；`require_user_id` 委托给它后继续调用 `parse_user_id`。各 Worker 保留原文案和错误顺序；Project Read 的 User/Service 分支仍按原参数选择，Admin 随后仍执行 caller ID 与 payload ID 绑定。Project Access 的 runtime subject 联合校验及 Artifact Media 的 User-or-Service 协议不是单一 kind guard，保留。
- 新增合同测试覆盖匹配类型、不匹配类型、自定义文案、Unauthorized code 和不可重试属性；`a4n-worker-contract` 53/53，8 个消费 Worker 的 all-targets 检查通过且无警告。
- 记录文件：`crates/process/a4n-worker-contract/{src/wire.rs,src/wire_tests.rs}`，以及 Admin、Config Read、Inspiration、Import Review、Project Access/Read、Workflow Read/Lifecycle Worker 的 principal validator。

### R-27：统一 Worker 非空文本校验

- 类别：重复输入边界校验。
- BFS 覆盖：从 6 个 Worker 的 route/command 到字段消费者追踪空白判断、规范化后的值、持久化调用与错误响应；CodeGraph/静态搜索核对 8 个调用点。它们都按 Unicode `trim()` 判空并返回不可重试的 InvalidRequest，错误文案由字段/操作传入。Video Update 对空字符串使用原始 `is_empty()`、不拒绝纯空格，保留其不同合同。
- 新增 `WorkerFailure::require_non_blank(value, error_message)`，返回 trim 后的借用文本；`validate_non_blank` 与该入口共用唯一 `trimmed_non_blank` 谓词。章节正文/场景类型、角色名、快照哈希、主题名和 asset graph node ID 消费裁剪后的返回值；取消原因、active task ID、协作邀请身份仅校验并保留下游原始值。删除 Chapter Scene 单调用 `required_text` wrapper。
- 合同测试覆盖边缘空白/全空白、返回的裁剪值、InvalidRequest code、原错误文案及 retryable=false；contract 54/54，6 个受影响 Worker all-targets 检查通过且无警告。
- 记录文件：`crates/process/a4n-worker-contract/{src/wire.rs,src/wire_tests.rs}`，以及 Asset Graph、Chapter Scene、Workflow Lifecycle、Character Mutation、Theme Read 和 Project Collaboration Worker 的字段校验调用点。

### R-28：统一 Project Mutation DB 错误映射

- 类别：重复错误适配。
- BFS 覆盖：从 Project Mutation DB API 的 6 个 Worker caller 追到 `ProjectMutationError` 各变体、`WorkerFailure` wire code 与 response builder；CodeGraph caller 路径和静态引用搜索确认消费者为 Chapter Content、Blueprint、Project Mutation、Project Read、Project Status、Source Snapshot Workers。六份 mapper 对 InvalidArgument、NotFound、Forbidden、Internal、Database 的分类、文案和 retryable 完全相同。
- 在 R-25 的 `a4n-worker-db-adapter` 增加唯一 `map_project_mutation_error` match；六个 Worker 的本地 mapper 收敛为薄转发，调用名与各 command 的错误边界保持不变。未把 Project Status 的 StatusWrite/Reset 错误或其他 Project DB error 迁入，因为它们的变体合同不同。
- adapter 新测试覆盖五种变体的 wire code、消息与 retryable；adapter 测试 3/3，六个消费者与 adapter all-targets 检查通过且无警告。
- 记录文件：`crates/process/a4n-worker-db-adapter/src/lib.rs`，以及上述 6 个 Worker 错误映射文件。

### R-29：统一 Workflow Read 与 Project Family DB 错误映射

- 类别：重复错误适配。
- BFS 覆盖：从 `WorkflowReadError` 和 `ProjectFamilyDbError` 两个 DB 合同追到各 Worker 的本地 mapper、`WorkerFailure` wire 分类和 response builder；CodeGraph caller 路径及仓库静态引用核对确认触及 Chapter Content、Manga Record、Video Record、Workflow Read、Novel Planning、Portrait 六个 Worker。
- 将重复的 Project Family DB 映射收敛到 adapter；Workflow Read 复用同一映射主体，并用显式 `InvalidDataPolicy` 保留两条路径原有的 Conflict 与 InvalidRequest 分类。其余 InvalidArgument、NotFound、Database 的 wire code、消息与 retryable 维持原合同。
- adapter 测试覆盖两种 `InvalidData` 分类及其他错误变体；测试 6/6，adapter 与 6 个消费者 all-targets 检查通过且无警告。
- 记录文件：`crates/process/a4n-worker-db-adapter/src/lib.rs`，以及上述 6 个 Worker 的错误映射文件。

### R-30：统一 typed Worker client 响应状态解码

- 类别：重复响应处理。
- BFS 覆盖：从 `HttpWorkerClient::command` 追到请求合同校验、HTTP JSON 解析、`validate_command_response`、`WorkerResponse::validate` 及 request/trace identity 核验，再静态枚举所有 typed client 对 Completed/Failed/其他状态的消费点。共覆盖 `a4n-agent-worker-client`、`a4n-admin-worker-client`、`a4n-asset-graph-worker-client`、`a4n-canvas-record-worker-client`、`a4n-chapter-content-worker-client`、`a4n-chapter-scene-worker-client`、`a4n-config-read-worker-client`、`a4n-inspiration-worker-client`、`a4n-manga-record-worker-client`、`a4n-blueprint-worker-client`、`a4n-foreshadowing-worker-client`、`a4n-novel-planning-worker-client`、`a4n-outline-worker-client`、`a4n-character-mutation-worker-client`、`a4n-object-profile-worker-client`、`a4n-portrait-worker-client`、`a4n-profile-material-worker-client`、`a4n-scene-profile-worker-client`、`a4n-project-access-worker-client`、`a4n-project-collaboration-worker-client`、`a4n-project-mutation-worker-client`、`a4n-project-read-worker-client`、`a4n-project-status-worker-client`、`a4n-source-snapshot-worker-client`、`a4n-theme-read-worker-client`、`a4n-import-review-worker-client`、`a4n-video-record-worker-client`、`a4n-workflow-read-worker-client`、`a4n-workflow-lifecycle-worker-client` 共 29 个 client crate；逐一确认它们依赖 `a4n-worker-client`。
- 将重复状态分支收敛到 `a4n-worker-client::worker_response_payload`。每个调用仍传入自己的 worker 标签、operation 上下文和原 `worker_response_error` mapper；helper 保留完成 payload 缺失、Failed 与非终态的原错误类别及原诊断文案。传输层先行校验 Failed 必须带错误，因此保留既有 `expect("validated failed response")` 前置条件。
- Inspiration 与 Source Snapshot 原本不带 operation 的诊断格式通过显式 `None` 保留。Artifact Media client 使用专属 `ArtifactMediaClientError` 和不同的缺失错误合同，故不并入；Worker Runtime 的通用响应入口也没有 typed payload 分支，不改。
- 修改后对 `crates/` 做状态分支全仓搜索，只剩共享 decoder 与 Artifact Media 专属 decoder 两处生产 match；其余命中为 wire 构造/合同校验或测试。
- 新增 helper 测试覆盖完成 payload、缺失 payload、Failed mapper 透传，以及带/不带 operation 的 Running 错误文案；`a4n-worker-client` 测试 18/18，`a4n-worker-client` 与全部 29 个消费者 all-targets 检查通过且无警告。
- 记录文件：`crates/process/a4n-worker-client/{src/lib.rs,src/responses.rs,src/tests.rs}`，以及上述 29 个 client crate 的 `src/lib.rs`。

### R-31：统一 Chapter Content 与 Video Record AppError 映射

- 类别：重复错误适配。
- BFS 覆盖：静态枚举并对比 9 个 Worker 的 `AppError -> WorkerFailure` mapper，再沿 Chapter Content 的 3 个 `map_err` 调用点（Reads、Mutations）与 Video Record 的 2 个 prompt 调用点追到 Worker 响应路径。Chapter Content 与 Video Record 使用完全相同的 `InvalidArgument`、`NotFound`、`Forbidden`、`InvalidState` 和其他错误映射；静态核对两份 Cargo.toml 确认这两处消费者均已依赖 `a4n-worker-db-adapter`。
- 新增共享 `map_app_error`，两处调用直接使用它，删除局部 mapper；错误码、`InvalidState` 消息格式及 retryable 保持一致。测试特意固定两处原有的 `AppError::Unauthorized -> Unavailable` 行为。
- 其余 7 个 mapper 保持局部映射：Agent Health 使用 Internal 回退和不同的 Conflict 消息；Asset Graph 改写 operation/current 顺序；Config Read 会映射 Unauthorized 并保留 InvalidState operation；Admin/Workflow Read 的变体分类不同；Import Review/Theme Read 把部分 ConfigError 映射为 Conflict。未强行统一这些合同。
- Chapter Content 与 Video Record 删除已无直接引用的 `a4n-error` Cargo 依赖；adapter 增加无默认 features 的 `a4n-error` 依赖，Cargo.lock 仅更新依赖列表。
- adapter 测试 7/7；adapter、Chapter Content Worker、Video Record Worker all-targets 检查通过且无警告；`cargo machete` 未发现未使用依赖。
- 记录文件：`crates/process/a4n-worker-db-adapter/{Cargo.toml,src/lib.rs}`，Chapter Content 与 Video Record Workers 的 Cargo.toml、mapper 和调用点，以及 `Cargo.lock`。

### R-32：合并重复的 Outline DB 错误转换

- 类别：重复错误适配。
- BFS 覆盖：从 `OutlineDbError` 和 `ProjectOutlineRecordError` 的变体定义追到 mutation 与 cleanup 中两份 `map_outline_error`，静态调用搜索确认共有 4 个调用点：手工章节大纲保存时的分部查询、手工分部大纲保存时的记录校验和章节归属同步，以及删除分部大纲前的记录校验。两份 match 对 Database、InvalidArgument、InvalidState、Generation 的输出完全相同。
- 在 `ProjectOutlineRecordError` 实现唯一 `From<OutlineDbError>` 转换；四处调用改用 `?`，删除 mutation/cleanup 两份 mapper。明确保留 `OutlineDbError::Generation` 转为 InvalidArgument 的旧合同，未使用目标 enum 上已有的 `Generation` 自动转换。
- 新增单测覆盖四个源错误变体。`a4n-db` all-targets check 通过；首次定向测试在冷编译阶段到 60 秒上限，预编译测试二进制后定向用例 1/1 通过。
- 记录文件：`crates/platform/a4n-db/src/project/outline_records/{contracts.rs,mutation.rs,cleanup.rs}`。

### R-34：统一 Project Summary 的 Progress/Family 错误映射

- 类别：重复错误适配。
- BFS 覆盖：从 `ProjectSummaryError` 的三个 Worker 消费者（Project Read、Project Mutation、Project Status）追到共享的 `ProjectProgressReadError`、`ProjectFamilyMetadataReadError` 变体与 batch-read 调用点。静态搜索同时核对 Project Read Worker `reads.rs` 的 3 处 Progress/Family 转换；三个 Worker 对这些源错误使用相同 wire code、消息和 retryable。
- 在 `a4n-worker-db-adapter` 新增 `map_project_progress_read_error` 与 `map_project_family_metadata_read_error`，统一三个 Worker 的错误分支。Progress 的每种现有持久化无效状态继续转 Conflict，Database 继续只传底层错误消息；Family 的缺根项目/缺源快照继续转 Conflict。Progress mapper 显式列出当前全部非数据库变体，新增枚举变体会要求调用方重新分类。
- 新增 adapter 测试覆盖 InvalidArgument、Conflict、Database 及 retryable；adapter 测试 9/9，adapter 与三个 Worker all-targets 检查通过。
- 记录文件：`crates/process/a4n-worker-db-adapter/src/lib.rs`，以及 Project Read、Project Mutation、Project Status Workers 的 summary/error call sites。

### R-35：统一 Profile DB 错误到 Worker wire 的映射

- 类别：重复错误适配。
- BFS 覆盖：从 Object Profile、Profile Material、Scene Profile 三个 Worker 的 catalog/route handler 进入 `reads.rs` 与 `mutations.rs`，沿所有数据库 `Result` 的 `map_err` 调用追到 `WorkerFailure`/`WorkerResponse` 失败路径；静态搜索枚举三种 DB error 的全部生产消费者，并核对 Cargo 依赖、模块导入和测试入口。共覆盖 9 个 Worker 源文件、1 个共享 adapter 源文件、29 个 `map_err` 调用：`AssetDbError` 由 Object Profile 与 Profile Material 消费，`ProfileMaterialDbError` 由三个 Worker 消费，`SceneProfileDbError` 由 Profile Material 与 Scene Profile 消费。
- `AssetDbError` 与 `SceneProfileDbError` 的 `InvalidArgument`、`NotFound`、`InvalidState`、`Database` 映射完全一致；`ProfileMaterialDbError` 的 `InvalidArgument` 与 `Database { source, .. }` 映射在三个 Worker 中完全一致。共享 adapter 分别新增 `map_asset_db_error`、`map_profile_material_db_error`、`map_scene_profile_db_error`，删除三个 Worker 中共 7 份局部 mapper，所有调用点直连唯一实现。错误码、`{operation}: {current}` 冲突文案、数据库 source 文案和 retryable 标志保持原值。
- Orchestrator 的 Asset/Profile Material/Scene Profile 到 `AppError` 映射仍保留：它们跨越不同 wire/API 边界，`ProfileMaterialDbError` 的 operation 信息和 Host `AppError` 合同不能与 WorkerFailure 合并。没有新增默认值、兼容转换、回退或吞错路径。
- adapter 测试覆盖三种 enum 的全部变体、wire code、消息和 retryable；三 Worker 的 route/response 链只改变映射函数来源，不改变授权、范围校验、数据库调用或 payload。
- 记录文件：`crates/process/a4n-worker-db-adapter/src/lib.rs`，以及 Object Profile、Profile Material、Scene Profile Workers 的 `src/lib.rs`、`reads.rs`、`mutations.rs`。

### R-36：统一通用数据库不可用错误映射

- 类别：重复错误适配。
- BFS 覆盖：从 Portrait、Import Review、Video Record 三个 Worker 的 route/command handler 进入 reads、mutations、prompts 和 Video DB error 分支，逐处追踪 `database_error` 的生产调用和最终 `WorkerResponse` 失败路径。静态搜索确认共 15 个调用点，全部来自数据库查询/持久化 `Result` 或 `VideoDbError::Database`，没有将 NotFound、权限或业务校验错误送入通用映射。
- 三个 Worker 原有的 `database_error(error: impl Display)` 逐字相同，均生成 retryable `Unavailable` 并透传 `Display` 文案。新增 `a4n-worker-db-adapter::map_database_error` 作为唯一实现，删除三份局部 wrapper；Video Record 的 `VideoDbError` 其他变体仍保留本地分类和文案。
- adapter 测试固定 `Unavailable` wire code、retryable=true 和 SeaORM `DbErr` 的实际 Display 文案；没有新增默认值、兼容分支、回退或吞错路径。
- 记录文件：`crates/process/a4n-worker-db-adapter/src/lib.rs`，以及 Portrait、Import Review、Video Record Workers 的 `src/lib.rs`、`reads.rs`、`mutations.rs`、`prompts.rs`。

### R-37：收敛 Worker 内联数据库不可用映射

- 类别：重复错误适配。
- BFS 覆盖：从 Chapter Scene/Content、Project Read、Profile Material、Blueprint、Manga Record、Character Mutation 七个 Worker 的 route/command handler 进入数据库查询、事务和快照存储调用，沿 25 处内联 `map_err(|error| WorkerFailure::unavailable(error.to_string()))` 追到失败响应；逐文件核对其错误源均为持久化或存储访问错误，且七个 Cargo manifest 已依赖 adapter。
- 25 处闭包改为直接调用 `a4n_worker_db_adapter::map_database_error`，与 R-36 的 15 个调用共享唯一 `Unavailable` 映射。保留每个调用点后续的 NotFound、权限、范围和业务错误分支，不改变查询结果、事务边界或快照缺失文案。替换后仓库中不再有该内联映射模式。
- 本项不扩展到带多变体的本地 `match`：`WorkflowWriteError`、`CanvasDbError`、`CharacterDbError` 等仍需保留各自业务分类，已在 R-33/既有保留项记录。
- 记录文件：`crates/process/a4n-worker-db-adapter/src/lib.rs`，以及上述七个 Worker 的 `src/` 调用文件。

### R-38：删除 Collaboration DB mapper 转发包装

- 类别：无行为的转发层。
- BFS 覆盖：从 `CollaborationDbError` 的 DB 变体和 R-25/R-26 adapter 映射合同出发，静态枚举所有 24 个 Worker 对本地 `collaboration_error` 的生产调用，追到 command/权限/Project Summary 失败响应，并检查跨模块 `pub(super)` helper、测试引用和 Cargo 依赖。
- 删除 22 个仅转发 `map_collaboration_db_error` 的 wrapper，调用点直接使用 adapter。Canvas Record 与 Foreshadowing 的 wrapper 原本分别转发 `map_collaboration_db_error_preserving_internal`；改为调用点显式选择同一 mapper，并把 Foreshadowing 回归测试改为直接断言 adapter。两种 Internal 分类策略仍只定义在 adapter 内。
- 未改普通 Collaboration 错误映射，也未改 Canvas Record/Foreshadowing 对 Internal 的保留分类；现已不存在 Worker 本地 `collaboration_error` 符号。
- 记录文件：`crates/process/a4n-worker-db-adapter/src/lib.rs` 与上述 24 个 Worker 的 Collaboration caller 文件。

### R-39：删除其他 DB adapter 的单行转发 wrapper

- 类别：无行为的转发层。
- BFS 覆盖：静态枚举仍直接返回 `a4n_worker_db_adapter::*_error(error)` 的 14 个 Worker wrapper，逐个搜索其全部调用点，并沿 command/read/mutation handler 追至 `WorkerFailure` response。覆盖 Chapter Content、Manga Record、Blueprint、Novel Planning、Portrait、Project Mutation、Project Read、Project Status、Source Snapshot、Video Record、Workflow Read 11 个 Worker；11 个 manifest 均已依赖 adapter。
- 删除 14 个仅隐藏 adapter mapper 名称的本地函数，调用处改为直接选择共享 mapper。ProjectMutation 与 ProjectFamily 的错误码/消息不变；WorkflowRead 的分类策略按调用方保留：Chapter Content 与 Workflow Read 使用 Conflict，Manga Record 与 Video Record 使用 InvalidRequest。共享 adapter 是策略和 wire 分类的唯一事实来源。
- 没有合并领域错误的多变体 `match`，例如 Chapter Content edit/import、Project Status write/reset/read、Video DB 等仍有独立业务状态映射；这些并非单行 adapter 转发。
- 记录文件：`crates/process/a4n-worker-db-adapter/src/lib.rs`，以及上述 11 个 Worker 的 mapper 和消费者文件。

### R-40：收敛 typed DB 错误的 Unavailable 分支

- 类别：重复错误适配。
- BFS 覆盖：从 17 个 Worker 中各类 DB error enum 的 `Database` 变体开始，核对每个变体的定义和 match 分支，再沿 mapper 调用追至 Worker failed response。共收敛 27 个显式数据库分支，覆盖 Admin、Canvas Record、Chapter Content、Chapter Scene、Config Read、Inspiration、Import Review、Manga Record、Outline、Novel Planning、Portrait、Character Mutation、Project Read、Project Mutation、Project Status、Foreshadowing、Workflow Lifecycle；相关 Worker 均已有或新增 `a4n-worker-db-adapter` 依赖。
- 这些数据库分支原来都以底层错误的 `Display` 文案构造 retryable `Unavailable`。现在统一调用 `map_database_error`，包括 `Database { source, .. }`、`DatabaseOperation { source, .. }` 和 Project Summary 的数据库错误。adapter 内既有 Asset/Profile/Scene/Project/Workflow/Collaboration DB mapper 也复用同一 helper，避免第二份 `Unavailable` 格式规则。
- 仅替换显式数据库变体；`Generation`、通用 AppError 的其他分支、InvalidData、NotFound、冲突和领域错误维持原映射。测试固定 Unavailable wire code、retryable 与原 Display 文案。
- 记录文件：`crates/process/a4n-worker-db-adapter/src/lib.rs`、`Cargo.lock`，以及上述 17 个 Worker 的 DB mapper 文件和 3 个新增 adapter 依赖 manifest。

### R-41：审查剩余边界映射组合

- 类别：核验后保留，不改代码。
- BFS 覆盖：从 `ProjectSummaryError` 的定义出发，追到 Project Read、Project Mutation、Project Status 三个 Worker 的 summary handler 和所有生产调用；同时搜索全仓把 Display 错误转为 `WorkerFailure::InvalidRequest` 的 closures。
- 三个 `ProjectSummaryError -> WorkerFailure` 组合映射逐变体一致，子类型映射已由 R-34/R-38/R-40 分别集中。将组合函数放进通用 process adapter 会使所有 adapter 消费者依赖 `a4n-project-read-core`；另建项目专用 crate 只服务三个调用点，成本高于这 4 分支 match，故保持在 Worker 边界。
- Object Profile/Scene Profile 的显示错误到 InvalidRequest 只有四处输入规范化闭包，另有两处已直接使用 `WorkerFailure::invalid_request` 处理 String。尚无所有者合适且收益足以覆盖的共享 helper，不增添新的 contract API。

### R-42：复核显性死代码与兼容入口

- 类别：全仓核验，无代码删除。
- BFS 覆盖：再次枚举 `#[allow(dead_code)]`、行注释中的代码样式、`legacy`/`compat`/`fallback` 入口，并沿可见 import、脚本调用、路由和测试 fixture 查引用；`cargo machete` 检查所有 workspace manifest。
- 三处 `allow(dead_code)` 分别用于脚本调用的 Generation schema 导出类型，以及 `cfg(test)` 项目包 record fixture。检索到的 33 行代码样式注释均为约束或未来条件说明，没有注释掉的可执行实现。
- Edge 的 Axum fallback 是 SPA index 路由；Runtime baseline config 的 fallback 参数在 env 缺失时用于播种配置；editor source / source-sync / family package / media-authoring legacy converters 分别有生产解析、迁移或导入调用。它们均为活跃路径，不按标识符中的 `fallback`/`legacy` 做症状性删除。
- `cargo machete` 未发现未使用 Cargo 依赖；本轮没有新增删除候选。

### R-43：统一 Web/Control 监听地址解析

- 类别：重复配置解析与伪安全 fallback。
- BFS 覆盖：从 Server startup configuration、`WorkerSpec.control_addr` 注入和 Agent Worker `async_main` 启动入口出发，追踪 `A4N_WEB_ADDR`/`A4N_CONTROL_ADDR` 到 Edge/Control、托管 Worker 和 Agent 自身 health listener。确认 Agent Worker 和 Server catalog 各自解析同一组环境变量并派生 Control 地址；Agent client 的 `A4N_AGENT_WORKER_URL`/旧式 `A4N_AGENT_WORKER_ADDR` 分支与此无关，保留。
- 将网页监听地址、显式 Control 地址解析及从 Web 端口派生 Control 端口收敛到 `a4n-config`。`A4N_CONTROL_ADDR` 仍优先；缺失时使用 `A4N_WEB_ADDR` 缺省值 `127.0.0.1:8080` 并派生 loopback `port + 1`，遇到网页端口 0 或 65535 时按 Server 现有规则使用 18080。显式 Control 端口 0、非法地址和非 UTF-8 值报错。
- 移除 Agent 对 `env::var("A4N_WEB_ADDR").unwrap_or_else(...)` 的错误吞并：仅变量缺失采用默认值，非 UTF-8 和非法配置现在失败；修复 Agent 在 Web 端口 0 时错误派生到端口 1 的分叉。
- 测试覆盖默认、无效/非 UTF-8 Web 地址、显式 Control 地址和 Web 端口 0/65535。`a4n-config` 定向测试 1/1；`a4n-config`、Agent Worker、Server all-targets 检查通过。
- 记录文件：`crates/config/a4n-config/src/lib.rs`、Agent Worker config/runtime、Server worker catalog/startup/process 测试。

### R-44：核验 Edge/Control 地址解析边界

- 类别：核验后保留，不改代码。
- BFS 覆盖：从 R-43 的 `a4n_config::{web_listen_addr, control_listen_addr}` 追到 `a4n-server::startup::configuration::load`，再追踪 `spawn_control_from_workers`、`spawn_edge`、`spawn_managed_worker` 注入的 Control 地址和 Agent Worker `async_main`；同时从 `a4n-edge::run`、`a4n-control::run` 分别追到环境读取、监听绑定及可执行入口。补查 Tauri `LocalServerProcess::start` 的端口预留/地址文件、CLI `spawn_server`、Server cleanup 测试、`scripts/run.sh`、`.env.example` 和中英文配置/入门文档。
- Edge 独立运行时缺省监听 Web `127.0.0.1:8080`、转发 Control `127.0.0.1:18080`；桌面 Tauri 路径则通过 `A4N_WEB_LISTEN_FILE` 为 Server/Edge 选择并发布端口，Edge 在地址文件存在时支持端口占用后的动态分配及重启复用。其监听地址解析与 `a4n-config` 的固定 Server 配置不是同一生命周期。
- `a4n-server` 运行路径以 R-43 的配置 owner 解析地址，并把明确的 Control 地址注入 Control、Edge 和托管 Workers；Agent Worker 通过同一 owner 读取 Control 地址。独立 `a4n-control` 可执行入口则使用自己的 `127.0.0.1:18080` 缺省值，不从 Web 地址派生，并保留标准 SocketAddr 对端口 0 的接受行为；空环境值由 `optional_non_empty_env_string` 明确拒绝。
- 全仓文本检索确认这些环境变量入口只落在上述代码、测试、脚本、示例和公开文档中。Edge 与 `a4n-config` 的低层 `SocketAddr::parse` 调用虽相同，但错误上下文、监听生命周期和默认/端口语义不同；抽出公共解析 API 只会包装标准库语法，不会收敛业务规则，因此不合并、不增加依赖。
- 记录范围：`crates/host/a4n-edge/{src/lib.rs,src/main.rs}`、`crates/host/a4n-control/{src/lib.rs,src/main.rs,src/state.rs}`、`crates/config/a4n-config/src/lib.rs`、`crates/host/a4n-server/src/server/{startup/configuration.rs,process.rs}`、Agent Worker runtime、Tauri server process、CLI local runtime、cleanup recovery 测试、启动脚本、环境示例和公开文档。

### R-45：复核 Qdrant 环境变量读取重复

- 类别：核验后保留，不改代码。
- BFS 覆盖：从 `QdrantStoreConfig::from_env` 追到 `QdrantVectorStore::connect_from_env`，再到 Host `create_embedded_rag_store` 与 RAG Service `run` 两个生产调用；核对 `qdrant_live` 集成测试直接构造 `QdrantStoreConfig` 并调用 `connect`，不会绕过或覆盖生产环境读取路径。静态搜索枚举 workspace 中 `optional_non_empty_env_string` 的定义和调用。
- RAG adapter 的 optional/required helper 只在私有 `qdrant_store/config.rs` 内使用，并返回 `a4n_domain::ProviderError::Configuration`；`a4n-config` 的同名 helper 返回 `a4n_error::AppError::ConfigError`。Host 边界只定义了 ProviderError 到 AppError 的单向转换，错误 owner 和依赖层不同。
- 将 RAG adapter 改为依赖 `a4n-config` 会把 Host 配置 crate 引入低层存储 adapter；增加错误转译 wrapper 仍需维护一个反向适配层，且没有减少业务规则。因此保留小型本地解析器，不新增依赖、fallback 或错误分类变化。
- 记录范围：`crates/rag/a4n-rag-index/src/qdrant_store/{config.rs,store.rs,mod.rs}`、`crates/rag/a4n-rag-index/tests/qdrant_live.rs`、Runtime RAG factory、RAG Service app、`a4n-config` env helper、`a4n-domain::ProviderError` 与 Host 错误转换。

### R-46：删除 RAG tracing 转发函数

- 类别：无行为的单调用点 wrapper。
- BFS 覆盖：从 RAG Service `run` 追到私有 `init_tracing` 和 `a4n-sidecar-bootstrap::init_tracing`；CodeGraph 与模块内静态搜索都确认唯一 caller 是 `run`，没有外部导出或测试引用。对照 Embedding、Image、LLM Service 启动入口，它们都直接调用同一 bootstrap helper。
- 在 `run` 中直接调用 shared bootstrap helper，删除只转发服务日志 env 名称和 filter 的本地函数；保留相同参数、`?` 错误传播及启动顺序。
- 验证：`cargo check --all-targets --locked --offline -p a4n-rag-service`、`cargo fmt --all -- --check`、`git diff --check` 通过。
- 记录文件：`crates/rag/a4n-rag-service/src/app.rs`。

### R-47：复核 Storage Paths 与 Sidecar health 适配

- 类别：核验后保留，不改代码。
- BFS 覆盖：从 `get_storage_paths` 命令字符串追到 Control dispatch、`operations::get_storage_paths`，再追到 Runtime action 的管理员权限校验、环境读取和 `StoragePathsSummary` 构造；同时枚举所有 `known_service_health` 调用及各 `healthz` 路由注册，并核对 Sidecar service registry。
- Control 层函数负责校验命令参数并序列化 Runtime 结果；Runtime action 承担权限和数据构造。两者不是两套业务实现，删除 Control adapter 会把 API/序列化边界移入 dispatch。
- 9 个已注册 Sidecar 的 health handler 都调用 `known_service_health`。Demo sidecar 使用未注册的 `demo` 身份直接构造 health；该 helper 对未知身份会 panic，不能替代。Agent Worker health 需要 token 鉴权并报告 WorkerHealth，Control health 返回简单存活响应；它们的合同和职责不同。
- 记录范围：Control/Runtime Storage Paths dispatch 与 action、API command manifest、9 个 Sidecar health handler、`a4n-service-protocol` registry、Demo sidecar、Agent Worker 与 Control health endpoints。

### R-48：删除 Prompt tracing 转发函数

- 类别：无行为的单调用点 wrapper。
- BFS 覆盖：从 Prompt Service 的二进制入口追到 `run_from_args -> run -> init_tracing -> a4n-sidecar-bootstrap::init_tracing`；CodeGraph 确认私有 wrapper 唯一 caller 为服务 `run`，backup/restore 分支不经过该初始化路径。与 R-46 的 RAG Service 对照确认传入的是同一类服务日志配置，不涉及其他 runtime 状态。
- 在 `run` 中直接调用 bootstrap helper 并删除 wrapper，保留 `A4N_PROMPT_SERVICE_LOG`、原 default filter、启动顺序和 `?` 错误传播。
- 验证：`cargo check --all-targets --locked --offline -p a4n-prompt-service`、`cargo fmt --all -- --check`、`git diff --check` 通过。
- 记录文件：`crates/prompt/a4n-prompt-service/src/app.rs`。

### R-49：统一可选非空路径环境变量解析

- 类别：重复配置解析。
- BFS 覆盖：从 `a4n_config::optional_non_empty_env_path` 追踪已有 9 个 Host/Runtime 调用点及其 unset/empty/configured 定向测试；对比 Prompt Service 的 prompts/git/data path 入口，以及 CLI 配置路径、ONNX Runtime、Sidecar binary、source root、target directory 五条流程。静态搜索确认 Prompt 的 3 个调用点和 CLI 的 10 个调用点都使用相同的 `var_os`、lossy-trim 空值判定与 `PathBuf` 构造。
- 删除 Prompt Service 和 CLI 的局部解析器，13 个调用点改用 `a4n-config` owner；两个 crate 增加直接依赖并更新 Cargo.lock。Unset、空白、非 UTF-8 `OsString` 和有效路径的判定保持一致；错误通过 `AppError::ConfigError` 进入 anyhow 边界后会带 `配置错误：` 前缀，缺失的必填路径仍由各业务入口返回原有专属错误。
- 验证：`cargo check --all-targets --offline -p a4n-config -p a4n-cli -p a4n-prompt-service` 通过；`timeout 60s cargo test --locked --offline -p a4n-config --lib optional_path_distinguishes_unset_empty_and_configured_values` 1/1 通过；`cargo fmt --all -- --check`、`git diff --check` 通过。
- 记录文件：`crates/config/a4n-config/src/lib.rs`、`crates/host/a4n-cli/{Cargo.toml,src/config.rs,src/local/mod.rs,src/local/runtime.rs}`、`crates/prompt/a4n-prompt-service/{Cargo.toml,src/app.rs}`、`Cargo.lock`。

### R-50：统一 CLI 可选字符串环境变量解析

- 类别：重复配置解析。
- BFS 覆盖：从 CLI 私有 `non_empty_env_string` 追踪 5 个实际调用点：Server URL、Access Token、local doctor/start 的 Qdrant URL，以及 `required_env`；继续从 `required_env` 追到 local start/migrate。与 `a4n-config::optional_non_empty_env_string` 的 var/trim/missing/empty/invalid-Unicode 分支逐项对比，并核对 `a4n-config` 已在 CLI 的直接依赖中。
- 删除 CLI 私有 parser，5 个调用路径改用 `a4n-config` owner。缺失仍为 `None`，空值仍显式失败；`required_env` 对缺失值的专属错误未变。非法空值/Unicode 错误现在使用 `AppError::ConfigError` 文案并由 anyhow 边界返回，不再保留 CLI 的局部措辞。
- 验证：`cargo check --all-targets --locked --offline -p a4n-cli`、`cargo fmt --all -- --check`、`git diff --check` 通过。
- 记录文件：`crates/host/a4n-cli/src/{config.rs,local/mod.rs,local/runtime.rs,transport.rs}`。

### R-51：复核 RAG Service 必填数据目录解析

- 类别：核验后保留，不改代码。
- BFS 覆盖：从 RAG Service `run` 追到唯一的 `data_dir` 调用，再沿结果追踪目录创建、sidecar data manifest 校验、未完成 mutation marker 检查和后续 `RagServiceState` 所有权；对照 `a4n-config::optional_non_empty_env_path` 的共享 parser 与错误边界。
- `data_dir` 不只是可选路径读取：它实现 RAG sidecar 必填 `A4N_RAG_SERVICE_DATA_DIR` 的领域错误合同，并将路径交给 RAG 专属 manifest/marker 生命周期。该模块返回 `a4n_domain::ProviderError::Configuration`，通用 helper 返回 `a4n_error::AppError::ConfigError`；仓库只定义 ProviderError 向 Host AppError 的单向转换。加入 Host 配置依赖并反向转译错误会新增 adapter，超过这段局部校验的重复收益，因此保留。
- 记录范围：`crates/rag/a4n-rag-service/src/app.rs`、RAG Service manifest/mutation marker、`a4n-config::optional_non_empty_env_path`、ProviderError 到 Host AppError 转换。

### R-52：复核 Sidecar 默认字符串环境解析

- 类别：核验后保留，不改代码。
- BFS 覆盖：从 Sidecar `string_or_default` 追到 tracing filter 与 service socket bind；再从 `optional_non_empty_string` 追到数值配置、服务鉴权、metrics 和缺省值解析。对照 Host `a4n-config::optional_non_empty_env_string` 与 Runtime baseline `env_string_or` 的调用路径及 Cargo 依赖边界。
- 两边对环境变量的 trim、缺失和显式空值检查相同，但 Sidecar bootstrap 的默认值也经 `normalize_non_empty_string` 验证非空；Runtime `env_string_or` 只 trim fallback。前者还承载 anyhow 错误、认证和数值读取，后者承载 Host `AppError::ConfigError`。
- `a4n-sidecar-bootstrap` 是多个 Sidecar 共用的进程层，当前不依赖 Host `a4n-config`。把配置层引入进程 bootstrap 会扩大所有 Sidecar 的依赖并混合错误 owner；抽到新底层 crate 也只为几行标准 env 解析增加组件。因此保留各自当前 owner，不添加回退或转换层。
- 记录范围：`crates/process/a4n-sidecar-bootstrap/{src/environment.rs,src/lib.rs,src/metrics.rs,Cargo.toml}`、`crates/config/a4n-config/src/lib.rs`、Runtime baseline config callers。

### R-53：复用数据目录文件路径校验

- 类别：重复安全路径校验。
- BFS 覆盖：从 `a4n-storage::canonicalize_path_in_data_dir` 追踪其 11 个调用点及 typed `CanonicalDataPathError`，并对照 `canonicalize_data_file_path`、`data_dir_relative_file_path` 的共享读写路径；再从 Orchestrator `create_media_asset` 的 file/thumbnail 两个输入追到 `resolve_data_file`，以及 Galgame asset、Canvas audio、media asset 的调用入口和已有路径边界测试。
- `resolve_data_file` 原先重复 canonicalize data root/target 并手工检查 `starts_with`。现在复用 `canonicalize_path_in_data_dir`；保留 `required_text`、绝对路径前置拒绝、I/O 错误到 `AppError::Io` 的映射、原越界文案和最终 `is_file` 检查。数据库写入、文件 checksum 和 provenance 流程不变。
- 验证：`cargo check --all-targets --locked --offline -p a4n-orchestrator` 通过；`cargo test --no-run --locked --offline -p a4n-orchestrator --lib` 编译通过；`timeout 60s cargo test --locked --offline -p a4n-orchestrator --lib media_thumbnail_path_must_be_relative_to_data_dir` 1/1 通过；`cargo fmt --all -- --check`、`git diff --check` 通过。
- 记录文件：`crates/host/a4n-orchestrator/src/media.rs`，共享实现位于 `crates/platform/a4n-storage/src/files/paths.rs`。

### R-54：复用 WebGAL 媒体文件路径校验

- 类别：重复安全路径校验。
- BFS 覆盖：从 WebGAL 导出入口 `run_export_galgame_webgal_validated -> build_webgal_package -> read_webgal_data_file` 追到所有导出资产 zip entry；并对照 `a4n-storage::canonicalize_data_file_path` 的空路径、绝对路径、canonicalize、数据根包含和文件类型规则。搜索确认 WebGAL helper 只有该打包调用点。
- WebGAL helper 改用 `canonicalize_data_file_path` 完成相对路径规范化、越界拒绝和 regular file 检查，之后仍由本地代码读取 bytes 并生成 zip entry。保留已有绝对路径 WebGAL 专属错误；空路径、越界、目录目标和 I/O 诊断转为 storage helper 的错误文字，并统一包装为现有 `InvalidState(current=webgal_export)`。
- 新增测试覆盖有效文件读取、绝对路径拒绝以及 `../` 越出数据根拒绝。验证：`cargo check --all-targets --locked --offline -p a4n-orchestrator` 通过；`cargo test --no-run --locked --offline -p a4n-orchestrator --lib` 编译通过；WebGAL 定向测试 1/1 通过；`cargo fmt --all -- --check`、`git diff --check` 通过。
- 记录文件：`crates/host/a4n-orchestrator/src/galgame/webgal_export_application.rs`，共享实现位于 `crates/platform/a4n-storage/src/files/paths.rs`。

### R-55：复核 Sidecar 多根输出路径解析

- 类别：核验后保留，不改代码。
- BFS 覆盖：从 `AppState::{data_media_file_ref,temp_media_file_ref}` 追到 `media_file_ref` 的 14 个实际输入/输出调用，覆盖 Canvas frame、video generation、assembly、thumbnail、trim、scrub-frame 和 audio probe；比较现有 `canonicalize_path_in_data_dir` 与 `deletable_data_file_path` 的输入合同、目标是否必须存在、文件类型约束及 symlink 处理。
- Sidecar helper 根据 `MediaRoot` 在 Data 与 Temp 根之间选择，并接收 `Path`（含绝对路径），对已存在目标 canonicalize、对尚不存在的输出文件 canonicalize 父目录后返回目标引用；`canonicalize_path_in_data_dir` 只接受已存在目标，`deletable_data_file_path` 仅服务数据根的相对字符串、要求已存在对象为普通文件，缺失时返回 `None`。它们的边界无法在不增加泛化 API 或改变接受规则的前提下合并。
- 记录范围：`crates/host/a4n-runtime/src/state/app/sidecar_media.rs`、所有 `data_media_file_ref`/`temp_media_file_ref` 调用点、`a4n-storage::canonicalize_path_in_data_dir` 与 `deletable_data_file_path`。

### R-56：统一导入图片相对路径转换

- 类别：重复文件路径校验与单调用 wrapper。
- BFS 覆盖：从 `persist_imported_image` 追到 staging/final 输出生成与 publication intent，确认两条路径都先创建项目图片父目录、目标文件尚未写入，再转换成 DB 相对路径；对照 `a4n-storage::data_dir_relative_path`、其消费者 `data_dir_relative_file_path` 及所有 `path_to_db_string` 调用。
- 删除 `data_path_to_db_string` 手写实现和该单调用 wrapper。staging 与 final 路径直接交给 `data_dir_relative_path`，由存储层统一 canonicalize 根/父目录、拒绝根外目录并规范平台分隔符；目标文件缺失仍被允许。越界诊断使用共享 helper 的 `{label}不在应用数据目录内` 文案。
- 在 `a4n-storage` 增加测试覆盖缺失目标的相对路径生成和根外父目录拒绝。验证：`cargo check --all-targets --locked --offline -p a4n-storage -p a4n-runtime` 通过；`timeout 60s cargo test --locked --offline -p a4n-storage --lib data_dir_relative_path_accepts_missing_target_and_rejects_outside_parent` 1/1 通过；`cargo fmt --all -- --check`、`git diff --check` 通过。
- 记录文件：`crates/platform/a4n-storage/src/files/paths.rs`、`crates/host/a4n-runtime/src/actions/image_records/upload/{file.rs,record.rs}`。

### R-57：复核生成图片预留路径验证

- 类别：核验后保留，不改代码。
- BFS 覆盖：从 `prepare_generated_image_file` 的 staging/final 相对路径生成追到 `persist_reserved_generated_image_file` 两个解析点、exclusive `create_new` 写入和 `promote_data_file`；继续追踪 Runtime Image/Portrait Host Port 与 Orchestrator 生成流程。检查 `PreparedGeneratedImageFile` 的 contract 定义确认结构公开且路径字段可由跨 crate 调用方构造。
- `resolve_reserved_path` 除 canonicalize 父目录及根目录包含外，还拒绝绝对、空和任何非 `Component::Normal` 路径段，确保 prepared reservation 不接受 `.`/`..` 别名。`a4n-storage::data_dir_relative_path` 面向已解析的输出 `Path`，允许可规范化的相对路径段，不能替代此跨 crate 持久化入口的更严格合同。
- 记录范围：`crates/host/a4n-runtime/src/actions/image_generation/generation_io/files.rs`、相关 Runtime Host Ports/Orchestrator caller、`crates/image/a4n-image-host-contract/src/lib.rs::PreparedGeneratedImageFile`、`a4n-storage::data_dir_relative_path`。

### R-58：复核外部音频/视频文件导入校验

- 类别：核验后保留，不改代码。
- BFS 覆盖：从 Canvas Audio `CanvasAudioImportSource::from_path` 追到 validated import、落盘/probe 和媒体资产创建；从 Video Record `import_video_source_path` 追到本地视频导入、上传 staging/copy 和最终资源落库。横向比较 trim/空路径、metadata、regular-file、canonicalize 的重复低层步骤及后续扩展名/文件名处理。
- 两者都读取用户提供的外部路径，不是应用数据目录内的持久化路径：视频入口在 canonicalize 前检查原始扩展名，并在 `import_video_source` 后续再次检查 canonical target 扩展名；音频入口还执行独立最大字节限制、canonical 文件名提取和音频 MIME/格式识别。空值和错误文案也属于各自导入交互合同。将这些流程合成 helper 会需要回调/配置多个验证阶段，或改变验证顺序和错误分类。
- 当前分别通过 `a4n-video::import_video_extension`、`decode_uploaded_audio_data`/`canvas_audio_format` 等领域 owner 验证格式；保留两处外部文件读取入口，不新增通用 path abstraction。已存在的数据目录内文件仍由 `a4n-storage` helper 唯一校验。
- 记录范围：`crates/host/a4n-orchestrator/src/canvas/audio_import.rs`、`crates/host/a4n-runtime/src/actions/video_records/imports/source/{paths.rs,upload.rs,copying.rs}`、`crates/video/a4n-video/src/import.rs`、`crates/media/a4n-media/src/upload.rs`。

### R-59：复核媒体 URI 与 Sidecar 路径合同

- 类别：核验后保留，不改代码。
- BFS 覆盖：从 Host `relative_media_path` 追到 Control `/media` 查询路径、Runtime token expected path 和 `relative_media_uri_path`；从 URI parser 追到 Tauri media protocol 的 canonical root 检查。另从 `MediaFileRef::new` 追到 `validate_relative_media_path` 和其 Sidecar 请求合同。
- Host helper 处理普通/URI 路径，URI 版本先 percent-decode；它只负责拒绝空值和绝对路径，调用方再执行 Data root 授权或 canonical root 检查。Wire `MediaFileRef` validator 直接拒绝 `..`、`.`、反斜杠和绝对路径，避免不规范路径进入 Sidecar 合同。移除其中任一层会把部署边界校验移到另一层或改变允许输入，不视为同一合同。
- 记录范围：`crates/media/a4n-media/src/media_file/path.rs`、Control media route、Runtime token path authorization、Tauri media protocol、`crates/media/a4n-media-contract/src/lib.rs::validate_relative_media_path` 与 `MediaFileRef`。

### R-60：复核 MediaAsset 文件大小事实读取

- 类别：核验后保留，不改代码。
- BFS 覆盖：从 `OrchestratorContext::create_media_asset` 追到三个生产 caller（Galgame visual asset、Canvas audio import、Galgame audio import）、`resolve_data_file`、`media_file_size`、checksum 和 DB record input；同时枚举 `a4n-storage::data_file_record_facts` 的 9 个外部生产调用与 storage crate 内调用。
- 两处都读取 metadata 长度，但 `data_file_record_facts` 还校验数据根、生成 DB 相对路径并返回 `String` 错误；Orchestrator 使用 `AppError::Io` 表示文件系统失败、用 `InvalidArgument` 表示超过 `i64` 大小，并原样持久化已由各 importer 生成的 `input.file_path`。直接替换会改 API 错误分类，若同时改存储路径则扩大行为变化；为一处轻量长度读取新增 typed adapter 也会增加结构。
- `media_file_checksum` 读取内容并服务 MediaAsset checksum 合同，与 storage 的路径/size facts 不同。保留 Orchestrator 本地 size/checksum 逻辑，不新增错误转译或路径规范化。
- 记录范围：`crates/host/a4n-orchestrator/src/media.rs` 与 `crates/platform/a4n-storage/src/files/persistence.rs::data_file_record_facts` 的全部生产调用者。

### R-61：复核 MediaRoots 根目录解析

- 类别：核验后保留，不改代码。
- BFS 覆盖：从 `MediaRoots::new` 追到 Data/Temp 两个根字段，再追踪 `input`/`output` 的 `MediaFileRef` 验证、canonical target/parent 检查和各 media processing worker 调用；对照 `a4n-storage::canonicalize_path_in_data_dir` 的根/目标责任。
- `canonical_root` 每次只由 `MediaRoots::new` 调用，用于两个配置根各自的初始化；它在 canonicalize 后验证根本身是目录。`canonicalize_path_in_data_dir` 用于验证单个存在目标是否位于 data root 内，不负责根目录必须为目录，也不接受 temp root 语义。把 storage 平台依赖引入 media render core，或为 7 行标准库检查新建跨层公共 API，均会扩大依赖而不消除同一业务实现。
- 记录范围：`crates/media/a4n-media-render-core/src/paths.rs`、`MediaRoots` 的 worker callers、`crates/platform/a4n-storage/src/files/paths.rs::canonicalize_path_in_data_dir`。

### R-62：复核数值环境配置解析

- 类别：核验后保留，不改代码。
- BFS 覆盖：从 Sidecar `optional_u32`/`u64_or_default` 追到 metrics restart count 与日志保留配置；从 Runtime `env_i32_or` 追到 public image/LLM/video baseline 配置。逐项比较缺失默认、整数符号范围、解析错误类型、调用者依赖和下游值语义。
- Sidecar 两处解析分别要求非负 `u32`、`u64` 并返回 `anyhow`；Runtime parser 支持完整 `i32`，缺失时返回调用方 fallback 并使用 `AppError::ConfigError`。即使底层都调用 `parse`，范围和配置 owner 不同，抽公共 parser 会引入类型参数/trait 或重分类错误，保留。
- 记录范围：`crates/process/a4n-sidecar-bootstrap/src/environment.rs`、Sidecar metrics/logging callers、`crates/host/a4n-runtime/src/baseline_configs/env.rs` 与 image/LLM/video callers。

### R-63：复核 Sidecar 日志目录环境解析

- 类别：核验后保留，不改代码。
- BFS 覆盖：从 Sidecar 私有 `optional_non_empty_path` 追到唯一生产 caller `a4n-sidecar-bootstrap::logging::init`，再追到服务日志目录创建、轮转文件和周期清理；对照 `a4n-config::optional_non_empty_env_path` 的 9 个 Host/Runtime 调用及直接依赖关系。
- `var_os`、未设置/空白和 `PathBuf` 语义相同，但 Sidecar parser 是 process bootstrap 本地 anyhow 环境模块；依赖 `a4n-config` 会把 Host 配置错误 owner 及其依赖引入所有 Sidecar bootstrap 消费者。该局部读取只有一处调用，提取新低层 crate 成本更高，故保留。
- 记录范围：`crates/process/a4n-sidecar-bootstrap/{src/environment.rs,src/logging.rs,Cargo.toml}`、`crates/config/a4n-config/src/lib.rs` 与其 Host/Runtime callers。

### R-64：统一图片发布 staging durable write

- 类别：重复 durable file write 与失败清理。
- BFS 覆盖：从 imported-image `persist_imported_image` 追踪 publication intent、staging facts、staging write、final promote、发布和 cleanup outbox；从 generated/portrait `persist_reserved_generated_image_file` 追踪 `PreparedGeneratedImageFile` 校验、exclusive staging create、写入/flush/fsync、promote 和 file facts；静态枚举 `create_new(true)` 后续写入并 sync 的 workspace 调用，确认另有 RAG 空 mutation marker（只创建空标记并 sync），不属于内容文件写入。
- 将重复的 exclusive create、write_all、flush、sync_all 和数据根路径处理收敛到 `a4n-storage::write_new_synced_data_file`，基于既有 `write_new_data_file` 路径生命周期/失败清理实现；普通非 durable `write_new_data_file` 调用保持原行为。两个图片发布流程直接调用 shared helper，各自的 intent、facts/hash 校验、严格 prepared-path validation、promotion 和 publish 顺序保留。
- 失败路径变化：生成图片此前写入/flush/fsync 失败会留下部分 staging file；现在和导入图片一样，写入失败会尝试删除，并在清理失败时返回附带次错误的主错误。新增 storage 测试覆盖根目录范围和不覆盖；生成图片既有 durable persistence 测试确认输出和 size 合同。
- 验证：`cargo check --all-targets --locked --offline -p a4n-storage -p a4n-runtime` 通过；storage durable-writer 定向测试 1/1、Runtime `generated_image_file_persistence_keeps_output_and_reports_actual_size` 1/1 通过；`cargo fmt --all -- --check`、`git diff --check` 通过。
- 记录文件：`crates/platform/a4n-storage/{src/files/path_lifecycle.rs,src/files/mod.rs,src/lib.rs}`、`crates/host/a4n-runtime/src/actions/{image_records/upload/record.rs,image_generation/generation_io/files.rs}`。

### R-65：复核 HTTP media byte-range 实现

- 类别：核验后保留，不改代码。
- BFS 覆盖：从 `/media` 和 `/temp-media` Axum route 追到 `a4n-control::file_response -> tower_http::ServeFile`；从 Tauri `handle_request` 追到 `file_response -> a4n-media::read_media_file_response -> requested_byte_range_header -> requested_byte_range`。同时检查两侧 Range 测试与所有 workspace `Range` header parser 搜索结果。
- Tauri custom URI 需要本 crate 构造 Tauri response body，因此使用共享 `a4n-media` parser/response builder；Control 使用 Tower `ServeFile` 获取异步文件流。Control regression test 覆盖标准/后缀/suffix ranges、无效 range 与 32 MiB body 首块不超过 64 KiB。把 Control 切换到当前返回 `Vec<u8>` 的 shared response 会一次性缓冲文件，改变已测试的大文件内存合同；另写一套 custom streaming response 则会复制传输实现。故保留标准框架服务路径和 Tauri shared parser 两个适配，不合并。
- `a4n-media::requested_byte_range` 的多段 Range 拒绝和 header-to-invalid mapping 由其单测覆盖；Control 的实际 Range 语义由 ServeFile 集成测试覆盖。
- 记录范围：`crates/media/a4n-media/src/media_file/{range.rs,response.rs}`、Control media/temp-media routes 与 tests、Tauri media protocol。

### R-66：统一本地视频文件与上传格式合同

- 类别：重复的视频扩展名与 MIME 上传映射。
- BFS 覆盖：从 `a4n-domain::media::{canonical_video_extension,video_mime_from_path,video_upload_mime_matches}` 追到 Video Record 导入、预览与播放；从 `a4n-video` 的导入/上传校验追到 Canvas、章节、参考媒体和后处理上传入口；从 Runtime `required_video_file_extension` 追到 provider 完成保存及 Canvas provider file persistence；从前端 `pickVideoUploadFile` 追到 8 个用户调用点。另追踪 `a4n-api-contract::generate_types` 的配置生成阶段，确认已有 Rust→TypeScript 生成路径可复用。
- 将 `mp4/m4v/webm/mov` 扩展、规范 MIME 和各扩展上传可接受的 MIME 别名集中到 `a4n-domain::media::VIDEO_MEDIA_TYPES`。Runtime 扩展校验、视频导入和后端上传 MIME 校验均从该表读取；API 合同生成器从同表生成前端 `VIDEO_UPLOAD_MIME_TYPES`，前端文件选择器不再维护副本。
- 保持既有合同：文件扩展名按原有路径规范大小写/空白；上传 MIME 仍忽略 ASCII 大小写并去首尾空白；后端上传仍要求小写扩展名。生成的前端类型要求每个扩展至少有一个 MIME，移除了不可能为空的首候选 `null` 回退。文件名扩展查表改为自有键检查，避免原型链键被当作支持格式。
- 验证：`a4n-domain` MIME/扩展定向测试 1/1、`a4n-video` 上传合同定向测试 1/1、Runtime provider 扩展定向测试 1/1；4 个受影响 Rust package 的 all-targets 检查通过；`generate_types --check`、`npm run check:web-api-parity`、Video Assembly 组件 Vitest 2/2、`videoUpload.test.ts` 2/2、前端生产构建、格式与 diff 检查通过。
- 记录范围：`crates/platform/a4n-domain/src/media/{files.rs,media.rs}`、`crates/video/a4n-video/src/import.rs`、Runtime video file persistence、`crates/host/a4n-api-contract/src/bin/generate_types{.rs,/stages.rs}`、生成的 `frontend/src/api/generated/config.ts`、`frontend/src/utils/videoUpload{.ts,.test.ts}`。

### R-67：统一 provider 视频扩展名白名单

- 类别：重复的 provider 视频格式 allowlist。
- BFS 覆盖：从 `normalize_video_format_str` 追到 chat response classification、format hint、download request 与 NewAPI result decode；从 `video_format_from_url` 追到 chat URL classification 与 download；从 HTTP `required_video_format` 追到 `decode_result` 的同步返回和 poll 完成路径。逐一核对 3 处扩展名集合和现有 provider 单测。
- provider 的规范化、URL 扩展名解析和 HTTP 响应 format 校验改用 R-66 的 `canonical_video_extension`，移除三份 `mp4/webm/mov/m4v` allowlist。保留各输入原合同：format 字段先去空白并小写；通用 provider format 继续去首尾空白及前导点；URL extension 保持不接受尾随空白。
- 保留 provider `Content-Type` 到格式的映射与字节魔数嗅探：`video/x-m4v` 是响应头映射别名；`ftyp` 同时覆盖 MOV/MP4 容器而返回 `mp4`；二者的语义不同于文件扩展名 allowlist。只有响应头缺失或无法映射时才尝试 magic sniff，仍由既有输入路径使用。
- 验证：`cargo check --all-targets --locked --offline -p a4n-video-providers` 通过；`timeout 60s cargo test --locked --offline -p a4n-video-providers --lib` 85/85 通过；`cargo fmt --all -- --check` 与 `git diff --check` 通过。
- 记录范围：`crates/video/a4n-video-providers/src/{format.rs,http/response.rs,http/tests.rs}`，及 R-66 的 `a4n-domain::media::VIDEO_MEDIA_TYPES` 定义和消费者图。

### R-68：统一临时媒体视频 MIME 子集

- 类别：重复的临时媒体 MIME→扩展名映射及前端文件类型列表。
- BFS 覆盖：从参考媒体组件的 `selectLocalFile` 追到 `ReferenceMediaUploadInput`、Orchestrator `prepare_reference_media_video_generation`、Runtime `upload_reference_media_input_for_preview -> preview::upload_reference_media_input -> media_bytes_to_remote_temp_media_url`，再到 Control `POST /temp-media`；同时追到 metadata/file 两次 MIME 校验、文件名后缀校验、artifact filename serving 的 `find_temp_media_path` 和过期清理。从生成图片 `image_file_to_remote_temp_media_url` 复核同一临时媒体入口的其他生产 caller。
- 将临时媒体限定的视频扩展名 `mp4/mov/webm` 收敛到 `a4n-api-contract::temp_media::TEMP_MEDIA_VIDEO_EXTENSIONS`。合同生成器据此与 R-66 domain MIME 表生成 `TEMP_MEDIA_VIDEO_MIME_TYPES`；Control MIME→扩展名和前端参考媒体文件选择都消费生成规则。Control 的文件名校验与 artifact lookup 共用同一扩展名迭代器，保留原图片、视频、音频搜索顺序。
- Provider `video_format_from_content_type` 与 Control Temp Media 共用 domain canonical MIME 反查；provider 独有 `video/x-m4v -> m4v` 响应头别名仍保留。TEMP_MEDIA 继续仅允许 mp4/mov/webm 三种视频扩展，拒绝 m4v 和 `application/mp4` / `video/mov` 上传别名。Magic-byte 校验保留为独立内容完整性合同。
- 验证：4 个受影响 Rust package all-targets 检查通过；domain MIME 测试 1/1、Control Temp Media 子集测试 1/1、video-provider 85/85 通过；API 类型生成检查和 `npm run check:web-api-parity` 通过；`referenceMediaWorkspace.test.ts` 2/2、前端构建、格式和 diff 检查通过。
- 记录范围：`crates/platform/a4n-domain/src/media/{files.rs,media.rs}`、`crates/host/a4n-api-contract/src/{temp_media.rs,bin/generate_types.rs,bin/generate_types/stages.rs}`、Control temp-media rules/storage、`crates/video/a4n-video-providers/src/format.rs`、生成配置与 `referenceMediaWorkspace.tsx`。

### R-69：统一 Temp Media 全部 MIME 与扩展名合同

- 类别：同一临时媒体上传能力在 API、Control 与前端各自维护 MIME/扩展名列表。
- BFS 覆盖：从参考媒体组件本地文件选择追到 `ReferenceMediaUploadInput`；从 Orchestrator 生成预览/任务准备追到 Runtime `upload_reference_media_input_for_preview`、临时文件上传请求和 Control `POST /temp-media`。继续追踪 metadata/file MIME 校验、文件名扩展约束、artifact ID serving 查找及过期清理；另外核对生成图片走同一临时媒体服务的路径。
- 在 `a4n-api-contract::temp_media::TEMP_MEDIA_FORMATS` 保存 Temp Media 支持的 image/video/audio canonical MIME、扩展名、服务端 MIME aliases 和浏览器 MIME 列表。Control normalization、artifact filename validation/search 统一读取此表；生成器从表生成 `TEMP_MEDIA_BROWSER_MIME_TYPES`，前端 picker 的 `accept` 和实际 `File.type` 检查都使用同一份数据。删除 Control 与 UI 的局部 MIME→扩展名/允许列表副本。
- 保持原始输入边界：服务端接受 `audio/mp3` 并规范化为 `audio/mpeg`；浏览器列表仍不包含 `audio/mp3`，因为本地选择器之前使用的是更窄的 File.type 子集。Temp Media 继续拒绝 `application/ogg`、`video/x-m4v` 和 `video/mov`；magic-byte 验证继续独立检查真实文件内容。
- 验证：4 个受影响 Rust package all-targets 检查通过；domain、Control Temp Media、API contract 定向测试各 1/1，video-provider 85/85；`generate_types --check`、`npm run check:web-api-parity`、`referenceMediaWorkspace.test.ts` 2/2、前端构建、`cargo fmt --all -- --check` 和 `git diff --check` 通过。
- 记录范围：`crates/host/a4n-api-contract/src/temp_media.rs` 与 generator/stages、`crates/host/a4n-control/src/routes/temp_media/{rules.rs,storage.rs}`、`frontend/src/pages/novel-detail/video/referenceMediaWorkspace.tsx` 和生成的 `config.ts`。

### R-70：统一图像上传格式事实来源

- 类别：PNG/JPEG/WebP 的规范 MIME、扩展名和浏览器接收列表在 domain、解码器、Tauri 选图、API/Temp Media 合同、provider 适配和前端各自重复维护。
- BFS 覆盖：从 `a4n-media::decode_uploaded_image_data` 追到 Portrait、图片记录、Profile Material 三个生产入口及魔数识别、格式解码、尺寸检查和持久化准备；从 `a4n-domain::image_mime_from_path` 追到 Runtime 图片生成/视频引用与 Video Provider 的四个调用点；从 `imageUpload.ts` 的 MIME 校验和 `accept` 追到 12 个消费者，并补查 Chapter Visual Plan 的内联文件类型列表。另从 `TEMP_MEDIA_FORMATS` 追到 Control MIME/扩展名校验、临时文件查找和生成的浏览器类型；从 Tauri `read_local_image_file` 经 command registry 追到前端 invoke；从 ThemeTab/`uploadThemeResource` 追到 Control command dispatch、Runtime 上传校验和主题包导入/导出；从 Image Provider `normalize_image_format` 追到声明格式、URL 扩展名与响应 MIME 的四个调用点。静态搜索复核 Rust/TypeScript、Tauri 注册、主题路由和生成配置中的 MIME/扩展名字面量。
- 在 `a4n-domain::media::IMAGE_UPLOAD_MEDIA_TYPES` 集中 canonical MIME、保存扩展名、接收的 `.jpeg` 别名、服务端 MIME 与浏览器 MIME。`a4n-media` 的输入 MIME 归一化、图像字节格式识别及 `DecodedUploadedImage` 扩展名复用该目录；Runtime 参考图与 Video Provider 的 ImageFormat→MIME 适配也共用它。Temp Media 的三个图像记录由该目录构造；API 生成器输出 `IMAGE_UPLOAD_MIME_TYPES`，Temp Media 浏览器图像集合、前端选择/检查、主题 PNG/JPEG/WebP 列表和 Chapter Visual Plan `accept` 复用生成结果。Tauri 本地选图与 Image Provider 的 canonical 扩展名映射改用 domain helper；移除 Control 中单独的 `.jpeg` alias 判断。
- 保留合同差异：主题资源仍额外接受 GIF；Image Provider 的响应头仍额外接受 `image/jpg`；通用媒体路径的 APNG/AVIF/GIF/SVG MIME 不属于上传编解码 allowlist；New API 视频首帧仅 PNG/JPEG、EPUB 封面仅 PNG/JPEG，均是目的地专属限制。Temp Media 魔数签名仍单独核对真实内容，图片解码、MIME 与内容一致性、尺寸/字节限制及压缩逻辑不合并。
- 验证：8 个受影响 Rust package all-targets 检查通过；domain/media/API contract/Control 测试分别 1/1、1/1、4/4、1/1；Runtime 参考图 5/5、主题 metadata 1/1；Video Provider lib 44/44、Image Provider lib 测试通过。`generate_types --check`、`npm run check:web-api-parity`、新增 `imageUpload.test.ts` 1/1、前端生产构建、`cargo fmt --all -- --check` 和 `git diff --check` 通过。首次 Control 冷编译超过单测 60 秒上限，预编译后重跑测试通过。
- 记录范围：`a4n-domain::media`、`a4n-media`、API contract Temp Media/generator/生成的 `config.ts`、Control Temp Media rules、Runtime image reference/theme resource、Tauri image import、Image/Video Providers，以及前端 `imageUpload.ts`、Theme Editor、Chapter Visual Plan 与定向单测。

### R-71：统一 canonical 音频格式来源

- 类别：音频 canonical MIME/扩展名分别出现在 domain MIME 归一化、通用媒体扩展名表、Temp Media API 记录和 Canvas 路径再反查中。
- BFS 覆盖：从 `canonical_audio_mime` 沿 `a4n-media::decode_uploaded_audio_data` 到 Canvas 与 Galgame 音频导入；沿 Canvas `canvas_audio_format` 追到本地路径导入和 `persist_canvas_audio`；从 `media_mime_from_path` 追到 `a4n-media::read_media_file_response`；从 `TEMP_MEDIA_FORMATS` 追到 Control 上传元数据/file MIME 校验、文件名扩展名检查、artifact 查找和生成的浏览器类型。静态引用搜索复核所有音频 MIME/扩展名常量及 `application/ogg` 特例。
- 在 `a4n-domain::media::AUDIO_MEDIA_TYPES` 集中 MP3/WAV/OGG/M4A/AAC/FLAC 的 canonical MIME、扩展名和服务端 MIME aliases。`canonical_audio_mime`、`media_mime_from_extension` 与音频路径解析统一读取该表；Canvas 改为直接接收解析后的 `AudioMediaType`，删除原来“路径→通用 MIME→canonical MIME→扩展名”的第二次查表。Temp Media 记录从对应 audio format 构造，不再复制 MP3/WAV/M4A canonical MIME、扩展名和服务端 alias；browser MIME 列表继续由 Temp Media API contract 持有。
- 保留合同差异：Temp Media 仅列出 MP3/WAV/M4A/OGG；其 OGG accepted MIME 仍采用 browser 子集 `audio/ogg`，拒绝 domain 通用上传规则接受的 `application/ogg`。MP3 服务端仍接受 `audio/mp3`，浏览器列表仍仅为 `audio/mpeg`。AAC/FLAC 保留在通用 domain audio mapping，但不加入 Temp Media。Control signature/magic 检查继续作为独立的真实内容校验。
- 验证：5 个受影响 Rust package all-targets 检查通过；domain canonical/path 测试 1/1、media audio upload 1/1、API contract Temp Media 4/4、Control MIME/扩展名测试 1/1、Orchestrator Canvas audio path 1/1。`generate_types --check`、`npm run check:web-api-parity`、`cargo fmt --all -- --check` 和 `git diff --check` 通过。
- 记录范围：`crates/platform/a4n-domain/src/media/{files.rs,media.rs}`、`crates/media/a4n-media/src/upload.rs`、`crates/host/a4n-api-contract/src/temp_media.rs`、Control Temp Media rules 与 `crates/host/a4n-orchestrator/src/canvas/audio_rules.rs`。

### R-72：统一前端字节大小格式化

- 类别：Embedding 下载面板与 Storage 清理面板分别实现 locale-aware 字节大小格式化。
- BFS 覆盖：从 AboutTab 动态渲染到 `StoragePathsPanel`，追踪 orphan summary、删除确认、清理结果及单项目录大小的 formatter 调用；从 EmbeddingTab/`EmbeddingConfigEditor` 到 `FastembedModelPanel`，追踪总大小、缓存/缺失大小、下载按钮和进度提示的 formatter 调用。CodeGraph 与静态引用搜索确认生产代码中原有两份字节大小实现，二者都委托 `i18n::formatNumber`。
- 将 formatter 收敛到 `frontend/src/i18n/format.ts::formatBytes`，并从 i18n barrel 导出。统一使用 1024 进位、B 到 TB、locale 数字分组和最多一位小数；超出 TB 时继续显示 TB。Embedding 不再截在 GB；Storage 面板大于等于 10 的 unit 值不再显示超过一位小数。非有限或非正值继续显示 locale 格式的 `0 B`。
- 验证：`i18n.test.ts` 与 `EmbeddingConfigEditor.test.tsx` 共 2 个文件、11/11 通过；新增用例覆盖 0、Infinity、KB 精度和 TB 上限；`npm run build` 通过，`git diff --check` 通过。首次 build 发现并清理局部 formatter 移除后遗留的未使用 import，之后重跑通过。
- 记录范围：`frontend/src/i18n/{format.ts,index.ts,i18n.test.ts}`、Embedding formatter consumers/utility 与 `StoragePathsPanel.tsx`。

### R-73：复用 Image Metadata 字节格式化

- 类别：图片库 `fileSizeLabel` 保留第三份 MIME 无关的字节单位换算。
- BFS 覆盖：从 `fileSizeLabel` 追到 `imageBaseMetadataFields` 的调用点和图片 metadata 展示；静态搜索 `formatBytes`/字节单位转换确认该路径与 R-72 的 Storage/Embedding formatter 是三个生产实现。
- Image Metadata 改用 `i18n::formatBytes`，显式设置 `maxUnit: MB` 和一位固定小数，保留图片条目的 MB 上限及 KB/MB 展示精度；null、负数和非有限值仍显示“未知”。共享 formatter 仅增加 `maxUnit` 与 `minimumFractionDigits` 选项，不保留图片库本地换算。
- 验证：`i18n.test.ts` 与 `EmbeddingConfigEditor.test.tsx` 2 个文件、11/11 通过；新增断言覆盖 MB 上限及固定小数；`npm run build` 和 `git diff --check` 通过。
- 记录范围：`frontend/src/i18n/format.ts`、`frontend/src/i18n/i18n.test.ts` 与 `frontend/src/pages/novel-detail/images/imageMetadata.ts`。

### R-74：统一图片上传大小合同

- 类别：通用图像上传、Profile Material 图片上传和 Theme 资源上传的大小限制分别在前端、Orchestrator、Runtime、Tauri 和错误文案中重复定义。
- BFS 覆盖：从通用图片上传的五个前端消费者追到 `uploadImage` API、Orchestrator `prepare_imported_image_upload_for_user_validated` 和共享图像解码器；从 `readLocalImageFile` 经既有 Tauri command 注册路径追到 `read_local_image_file` 的文件读取边界；从 `VideoMaterialWorkspace.uploadMaterialFile` 追到 `uploadProfileMaterialImage`、`ProfileMaterialUploadApplicationPort` 实现和图像解码；从 Theme Editor 文件选择追到 controller/API、Runtime 资源持久化与主题包导入。CodeGraph 检查三类常量及调用点，全仓静态搜索复核 20/50 MiB 字面量、大小常量、生成配置、翻译和测试 fixture。
- 在 `a4n-api-contract::image`、`profile`、`theme` 分别定义 `MAX_UPLOAD_IMAGE_BYTES`（50 MiB）、`MAX_PROFILE_MATERIAL_IMAGE_UPLOAD_BYTES`（20 MiB）和 `MAX_THEME_RESOURCE_BYTES`（20 MiB）；`generate_types` 将三者写入生成的 `config.ts`。Rust 调用端读取对应合同常量，前端消费者直接读取生成常量；`imageUpload.ts` 只保留 MIME helpers，不再转发大小常量。Tauri 文件读取仍最多读取上限加 1 字节并明确拒绝超限文件，错误文案从合同常量计算；Theme 中英文错误文案改为插值上限。
- 保留独立边界：视频 sidecar 图片引用 20 MiB、参考媒体本地选择 20 MiB、视觉分析单图 8 MiB/总量 16 MiB、图片 provider 压缩后参考图 700 KiB 和 Temp Media 配置上限各自服务不同合同或处理阶段，不按数值相同合并。Theme 迁移中的历史 8 MiB→20 MiB 说明保持为历史事实。
- 验证：API contract、Orchestrator、Runtime、Tauri、host-assembly 的 `cargo check --all-targets --locked --offline` 通过；生成器 `--write`/`--check` 和 `npm run check:web-api-parity` 通过；`npm run build` 通过（保留仓库已有大 chunk 提示）；`cargo fmt --all -- --check`、`git diff --check` 通过。未运行测试套件；all-targets 检查编译了测试目标。
- 记录范围：API contract 的 image/profile/theme 常量与生成器、生成的 `frontend/src/api/generated/config.ts`、Orchestrator/Runtime/Tauri/host-assembly 上传消费者、前端五个通用图像上传消费者、Profile Material/Theme Editor 消费者及 Theme 中英文消息。

### R-75：复用 Worker shutdown signal waiter

- 类别：Agent、Canvas Record、Artifact Media、Novel Planning Worker 分别实现了相同的 Ctrl-C/SIGTERM 等待分支。
- BFS 覆盖：从四个 Worker 的启动入口追到 Agent `async_main` 的 shutdown/driver/health select，以及其余三者的 `axum::serve().with_graceful_shutdown(...)`；继续检查 shutdown 后的 runtime cancellation、maintenance task 终止和 server 返回路径。CodeGraph 确认四个局部 waiter 各只有一个生产调用；全仓搜索 `SignalKind::terminate()` 找到四个 Worker 副本及 Control、Edge、Tauri 的宿主级处理。
- 新增 `a4n-worker-runtime::wait_for_shutdown_signal()`，集中等待 Ctrl-C 或 Unix SIGTERM，非 Unix 继续等待 Ctrl-C。四个 Worker 直接调用该 helper；既有 `wait_for_ctrl_c()` 保持 Ctrl-C-only 行为。Agent 仍取消 runtime 并等待 driver/health server，Artifact Media 仍在 serve 返回后 abort maintenance，Canvas/Novel Planning 仍由 Axum 完成 graceful shutdown。
- 保留边界：Control 在 serve 后还关闭 managed sidecars；Edge 还响应父进程 stop flag；Tauri 收到信号后触发应用退出。这三者的生命周期和清理合同不同，没有改为依赖 Worker runtime。
- 验证：`cargo check --all-targets --locked --offline -p a4n-worker-runtime -p a4n-agent-worker -p a4n-canvas-record-worker -p a4n-artifact-media-worker -p a4n-novel-planning-worker`、`cargo fmt --all -- --check` 和 `git diff --check` 通过。未运行测试套件；all-targets 检查编译了测试目标。
- 记录范围：`a4n-worker-runtime` signal helper 与四个 Worker 的启动/serve 路径。

### R-76：统一 managed Worker 的 SIGTERM graceful shutdown

- 类别：26 个 Worker 只等 Ctrl-C，4 个 Worker 等 Ctrl-C 或 SIGTERM；同一 managed Worker 关闭行为分叉。
- BFS 覆盖：从 `a4n-server` 的 `spawn_managed_worker` 追到 `configure_parent_death_signal`，Linux 父进程退出会发 SIGTERM；从所有 30 个 Worker `run`/`serve` 入口追到 Axum `with_graceful_shutdown`。继续追到 `stop_child`：管理器发送 SIGTERM，最多轮询 50 次、每次间隔 100ms，之后才调用强制 `child.kill()`。`rg` 全仓列举 30 个 Worker helper 调用点，并确认不再有 `wait_for_ctrl_c` 引用。
- 将所有 30 个 Worker 的 graceful-shutdown future 统一为 `a4n-worker-runtime::wait_for_shutdown_signal()`，删除 Ctrl-C-only helper。Unix 统一响应 Ctrl-C/SIGTERM；非 Unix 维持 Ctrl-C 行为。Control 关闭 managed sidecars、Edge 响应父进程 stop flag、Tauri 触发应用退出的宿主级路径仍保持独立。
- 验证：`cargo check --all-targets --locked --offline` 覆盖 `a4n-worker-runtime` 和全部 30 个 Worker package，通过；`timeout 60s cargo test --locked --offline -p a4n-worker-runtime --lib wait_for_shutdown_signal_handles_sigterm -- --nocapture`：1/1 通过，子进程在收到 SIGTERM 后正常完成；`cargo fmt --all -- --check` 与 `git diff --check` 通过。
- 记录范围：`a4n-worker-runtime` signal helper，以及 30 个 managed Worker 的 startup/serve 调用点。

### R-77：统一 Server/Tauri 子进程终止流程

- 类别：重复子进程关闭实现。
- BFS 覆盖：从 `a4n-server::stop_child` 追踪 8 个 managed Worker/startup caller，覆盖 Worker readiness 失败、binary fingerprint 失败、重启、批量更新回滚和 Server shutdown；另追踪 `stop_control`/`stop_edge` 在生命周期、supervision 与 startup 回滚中的调用和错误聚合。从 Tauri `terminate_child` 追踪 Native Worker readiness/Drop、Local Server stderr 启动失败/健康检查失败/正常退出路径，并用 `rg` 复核该符号的所有生产引用。两处实现均为 Unix SIGTERM、最多 50 次 100ms 轮询、超时后强制 kill；Sidecar 的 HTTP shutdown 协议、CLI 按 PID/marker 校验的终止、控制台任务进程组关闭和测试夹具进程管理合同不同，排除在本项之外。
- 将等待与终止流程集中到 `a4n-worker-runtime::terminate_child`，Server/Tauri 调用同一实现并删除 Tauri 私有副本。SIGTERM 命令失败时先用 `try_wait` 区分已退出子进程，再记录失败并强制终止；最终 `kill`/`wait` 错误由 helper 显式返回。Server 保留 `anyhow` 错误传播；Tauri 保留既有调用点日志和启动错误策略，但不再吞掉最终 wait 错误。
- 新增 SIGTERM 被忽略时升级到强制终止的 Unix 回归用例；复用原 SIGTERM graceful-shutdown 子进程测试验证共享 helper。`timeout 60s cargo test --locked --offline -p a4n-worker-runtime`：5/5 通过；`cargo check --all-targets --locked --offline -p a4n-server -p a4n-tauri` 通过；`cargo fmt --all -- --check` 与 `git diff --check` 通过。构建期间在锁内离线更新了 `Cargo.lock`，并在验证结束后清理共享 agent target。
- 记录范围：`crates/process/a4n-worker-runtime/`、`crates/host/a4n-server/src/server/process.rs`、`crates/host/a4n-tauri/src/server_process.rs`。

### R-78：统一 Worker required capability 成员判断

- 类别：重复 capability 版本与启用状态判定。
- BFS 覆盖：从 Server `wait_for_worker_contract -> validate_worker_capabilities` 追踪所有 3 个本地调用及 `wait_for_managed_worker` catalog 输入；从 Tauri `wait_for_native_worker -> validate_native_worker_capabilities` 追踪 Native Worker catalog required list 和 `HttpWorkerClient::capabilities`；静态搜索核对当前 Worker capability 构造均以该 Worker 的 catalog contract version 填充版本。两端共同规则为“名称、版本匹配且 enabled”；Server 还会拒绝任意上报项的版本不等于全局版本，Tauri 仅校验 required 项，保留该宿主策略差异。
- 将 required capability 的单项判断移入 `WorkerCapabilities::has_enabled_capability`，Server 和 Tauri 复用；保留各自错误消息、调用顺序以及 Server 对全部上报项的严格版本检查。
- contract 回归用例覆盖名称、版本、enabled 三个条件。`timeout 60s cargo test --locked --offline -p a4n-worker-contract --lib`：55/55 通过；`cargo check --all-targets --locked --offline -p a4n-worker-contract -p a4n-server -p a4n-tauri` 通过；`cargo fmt --all -- --check` 与 `git diff --check` 通过。验证后在构建锁内清理共享 agent target。
- 记录范围：`crates/process/a4n-worker-contract/src/capabilities.rs`、`crates/process/a4n-worker-contract/src/wire_tests.rs`、`crates/host/a4n-server/src/server/health.rs`、`crates/host/a4n-tauri/src/server_process.rs`。

### R-79：删除 Server 子进程停止转发包装

- 类别：无策略的单行转发函数。
- BFS 覆盖：CodeGraph 与 `rg` 全仓搜索确认 `stop_control` 有 3 个调用点、`stop_edge` 有 2 个调用点，定义均只调用 `stop_child` 且没有额外错误映射、日志或资源清理；调用分别位于 startup lifecycle、remaining 和 supervision 重启/回滚流程。
- 删除两个 wrapper，5 个生产调用改为直接使用 `stop_child`。进程对象、停止顺序、结果聚合和错误消息上下文均保留。
- `cargo check --all-targets --locked --offline -p a4n-server` 通过；`cargo fmt --all -- --check` 与 `git diff --check` 通过。CodeGraph 同步修改文件后索引 up to date，报告 25 个 unresolved references；构建后在锁内清理 agent target。
- 记录范围：`crates/host/a4n-server/src/server/process.rs`、`crates/host/a4n-server/src/server/startup/{lifecycle,remaining,supervision}.rs`。

### R-80：统一前端 API 时间戳解析

- 类别：Canvas workflow state 与 panel 有逐行相同的 ISO 时间戳解析；Agent budget、Agent 消息运行时长、视频提示排序和灵感历史又各自直接或局部解析 API 时间字符串，通用 locale 日期 formatter 也用原生 `Date` 独立解释。
- BFS 覆盖：从 `parseWorkflowTimestampMs` 追到 Canvas checkpoint/current-step/discovery 和 `CanvasWorkflowPanel.runningWorkflowLooksStale`；从 `parseHistoryTime` 追到 `normalizeHistoryMessage`、三个 controller 消费点和 `getInspirationHistory`。检查 `InspirationMessageRecord.created_at` 的类型及写入/投影路径：数据库字段是 `NaiveDateTime`，写入按 `Utc::now().naive_utc()`，API 投影 `to_string()` 不携带时区。Agent 的两个 started-at 消费者来自同一 `AgentRunView`，后端序列化为 RFC3339；视频 prompt 的 `updated_at` 来自 API summary。另核对 `formatDate*` 的 79 个引用、Novel List 排序和 Session expiry 显示。
- 新增 `frontend/src/utils/timestamp.ts::parseApiTimestampMs` 作为 API 时间解析唯一实现：保留显式偏移，按 UTC 解释无偏移 ISO datetime，保留 date-only 的原生 ISO 语义；非法/空值返回 null。Canvas/Panel、Agent budget/消息列表、灵感历史、视频提示排序、项目更新时间排序、Session expiry 和 i18n 日期格式复用该 helper。调用方原有的抛错、null、elapsed=0、排序 NaN、invalid session expiry 状态策略不变。灵感历史现在按 UTC 解释服务端 naive UTC 时间，消除浏览器本地时区造成的消息顺序/显示偏移。
- 静态复核 `frontend/src` 后，生产代码的 `Date.parse` 仅留在该 helper；其它直接调用都只在测试中。受影响 Vitest 7 文件共 33/33 通过；`TZ=America/Los_Angeles` 下 timestamp、i18n、inspiration 3 文件 15/15 通过；`npm run build` 通过，保留已有大 chunk 提示；`git diff --check` 通过。
- 记录范围：`frontend/src/utils/timestamp{.ts,.test.ts}`、Canvas workflow state/panel、Inspiration history、Agent budget/message list、video frame prompt utilities、Novel List sorting、Account Session expiry 和 i18n date formatting。

### R-81：统一前端正安全整数 ID 边界

- 类别：API ID shared parser、路由守卫和选择回调分散校验正数/整数/安全整数；有限正数守卫会让小数路由进入 API 后才失败，`idNum` 通过 `Number.isInteger` 会接受超过 JS 安全精度的值。
- BFS 覆盖：从 `App.tsx` 注册的 Novel/Video/Comic/Galgame/Canvas/Writing 路由追到各 page 的 `Number(useParams().id)`、加载/恢复 callback、localStorage tab key 和对应 `getProject` API；继续追踪 Inspiration controller/blueprint、Novel List project selector 和 Agent monitor writing-target 到 API/导航。CodeGraph 确认 `idNum` 有 236 个调用者；全仓搜索列出全部 project-ID 条件并确认 API `idNum` 是其失败边界。
- 在 `numberRange.ts` 集中 `isSafeInteger` 与 `isPositiveSafeInteger`；`integerNum` 拒绝不安全整数但继续允许安全的负系统哨兵，`idNum` 与所有上述项目 ID 守卫共用正安全整数规则。页面现有错误文案、导航、状态清理和选择无效时返回 null 均保留。
- 回归覆盖零、负数、小数、非数字、`MAX_SAFE_INTEGER` 边界和系统哨兵。相关 Vitest 6 文件 26/26 通过；前端生产构建通过；迁移后搜索不再发现 projectId/requestProjectId 的有限正数/安全整数分叉；`git diff --check` 通过。
- 记录范围：`frontend/src/utils/numberRange.{ts,test.ts}`、`frontend/src/api/shared.ts`、API config shared test、Novel/Video/Comic/Galgame/Canvas/Writing/Inspiration/Agent Monitor 前端项目 ID 使用点。

### R-82：收敛前端 API 安全整数校验

- 类别：重复边界校验。
- BFS 覆盖：从 Agent SSE 事件、Auth portable storage、Blueprint/Canvas/Chapter API、生成结果校验、图像尺寸、运行时设置和 Visual Plan SSE 回执入口出发，沿各自的调用方、测试和 generated API 类型追踪到下游 API 请求、状态更新与错误路径；CodeGraph 与 `rg` 复核这些模块的全部 `Number.isSafeInteger`/`Number.isInteger` 生产引用。
- 9 个 API 文件改用现有 `frontend/src/utils/numberRange.ts` 的 `isSafeInteger`/`isPositiveSafeInteger`，删除重复的类型、整数和正数组合判断；运行时设置校验直接返回已完成类型收窄的数值，图像尺寸保留 32 位上限，其他错误文案、顺序和失败策略不变。章节生成目标原本只要求安全整数，因此保留非正章节号的正则边界，不扩大为正数校验。
- 静态搜索确认 `frontend/src/api` 已无直接 `Number.isSafeInteger`/`Number.isInteger`；未新增兜底、兼容或回退逻辑。
- 记录范围：`frontend/src/api/{agent,authStorage,blueprint,canvas,chapter,generationResultVerification,visualPlanEdits}.ts`、`frontend/src/api/image/types.ts`、`frontend/src/api/localRuntimeSettingsValidation.ts`。

### R-83：收敛前端页面正安全整数校验

- 类别：重复边界校验。
- BFS 覆盖：从 Page Session URL/query 上下文、Domain Agent 绑定、Agent 监控写作目标、内容优化 diff、角色章节输入、视频画布配置/连线/节点引用、视频参考图元数据/提示词，以及写作台/优化/导演时间线选择回调出发，沿测试调用和下游导航、API 请求、图像/提示词消费路径核对校验语义。
- 14 个页面/组件工具改用 `numberRange.isPositiveSafeInteger`，删除逐处的 `typeof`、`Number.isInteger`/`Number.isSafeInteger` 与 `> 0` 组合；字符串正则、引用数量上限、时间线总时长和其他字段合同保持原样。允许 0 的通用视频时长整数解析、带独立范围的配置校验和 32 位上限路径未合并。
- 静态复核确认本批选定的正 ID/引用路径不再复制安全整数谓词；未新增兜底、兼容或回退逻辑。
- 记录范围：`frontend/src/hooks/usePageSession.ts`、`frontend/src/components/agent/{DomainAgentBinding,contentOptimizationDiff}.tsx/ts`、`frontend/src/pages/{agentMonitorUtils.ts,character-detail-dialog/manageViewUtils.ts,video-studio/{canvasAssemblyEdges,canvasConfig,canvasGraphMapping,canvasNodeRefs}.ts,novel-detail/video/{videoImageUtils,videoFramePromptUtils}.ts,writing-desk/WritingDeskHeader.tsx,novel-detail/optimize/OptimizeHeader.tsx,novel-detail/visual-plan/DirectorTimeline.tsx}`。

### R-84：统一前端结果引用与正整数输入校验

- 类别：重复边界校验。
- BFS 覆盖：从章节/灵感/蓝图生成结果引用恢复路径、Galgame 媒体绑定、视频媒体引用与大纲工具，到写作台章节范围、项目列表派生章节范围、管理测试中心和提示词项目选择，追踪到历史记录查找、章节重载、导航和 API 请求；CodeGraph 复核调用方、既有测试和生成合同 ID 类型。
- 11 个路径改用 `isPositiveSafeInteger`，蓝图章节数改用 `isSafeInteger` 后继续保留 5–10000 范围；保留正则格式、章节 32 位上限、范围关系和错误文案。结果引用 ID 现在拒绝 0，避免把无效标识当作可恢复记录；未新增兜底、兼容或回退逻辑。
- 静态搜索确认本批选定文件不再直接调用 `Number.isSafeInteger`/`Number.isInteger`；允许空值或 0 的通用数值解析不在本批合并。
- 记录范围：`frontend/src/pages/{NovelList.tsx,WritingDesk.tsx,admin/TestCenterPage.tsx,inspiration/{useBlueprintWorkflow,useInspirationChatController}.ts,novel-detail/galgame/MediaAssetIdBinder.tsx,novel-detail/outlines/outlineTabUtils.ts,novel-detail/video/videoTabShared.ts,settings/prompt/PromptProjectSelect.tsx,writing-desk/{generationWordCount,useChapterStreamGeneration}.ts}`。

### R-85：统一档案与章节表单正安全整数校验

- 类别：重复边界校验。
- BFS 覆盖：从 Object/Scene Profile 编辑器、章节手动导入、角色属性删除标记和事实反馈间隔入口，追踪到 API input payload、章节/档案数据库消费者、并发操作状态和现有表单测试；CodeGraph/static search 复核五个路径的字段类型与范围。
- 五个页面组件改用 `isPositiveSafeInteger`，保留对象/场景的首末章节顺序检查、事实反馈的 `2147483647` 上限、删除标记字段完整性和原错误提示。未新增兜底、兼容或回退逻辑。
- 静态搜索确认本批文件不再直接复制 `Number.isSafeInteger`/`Number.isInteger` 组合；0、时间轴位置和其他不同合同仍保留原实现。
- 记录范围：`frontend/src/pages/novel-detail/{ObjectProfilesTab,SceneProfilesTab,ChaptersTab,FactFeedbackPanel}.tsx`、`frontend/src/pages/character-detail-dialog/DeletionManager.tsx`。

### R-86：统一设置与视频选项安全整数底层校验

- 类别：重复边界校验。
- BFS 覆盖：从图像/LLM/视频设置表单、Agent 预算编辑、Web Search/Web Fetch 保存入口和视频章节选项解析器出发，追踪到配置 input、提交 API、时长范围判定和视频生成调用；CodeGraph/static search 核对各字段的 0/正数/范围语义。
- 6 个文件改用 `isSafeInteger`/`isPositiveSafeInteger` 作为底层整数谓词，保留每个模块自己的尺寸上限、预算上限、时长 1–60 范围、上下文/输出比较、重复值和错误文案。未新增兜底、兼容或回退逻辑。
- 静态搜索确认本批设置与视频选项文件不再直接使用 `Number.isSafeInteger`/`Number.isInteger`；允许 0 或有独立时间轴合同的路径未被强行合并。
- 记录范围：`frontend/src/pages/settings/{image/imageConfigUtils.ts,llm/llmConfigUtils.ts,video/videoConfigUtils.ts,WebSearchConfigPanel.tsx}`、`frontend/src/components/agent/AgentBudgetDialog.tsx`、`frontend/src/pages/novel-detail/video/videoOptionUtils.ts`。

### R-87：统一页面视频与内容工具安全整数校验

- 类别：重复边界校验。
- BFS 覆盖：从大纲/游戏循环/改编页数、Embedding 参数、漫画来源、导演时间轴，到视频组装、后处理、参考媒体、结果展示、插入段生成和章节时长编辑入口，追踪到 localStorage 恢复、编辑状态、视频 API payload、排序与显示路径；CodeGraph 复核已有视频测试及动态组件调用。
- 14 个页面/视频工具改用 `isPositiveSafeInteger` 或 `isSafeInteger`，保留 0 合法的章节分段/时间轴位置、漫画 `startSection` 的非负合同、视频时长范围、循环 1–10000 上限和原错误文案。未新增兜底、兼容或回退逻辑。
- 静态搜索确认本批文件不再直接复制 `Number.isSafeInteger`/`Number.isInteger`；其他 segment index/时长状态机路径仍待下一轮按其专属合同复核。
- 记录范围：`frontend/src/pages/{derived/AdaptationPlanEditor.tsx,novel-detail/{OverviewTab,OutlinesTab,ChapterVisualPlanTab,MangaTab}.tsx,novel-detail/galgame/StoryGraphPanel.tsx,novel-detail/video/{VideoAssemblyWorkspace,videoAssemblyUtils,VideoResultsPane,VideoPostProcessWorkspace,referenceMediaWorkspace,useVideoTabActions,VideoGenerationEditor}.tsx,settings/EmbeddingTab.tsx}`。

### R-88：收敛视频 segment 与进度整数校验

- 类别：重复边界校验。
- BFS 覆盖：从视频 segment plan/recipe、execution identity、draft loader、tab state/view state，到 Galgame choice、Canvas 数字字段和通用 workflow progress total，追踪到持久化恢复、视频 API 请求、图谱选择和 UI 进度显示；CodeGraph 复核对应测试与调用链。
- 8 个文件改用安全整数谓词，正 segment/identity/duration 使用 `isPositiveSafeInteger`，允许任意整数的字段使用 `isSafeInteger`；保留现有 1–60 时长范围、segment 关系、`step === "any"`、场景查找和默认总量行为。未新增兜底、兼容或回退逻辑。
- 全仓前端静态搜索目前只剩 `CharacterDetailDialog` 的通用整数解析和 `numberRange` 自身实现；前者不属于正 ID/安全范围合同，保留并记录为待独立语义审查。
- 记录范围：`frontend/src/hooks/useProgressTask.tsx`、`frontend/src/pages/novel-detail/video/{videoSegmentPlan,videoExecutionIdentity,loadVideoTabData,useVideoTabState,useVideoTabViewState}.ts`、`frontend/src/pages/novel-detail/galgame/GalgameChoiceEditor.tsx`、`frontend/src/pages/video-studio/CanvasNodeInspector.tsx`。

### R-89：统一 Character Detail 可选章节安全整数解析

- 类别：重复边界校验。
- BFS 覆盖：从 Character Detail 冲突范围表单的 start/end chapter 输入追踪到冲突检查与同步 API payload；确认空值仍映射为 `null`，负数/零的既有业务语义不被改写。
- 将通用整数判断替换为 `isSafeInteger`，只拒绝超过 JS 安全精度的数；错误文案、空值处理和下游请求保持不变。
- 静态搜索确认 `frontend/src` 生产代码仅剩 `numberRange.ts` 自身直接调用 `Number.isSafeInteger`；未新增兜底、兼容或回退逻辑。
- 记录范围：`frontend/src/pages/CharacterDetailDialog.tsx`。

### R-90：核验并保留 Agent Runtime application accessor 边界

- 类别：候选转发层复核，未删除。
- BFS 覆盖：从 `AppState` 的全部 Agent/Agent Worker application accessor 追到 Runtime actions、Agent Runtime Host、Control route 和 `a4n-agent-runtime-contract` trait；共核对 30 余个 accessor 的返回 trait、调用点、所有权和异步生命周期。
- 这些方法不是同一事实的重复 API：每个方法返回不同的 application port trait object，形成编译期能力边界；`agent_worker_lease_application` 与 `agent_worker_failure_application` 返回 `Arc`，调用者需要跨异步调用持有所有权。收敛为一个具体字段 accessor 会暴露实现、绕过端口隔离并增加耦合，因此保留。
- 记录范围：`crates/host/a4n-runtime/src/state/app/agent.rs`、`crates/host/a4n-runtime/src/actions/agent_runtime/**`、`crates/host/a4n-runtime/src/agent_runtime_host.rs`、`crates/agent/a4n-agent-runtime-contract/src/ports.rs`。

### R-91：核验 baseline model config seed 分叉

- 类别：候选重复实现，未合并。
- BFS 覆盖：从 `ensure_baseline_model_configs` 追到 LLM、Image、Video 三条 seed 路径及 `bootstrap` 调用，核对环境变量表、fallback 配置、API key 优先级、base URL 推导、数值转换、输入结构和插入端口。
- LLM/Image 虽共享“配置表为空才 seed”的外壳，但字段集合、缺少凭证错误、OpenAI 专用环境变量、默认 URL、JSON extra body 和输出合同不同；Video 还使用独立 provider 配置。抽成泛型会把这些业务差异塞进闭包，保留三条清晰 owner 更低耦合。
- `cargo machete` 全仓扫描未发现未使用 Cargo 依赖。未新增兜底、兼容或回退逻辑。
- 记录范围：`crates/host/a4n-runtime/src/baseline_configs/{mod,llm,image,video,env}.rs`、`crates/host/a4n-runtime/src/state/app/bootstrap.rs`、相关 `ConfigRead/ConfigMutation` ports。

### R-92：全仓死代码与兼容标记复核

- 类别：显性死代码/伪安全路径审计，未删除不确定命中。
- BFS 覆盖：CodeGraph 扫描 Host、Runtime、Platform、Process、Worker、Frontend 和 Scripts 的零调用候选；`rg` 复核 `allow(dead_code)`、兼容/legacy/fallback 标记、生成入口、迁移注册、trait 实现和路由注册；`cargo machete` 检查 workspace 直接依赖。
- 零调用候选均落在迁移 `up/down`、测试夹具、schema/合同生成二进制、trait/动态路由入口或已有生产消费者；legacy/compat 命中均有数据库迁移、历史导入、持久快照重放或明确 UI fallback 调用。未发现可在所有活跃路径中证明无用的生产模块、配置项或异常分支，因此没有删除。
- `cargo machete` 结果：未发现未使用 Cargo 依赖。未新增兜底、兼容或回退逻辑。
- 记录范围：全仓 `crates/{host,platform,process,agent,planning,project,video}`、`frontend/src`、`scripts` 的 CodeGraph/静态搜索结果；具体保留证据继续汇总在本节后续条目。

### R-93：核验 Agent Control Worker route 骨架分叉

- 类别：候选重复路由骨架复核，未合并。
- BFS 覆盖：从 `worker_routes.rs` 的 52 个 Agent Worker handler 追到 Control 路由注册、每个 `AgentWorker*Request` 的校验实现、runtime action、响应类型和 204/JSON 状态分支。
- 共同的鉴权与 bad-request 映射只是薄协议边界；实际校验既有普通 `validate`，也有 `validate_agent_run` 等专用方法，响应同时包含 `Json<Value>`、具体 wire 类型、`Response`/204 和不带 payload 的 action。泛化为闭包/trait/macro 会隐藏 route 合同并增加抽象层，因此保留显式 handler。
- 记录范围：`crates/host/a4n-control/src/routes/agent_runtime/worker_routes.rs`、`crates/host/a4n-control/src/routes/mod.rs`、`crates/host/a4n-runtime/src/actions/agent_runtime/**`、`crates/agent/a4n-agent-runtime-contract/src`。

### R-94：核验本机 Runtime Settings CRUD 分叉

- 类别：候选重复配置实现复核，未合并。
- BFS 覆盖：从 `localRuntimeSettings.ts` 的 LLM/Image/Video/Embedding/Web settings API 追到 `mutateSettings`、写入串行化、normalizer、默认源状态和前端 Config 面板调用；核对各类字段、API key 保留/清除语义和 active/default 行为。
- 已共享的事实源包括 `mutateSettings`、`serializeSettingsWrite`、`allocateId`、`setSingleDefault`、`removeConfigById`、`activeLocalConfig` 和各自的 `normalizeLocal*Config`。各 provider CRUD 剩余差异是结构化字段、可选密钥更新策略、系统默认源和必填/可空字段；再抽一个泛型 CRUD 会隐藏合同差异，保留显式类型边界。
- 记录范围：`frontend/src/api/localRuntimeSettings.ts`、`frontend/src/api/localRuntimeSettingsValidation.ts`、`frontend/src/api/{config, image, video}.ts`、设置页面调用方。

### R-95：统一 LLM/Image connectivity target 解析

- 类别：重复配置目标解析。
- BFS 覆盖：从 LLM/Image 的 test/preview API 入口追到 server/local scope、system sentinel、系统默认加载器、本机配置读取和 runtime snapshot；确认两条 resolver 仅在 ID 标签、配置类型和 provider loader 上有差异。
- 新增 `frontend/src/api/config/shared.ts::resolveProviderConnectivityTarget`，LLM/Image resolver 复用同一 server/system/local 分支逻辑；Embedding 没有 system sentinel，保留独立 resolver。系统 API key 要求、本地 runtime 判断、错误和 snapshot 选项均由调用方继续持有。
- 新增 shared helper 的 server/system/local 回归测试；相关 Vitest 15/15、生产构建和 `git diff --check` 通过。未新增兜底、兼容或回退逻辑。
- 记录范围：`frontend/src/api/config/shared.ts`、`frontend/src/api/config/{llm.ts,embedding.ts}`、`frontend/src/api/image/configs.ts`、`frontend/src/api/config/shared.test.ts`。

### R-96：删除 Rust DB workflow/Agent 同类型转换

- 类别：显性冗余表达式。
- BFS 覆盖：从 Clippy 报告的 `AgentInboxRecord` tuple 返回和 Workflow summary wrapper 追到 `a4n-db` delegation、workflow mutation/claim/progress callers，核对 `WorkflowWriteError` 与返回记录的具体类型。
- 删除 2 个同类型 `AgentInboxRecord.into()` 和 12 个同类型 `WorkflowWriteError::from`；保留 summary projection 的真实错误转换以及其它 ActiveModel/跨层转换。行为、错误类型和 API 合同不变。
- 验证：目标 workspace Clippy 完成；本批警告消失，剩余仅参数数量、类型体积和既有结构提示；`cargo fmt --all -- --check`、`git diff --check` 通过。
- 记录范围：`crates/platform/a4n-db/src/agent/delegations.rs`、`crates/platform/a4n-db/src/workflow/runs/{claim,mutations,progress}.rs`。

## 核验后保留

- `crates/host/a4n-api-contract/src/bin/export_generation_schema.rs` 的 `GenerationContractBundle` 由 schema derive 读取，且 `scripts/contracts/generate-generation-contracts.sh` 明确调用该二进制；它不是未使用模块。
- `crates/host/a4n-orchestrator/src/project/family_package/mod.rs` 的 `Model` 别名位于 `cfg(test)` 路径，项目包测试 fixture 使用这些类型；未移除测试支撑代码。
- `normalize_legacy_editor_source` 有 `parse_persisted_editor_source` 生产调用；`archive_legacy_source_sync_payload` 由媒体迁移调用；`maximum_manifest_id` 参与旧 family package 导入。它们仍覆盖输入或持久数据路径，不能按名称认定为伪安全回退。
- 项目包升级路径也保留：CodeGraph 从 `ProjectBundlePackage::from_manifest` 追到 `ProjectFamilyPackage::from_manifest`；它识别 v5/v6/v7，`upgrade_shooting_contract` 仅改写 v5/v6 的旧剧本合同，并由现存 package fixtures 覆盖。`a4n-domain::media_authoring::legacy` 则分别由已注册的 `media_authoring` DB migration 和 family-package history importer 调用，转换旧工作流与 source-sync payload；`legacy_asset_lineage_shadow_cleanup` 同样注册在 `Migrator` 中。它们是当前导入/数据库升级入口，不是已退役的运行时 fallback。
- `ChatRequest::from_snapshot_value` 的旧 `cache_user_prefix_len`/`cache_system_prefix` 处理继续保留：CodeGraph 从 `restore_model_request_wire` 追到 Agent worker/model replay，再由 Canvas text generation 与 Optimize accounting 追到 `agent_execution.arguments_json` 的 durable read；`snapshot_value` 是对应写入路径。父提交显示旧字段曾作为请求字段序列化；`agent_replay_policy_canonicalization` 只规范 `replay_policy`，没有重写请求 JSON；迁移 SQL 搜索未发现生产代码删除这些历史执行记录。`cache_user_prefix_len` 转为新 cache breakpoint；旧 `cache_system_prefix` 从严格 DTO 输入移除并校验类型，当前 cache 配置从 system message 派生 breakpoint。移除这些读取会让仍存储的旧快照重放失败。
- Sidecar bootstrap 的 tracing 初始化还负责文件日志保留、鉴权和关闭路由，不能替代普通 Worker runtime；`a4n-control` 与 `a4n-edge` 的日志、关闭行为也独立保留。
- `cargo-machete` 未发现未使用 Cargo 依赖。`cargo tree -d` 中的多个版本属于传递依赖组合，没有足够证据作为可移除的直接重复实现。
- `ProjectStatusWriteError` 在 Blueprint DB、Import Reverse DB、Project Status Worker wire 与 Orchestrator AppError 边界的转换分类不同：`StatusChanged` 分别变为 InvalidArgument、InvalidState 或 Conflict，消息也保留各自的 expected/actual 与 operation 语义；对应源码为 `blueprint/manual.rs`、`import_reverse/candidate_apply.rs`、`a4n-project-status-worker/src/lib.rs` 和 `project/errors/lifecycle.rs`，不合并。
- `CanvasDbError` 的 Worker mapper 与 Orchestrator mapper 分别保留 wire Conflict 消息和具体 AppError operation；`CharacterDbError` 的 Worker mapper 把 Internal/Generation/Serialization 归为 Unavailable，而 Orchestrator 保留各自 AppError 类型。`GenerationDbError -> WorkerFailure` 只核到 Outline Worker 一个生产消费者，不增加单调用点 helper。
- Project Read、Project Mutation、Project Status 的 `ProjectSummaryError -> WorkerFailure` 四路组合 match 当前相同。其 Progress/Family/Collaboration 子映射已分别共享；把组合 match 放入通用 process adapter 会令所有 adapter 消费者依赖 `a4n-project-read-core`，另建项目专用 adapter 又只服务 3 个调用点，因此保留这三处小型组合 match。
- Object Profile/Scene Profile 的四处 `ObjectProfileInputError`/`SceneProfileInputError -> InvalidRequest` 和两处 Downstream conflict `String -> InvalidRequest` 都只有少数局部调用，且前者要求 Display 到 String 的额外适配；没有为几行转换再新增 Worker contract helper。
- Worker readiness 的 Server `wait_for_worker_contract` 与 Tauri `wait_for_native_worker` 都轮询子进程和 health，但同步/异步 transport、退出状态、超时错误和 Server 对全部上报项的严格版本策略不同；未合并整段轮询。共同的 required-capability 单项判断已在 R-78 收敛到 Worker contract。CodeGraph 确认各自生产调用均只有一处，静态复核对应的 readiness 和 capability validation 路径。
- 前端日期解析已由 `frontend/src/i18n/format.ts::parseDate` 共用；数值范围与 CSS RGB 解析已有共享 utility。本次未发现需要再拆分的新事实来源。

## 验证

- `cargo check --workspace --locked --offline`：续作后全 workspace 通过，无警告。
- `cargo check --workspace --all-targets --locked --offline`：通过，编译全 workspace 的测试/示例目标，无警告；本轮 23 个通用 handler 迁移后再次执行通过。
- `cargo check --all-targets --locked --offline -p a4n-worker-runtime -p a4n-portrait-worker`：通过，覆盖共享 helper 与 Portrait 的 state-capturing 闭包适配。
- `timeout 60s cargo test --locked --offline -p a4n-worker-runtime --lib`：1/1 通过，覆盖未授权短路、成功响应元数据和失败 wire 映射。
- R-14 `cargo check --all-targets --locked --offline`：`a4n-worker-runtime` 与 27 个 raw-byte authorizer 消费 Worker 通过，无警告；`timeout 60s cargo test --locked --offline -p a4n-worker-runtime --lib`：2/2 通过，覆盖 token bytes 和 command response 路径。
- R-15 `cargo check --all-targets --locked --offline`：`a4n-worker-contract`、runtime 和 26 个 Worker 消费者通过，无警告；contract 测试 49/49 通过。
- R-16 `cargo test -p a4n-worker-contract --offline --locked`：50/50 通过；Asset Graph、Canvas Record、Video Record 的 scope 定向测试各 1/1 通过；`cargo check --workspace --all-targets --offline --locked`：全 workspace 通过，无警告。
- R-17 `timeout 60s cargo test -p a4n-worker-contract --offline --locked`：51/51 通过；`cargo check --all-targets --offline --locked -p a4n-worker-contract -p a4n-scene-profile-worker -p a4n-video-record-worker`：通过，无警告。
- R-18 `cargo test -p a4n-worker-client --offline --locked --no-run`：测试 binary 编译通过；随后 `timeout 60s cargo test -p a4n-worker-client --offline --locked`：16/16 通过。`cargo check --workspace --all-targets --offline --locked` 完成且无编译错误；该组合 shell 的最终状态为 124，因为首次冷构建测试超时，后续独立测试重跑通过。
- R-19 `cargo test -p a4n-worker-client --offline --locked --no-run` 后，`timeout 60s cargo test -p a4n-worker-client --offline --locked`：17/17 通过；`cargo check --all-targets --offline --locked` 覆盖 11 个受影响 Worker client package，通过且无警告。
- R-20 `cargo test --locked --offline -p a4n-worker-contract --lib`：52/52 通过；`cargo check --all-targets --locked --offline` 覆盖 6 个受影响 Worker package，通过且无警告。
- R-21 `cargo check --all-targets --locked --offline -p a4n-chapter-content-worker-client -p a4n-chapter-scene-worker-client`：通过且无警告。
- R-22 `cargo check --all-targets --locked --offline -p a4n-portrait-worker -p a4n-theme-read-worker -p a4n-import-review-worker -p a4n-workflow-read-worker`：通过且无警告。
- R-23 `cargo check --workspace --all-targets --locked --offline`：首轮暴露 Chapter Content 与 Manga worker 的残余 import；修复后，`cargo check --all-targets --locked --offline -p a4n-asset-graph-worker -p a4n-canvas-record-worker -p a4n-chapter-content-worker -p a4n-chapter-scene-worker -p a4n-config-read-worker -p a4n-inspiration-worker -p a4n-manga-record-worker -p a4n-blueprint-worker -p a4n-foreshadowing-worker -p a4n-novel-planning-worker -p a4n-outline-worker -p a4n-character-mutation-worker -p a4n-object-profile-worker -p a4n-portrait-worker -p a4n-profile-material-worker -p a4n-scene-profile-worker -p a4n-project-access-worker -p a4n-project-collaboration-worker -p a4n-project-mutation-worker -p a4n-project-read-worker -p a4n-project-status-worker -p a4n-source-snapshot-worker -p a4n-theme-read-worker -p a4n-import-review-worker -p a4n-video-record-worker -p a4n-workflow-read-worker`：通过且无警告。全 workspace 首轮的 client unused `WorkerPrincipal` import 警告是既有代码，不属于本次改动；此检查不作为通过结果记录。
- R-24 `cargo check --all-targets --locked --offline -p a4n-agent-worker-client -p a4n-asset-graph-worker-client -p a4n-canvas-record-worker-client -p a4n-blueprint-worker-client -p a4n-foreshadowing-worker-client -p a4n-novel-planning-worker-client -p a4n-outline-worker-client -p a4n-object-profile-worker-client -p a4n-portrait-worker-client -p a4n-profile-material-worker-client -p a4n-scene-profile-worker-client -p a4n-project-status-worker-client -p a4n-source-snapshot-worker-client -p a4n-theme-read-worker-client`：通过且无警告。静态搜索确认 14 个文件不再引用已移除类型。
- R-25 `cargo test --locked --offline -p a4n-worker-db-adapter --lib`：2/2 通过；adapter 与 24 个受影响 Worker 的 `cargo check --all-targets --locked --offline`：通过且无警告。
- R-26 `cargo test --locked --offline -p a4n-worker-contract --lib`：53/53 通过；`cargo check --all-targets --locked --offline` 覆盖 contract 和 8 个受影响 Worker，通过且无警告。
- R-27 `cargo test --locked --offline -p a4n-worker-contract --lib`：54/54 通过；`cargo check --all-targets --locked --offline` 覆盖 contract 和 6 个受影响 Worker，通过且无警告。
- R-28 `cargo test --locked --offline -p a4n-worker-db-adapter --lib`：3/3 通过；adapter 与 6 个消费者的 all-targets 检查通过且无警告。
- R-29 `cargo test --locked --offline -p a4n-worker-db-adapter --lib`：6/6 通过；adapter 与 Chapter Content、Manga Record、Video Record、Workflow Read、Novel Planning、Portrait 六个消费者的 all-targets 检查通过且无警告。
- R-30 `timeout 60s cargo test --locked --offline -p a4n-worker-client --lib`：18/18 通过；`cargo check --all-targets --locked --offline` 覆盖 `a4n-worker-client` 与 29 个 typed Worker client crate，通过且无警告。
- R-31 `timeout 60s cargo test --locked --offline -p a4n-worker-db-adapter --lib`：7/7 通过；`cargo check --all-targets --locked --offline -p a4n-worker-db-adapter -p a4n-chapter-content-worker -p a4n-video-record-worker` 通过且无警告。
- R-32 `cargo check --all-targets --locked --offline -p a4n-db`：通过；`timeout 60s cargo test --locked --offline -p a4n-db --lib outline_db_errors_preserve_project_outline_record_mapping`：1/1 通过。
- R-34 `cargo check --all-targets --locked --offline -p a4n-worker-db-adapter -p a4n-project-read-worker -p a4n-project-mutation-worker -p a4n-project-status-worker`：通过；`timeout 60s cargo test --locked --offline -p a4n-worker-db-adapter --lib`：9/9 通过。
- R-35 `cargo test -p a4n-worker-db-adapter --locked`：12/12 通过；`cargo check --all-targets -p a4n-object-profile-worker -p a4n-profile-material-worker -p a4n-scene-profile-worker --locked`：通过且无警告。
- R-36 `cargo test -p a4n-worker-db-adapter --locked`：13/13 通过；`cargo check --all-targets -p a4n-portrait-worker -p a4n-import-review-worker -p a4n-video-record-worker --locked`：通过且无警告。
- R-37 `cargo test -p a4n-worker-db-adapter --locked`：13/13 通过；`cargo check --all-targets -p a4n-chapter-scene-worker -p a4n-chapter-content-worker -p a4n-project-read-worker -p a4n-profile-material-worker -p a4n-blueprint-worker -p a4n-manga-record-worker -p a4n-character-mutation-worker --locked`：通过且无警告。
- R-38 `cargo test -p a4n-worker-db-adapter --locked`：13/13 通过；Foreshadowing 保留 Internal 分类回归 1/1 通过；覆盖 24 个 Collaboration 消费 Worker 的 `cargo check --all-targets --locked`：通过且无警告。
- R-39 `cargo test -p a4n-worker-db-adapter --locked`：13/13 通过；Chapter Content、Manga Record、Blueprint、Novel Planning、Portrait、Project Mutation、Project Read、Project Status、Source Snapshot、Video Record、Workflow Read 的 all-targets 检查通过且无警告。
- R-40 `cargo test -p a4n-worker-db-adapter --locked`：13/13 通过；覆盖 17 个受影响 Worker 的 `cargo check --all-targets --locked` 通过且无警告，新增 Admin、Config Read、Workflow Lifecycle 的 adapter Cargo 依赖并更新 lockfile。
- R-43 `cargo test -p a4n-config web_and_control_addresses_share_environment_and_port_rules --locked`：1/1 通过；`cargo check --all-targets -p a4n-config -p a4n-agent-worker -p a4n-server --locked` 通过且无警告。
- `timeout 60s cargo test --locked --offline -p a4n-worker-contract --lib`：49/49 通过，覆盖共享失败构造、响应元数据、请求身份校验、principal ID 边界和 wire 序列化。
- `timeout 60s cargo test --locked --offline -p a4n-worker-client --lib`：14/14 通过，覆盖共享传输与响应错误映射。
- `cargo check --locked --offline -p a4n-worker-runtime -p a4n-service-manager -p a4n-server -p a4n-tauri`：通过，验证共享父进程退出信号 helper 的三个集成点和更新后的 lockfile。
- `timeout 60s cargo test --locked --offline -p a4n-config worker_listen_addr_requires_loopback_and_nonzero_port --lib`：1/1 通过。
- Worker client Debug 脱敏定向测试：30 个 client 各 1 项通过；共享 URL/token 解析测试 1/1 通过；28 个环境读取消费者 `cargo check` 通过。
- `cargo fmt --all -- --check`：通过。
- `git diff --check`：通过。
- R-16 完成后在 `/home/fufu/Code/A4N/work/.agent-build.lock` 独占锁内清理 `/home/fufu/Code/A4N/work/.build/agent-target`。
- R-17 完成后在 `/home/fufu/Code/A4N/work/.agent-build.lock` 独占锁内清理 `/home/fufu/Code/A4N/work/.build/agent-target`。
- R-18 完成后在 `/home/fufu/Code/A4N/work/.agent-build.lock` 独占锁内清理 `/home/fufu/Code/A4N/work/.build/agent-target`。
- R-19 完成后在 `/home/fufu/Code/A4N/work/.agent-build.lock` 独占锁内清理 `/home/fufu/Code/A4N/work/.build/agent-target`。
- `cargo-machete`：未发现未使用依赖；R-14 移除的 Worker `subtle` 直接依赖无遗漏。
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`：未通过；命中基线 `a4n-domain` 的 5 个既有 lint（`large_enum_variant`、`collapsible_if`、`nonminimal_bool`），未涉及本批改动，未修改以掩盖该失败。
- `npm ci`：成功；Vitest 配置时间、Agent Context Meter 和 i18n 三份测试文件 10/10 通过；`npm run build` 通过。构建保留既有 >500 kB chunk 提示；npm audit 显示 37 项现有依赖漏洞（1 low、5 moderate、31 high），本次未修改锁定的三方依赖。
- agent 公共构建目录在构建锁内清理；未运行数据库、Server 或前端服务。

## 覆盖边界

- 前端检查覆盖路由/API 关系、共享日期/数字 formatter、配置测试时间和 Agent Context Meter 调用；生产 TypeScript/Vite build 与定向测试通过，未运行浏览器端到端流程。
- Cargo 未使用依赖扫描不检查未使用的公开 API；Rust workspace 检查只编译当前平台的默认生产目标，不能证明跨平台运行或外部二进制调用不存在。
- 仓库中第三方传递依赖的重复版本和仍在使用的 legacy 输入/数据库迁移仍需后续人工架构审阅。Worker 现在共用相同的 Ctrl-C/SIGTERM 等待逻辑，但各服务的信号后清理流程仍独立保留。
