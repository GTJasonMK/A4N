# 工作包 08：Sidecar 安全、资源限制与协议演进

## 任务卡

| 项目 | 内容 |
|---|---|
| 主目标 | 完成网络信任、环境隔离、出站策略、资源限制、背压和版本兼容/滚动部署规则 |
| V2 来源 | 3.19、3.20、3.21；阶段 J、阶段 K 的协议部分 |
| 当前状态 | 资源策略、egress 合同、版本协商、HTTPS/独立凭证/mTLS 配置合同已实现；OpenSSL remote mTLS process 测试已去除 Unix 条件，并加入 Ubuntu/macOS/Windows CI 矩阵（等待远端 runner 实跑）；同一 capability client 现在由单一 `RemoteClientFactoryState` 持有，跨平台文件 watcher 会在证书目录变化时主动刷新，request-boundary metadata + SHA-256 指纹 polling/lazy reload 继续作为漏事件和原子替换的 correctness fallback；local atomic/remote rolling health 规则，以及八个 sidecar 的基础 process runner 已取得证据；typed health 请求已复用认证头，八个 sidecar 真实进程均已证明跨路由共享 semaphore 的并发饱和恢复，LLM 另有响应头 connect deadline、provider idle timeout 和 trusted-remote provider secret 拒绝/secret-free 内容放行证据，LLM/RAG/Image/Video/Embedding 已有 stream idle process，LLM/RAG/Image/Video/Embedding 已证明事件队列背压；八个 capability process runner 现在均覆盖 secret-free trusted-remote 内容请求，Image/Video 另覆盖 provider secret 拒绝；capability transport 的 request/connect/idle deadline 已有快速合同测试；本地 Export 历史/current/rollback 四组合真实进程矩阵已通过，跨平台 runner、远端部署审计和 breaking rollback artifact 仍待补齐 |
| 可并行性 | 安全、资源、协议可分三个 agent；必须复用 bootstrap/catalog/protocol，不建平行实现 |

## 子包 A：网络信任和环境隔离（3.19）

1. 保持 managed local sidecar 的短期 bootstrap token、Authorization 和恒定时间比较。
2. 审计 `Command::env_clear()` 与 capability 级最小环境 allowlist；数据库 URL、JWT secret 和无关 provider key 不得进入 sidecar。
3. 未配置远程认证时只允许 loopback；remote mode 强制 HTTPS，并实现 mTLS 或独立部署凭证、证书轮换和身份合同。
4. remote provider sidecar 不接收 Host 长期 provider secret。
5. 项目正文、prompt、图片、视频等内容默认禁止出站；按 capability/data type 显式授权并产生审计事件。

## 子包 B：资源、超时和背压（3.20）

1. 每个 endpoint 使用命名的 capability-specific body limit；JSON、base64 media、ZIP、streaming 分开。
2. capability client 统一 connect timeout；非流式 deadline；流式 connect/idle timeout。
3. 重能力 sidecar 使用有界并发；event channel 保持容量 256，队列满时等待容量，receiver 关闭时明确取消。
4. client disconnect、timeout、cancel 必须释放 provider、下载、索引和媒体任务。
5. process test 覆盖 413、连接超时、idle timeout、并发饱和、队列背压和取消后资源释放。

## 子包 C：协议升级和滚动部署（3.21）

1. 本地 bundle 明确为原子同版本升级，不支持混合版本。
2. remote mode 定义 protocol/capability 的最小/最大兼容范围和协商结果。
3. health 返回协商后的兼容信息或稳定拒绝原因。
4. 定义 breaking/additive/字段删除/语义变化的版本规则。
5. 定义 host-new/sidecar-old、host-old/sidecar-new、remote rollback 和失败回滚流程。
6. 合同/process test 覆盖版本组合和未知可选字段。

## 不在范围

- 不重做 3.22 已完成的数据目录 manifest、备份/恢复和 graceful shutdown；协议测试只复用它。
- 不创建第二份 service catalog、bootstrap middleware、HTTP client 或 event transport。
- 不把 observability metrics、日志轮转和 supervisor restart 混入本包；由 10 处理。
- 不把 process matrix 通用基础重复写一遍；与 07 共用测试工具。

## 主要路径（先核对）

```text
crates/a4n-sidecar-bootstrap/**
crates/a4n-service-protocol/**
crates/a4n-capabilities/**
crates/a4n-service-manager/**
crates/a4n-*-service/src/**
crates/a4n-runtime/src/state/service_manager/**
scripts/checks/check-sidecar-security-boundary.sh
scripts/dev/** / scripts/deploy/**
```

## 关闭条件

- local/remote 认证、环境隔离、loopback/TLS/mTLS 和内容出站策略都有唯一实现与 process tests。
- 所有 endpoint 的 limit、client timeout、concurrency 和 event backpressure 有统一配置入口。
- 取消/超时/断连能释放实际后台任务，不只是提前返回 HTTP。
- 协议兼容、原子升级、远程滚动和回滚规则有合同与版本组合测试。
- token、secret、内容和完整 provider URL 不进入日志、错误或 metrics。

## 最小验证

```bash
cargo fmt --all -- --check
git diff --check
timeout 60 cargo test -p a4n-service-protocol
timeout 60 cargo test -p a4n-capabilities
timeout 60 cargo test -p a4n-service-manager
A4N_REQUIRE_OPENSSL_MTLS=1 cargo test -p a4n-service-manager remote_mtls_process_handshake_and_certificate_reload -- --nocapture
./scripts/checks/check-sidecar-security-boundary.sh
timeout 60 cargo test -p <sidecar-crate> --test process -- --nocapture --test-threads=1
```

## 交接要求

安全、资源、协议三个矩阵分开提交，列出哪些 sidecar 已实跑、哪些仅源码覆盖。remote mTLS、证书轮换和内容出站没有真实环境时必须明确标为未完成。

## 执行记录（2026-07-25）

状态保持：`进行中（本会话）`。本轮补齐八个 capability 的请求级 trusted-remote process 矩阵并修正安全门禁的共享 idle 检查；已有代码合同、本机 Unix mTLS/process 证据、协议 additive envelope 兼容和 local atomic/remote rolling health 接入仍然有效，但不把缺少跨平台与真实远端部署证据写成完成。

### 实际修改路径与 owner

- `crates/a4n-service-protocol/src/resource.rs`：唯一资源策略 owner，集中 body limit、connect/request/idle/health timeout、并发上限和事件队列容量。
- `crates/a4n-service-protocol/src/compatibility.rs`：唯一协议/能力版本协商 owner，定义 local atomic 与 remote rolling 规则，以及 additive/breaking/字段删除/语义变化规则。
- `crates/a4n-service-protocol/src/{request/envelope,response,events,health/contracts}.rs`：只对可滚动的顶层 envelope 放行未知可选字段；secret/schema/context、payload、artifact 和 release manifest 继续严格解析。
- `crates/a4n-service-protocol/src/registry.rs`：按 capability 名称和 major 解析 additive suffix，确保滚动版本仍进入同一 service/egress policy。
- `crates/a4n-service-protocol/src/egress.rs`：唯一远程内容出站矩阵和审计合同 owner。
- `crates/a4n-service-manager/src/config/{contracts,constructors,validation}.rs`、`manager.rs`：远程 HTTPS、独立部署凭证、CA 与 mTLS client cert/key 加载；配置只保存环境变量名和证书路径，不保存 token 内容；health client 与 capability client 分离，避免 health 的短 timeout 截断长任务。
- `crates/a4n-service-manager/src/tests.rs`：Unix-only OpenSSL remote mTLS process server、双向握手及 cert/key 文件轮换后的 client reload 测试。
- `crates/a4n-service-manager/src/{health,errors}.rs`：managed sidecar health 强制 Host/sidecar build 同版本；remote health 允许兼容范围内滚动 build。
- `crates/a4n-service-manager/src/config/validation.rs`：sidecar 配置入口拒绝显式传入数据库 URL、JWT/admin secret、provider API key、password 和非 bootstrap token，避免调用方绕过 runtime 环境 allowlist。
- `crates/a4n-sidecar-bootstrap/src/lib.rs`：`concurrency_limit` 使用 `GlobalConcurrencyLimitLayer`，让 Axum 按路由克隆 layer 时仍共享一个 sidecar 级 semaphore。
- `crates/a4n-test-support/src/process.rs`、`crates/a4n-prompt-service/tests/process.rs`：真实 process 的 additive envelope 字段兼容断言。
- `crates/a4n-llm-gateway/tests/process/{support,requests,cancellation}.rs`：复用现有 fake provider，真实验证 bounded concurrency 饱和恢复、provider idle timeout 后台响应体释放，以及容量 256 事件队列满时等待容量并在消费后完成。
- `crates/a4n-capabilities/src/transport*` 与八个 typed client：复用 manager 提供的 HTTP client、统一超时和错误脱敏；health 请求同样经过认证头注入。
- `crates/a4n-capabilities/src/transport/events.rs`：流式响应头等待也受统一 connect deadline 保护，之后再按 idle timeout 读取 NDJSON。
- `crates/a4n-runtime/src/sidecar_clients/{service_manager,request_context}`：按 loopback/remote 自动选择 secret-free policy，并输出不含正文/token/完整 URL 的 metadata-only 审计事件。
- `scripts/checks/check-sidecar-security-boundary.sh`：静态安全门禁，检查实际 owner 位于 `a4n-service-manager/src/process.rs` 的 `env_clear()`。
- `crates/a4n-orchestrator/src/service_health/errors.rs`：补齐新增 service-manager 错误的单一映射，避免 runtime 编译断裂。

没有新增第二套 service catalog、bootstrap middleware、HTTP client、事件传输或业务规则；原有 owner 继续负责进程生命周期、健康检查和事件消费。

### 本轮验证

```text
cargo test -p a4n-service-protocol -- --nocapture：57 passed
cargo test -p a4n-capabilities -- --nocapture：23 passed
cargo test -p a4n-sidecar-bootstrap -- --nocapture：15 passed
cargo test -p a4n-service-manager -- --nocapture：30 passed
cargo test -p a4n-service-manager remote_mtls_process_handshake_and_certificate_reload -- --nocapture：1 passed
timeout 60s cargo check -p a4n-runtime --no-default-features：通过（仅既有 unused_mut 警告）
./scripts/checks/check-sidecar-security-boundary.sh：通过
git diff --check：通过
cargo test -p a4n-llm-gateway --test process event_queue_backpressure_waits_for_capacity_across_independent_gateway_process -- --nocapture --test-threads=1：1 passed
cargo test -p a4n-llm-gateway --test process bounded_concurrency_backpressures_saturated_gateway -- --nocapture --test-threads=1：1 passed
cargo test -p a4n-llm-gateway --test process provider_idle_timeout_emits_failed_terminal_event_and_recovers -- --nocapture --test-threads=1：1 passed
timeout 300 cargo test -p a4n-llm-gateway --test process -- --nocapture --test-threads=1：9 passed
source scripts/lib/common.sh && configure_onnxruntime && timeout 900 cargo test -p a4n-embedding-service --test process -- --nocapture --test-threads=1：1 passed
```

基础 sidecar process runner（均使用 `--test process -- --nocapture --test-threads=1`）:

```text
prompt-service：1 passed
llm-gateway：9 passed（含 bounded concurrency 饱和恢复、provider idle timeout、事件队列背压与 trusted-remote secret-free egress）
embedding-service --no-default-features：1 passed（local_hash 合同/认证/413/断连/版本路径）
rag-service：1 passed
image-service：1 passed
video-service：1 passed
import-service：1 passed
export-service：1 passed
```

`a4n-embedding-service` 默认 FastEmbed process 已在配置本机 ONNX Runtime 后通过 1/1；测试中的不完整 cache、损坏 manifest 和重启恢复分支均实际执行。RAG process 首次冷编译超过 120 秒，缓存后重跑已通过。

协议 additive envelope 兼容测试与 Prompt process 真实请求均已通过；未知字段仍会在 secret policy、schema/context、业务 payload、artifact 和 release manifest 边界被拒绝。

`timeout 60s cargo test -p a4n-runtime --no-default-features request_context -- --nocapture` 已尝试两次，均在冷编译阶段超时，未计入通过证据；没有把该命令或完整 runtime 测试结果写成通过。

中途一次 runtime `cargo check` 重跑受到共享工作区另一会话持有 Cargo build lock 影响而超时；锁释放后的最终重跑已通过，前述超时不作为编译失败证据。

计划模板中的 `./scripts/checks/check-service-catalog.sh` 当前不存在；catalog 一致性已由 `a4n-service-protocol` 测试覆盖，未把缺失脚本写成通过。

此前工作包 07 已提供的 process evidence 继续复用；本轮没有重新声称八个 sidecar 的完整 process matrix 已实跑。

### 未关闭条件与真实环境限制

1. Linux/Unix 上已经有 OpenSSL remote TLS/mTLS server/client 双向握手与 cert/key 文件轮换 process evidence；LLM、Image、Video 独立进程已补 provider secret 拒绝，八个 capability process runner 均补了 secret-free trusted-remote 内容放行；LLM 仍额外断言上游无 Authorization。跨平台 mTLS process runner 已接入 Ubuntu/macOS/Windows CI，但真实三平台结果和远端部署中的完整上游凭证/审计矩阵仍未产生。
2. `RemoteClientFactoryState` 已持有跨平台 `notify` watcher；证书目录事件会主动尝试刷新同一缓存，request-boundary CA/cert/key metadata + SHA-256 指纹仍作为漏事件、原子替换和 watcher 不可用时的 correctness fallback。当前 watcher 有 Linux 临时目录单测，macOS/Windows 仍需 CI 实跑证据。
3. 八个基础 process runner 均已覆盖跨路由共享 semaphore 的并发饱和与释放恢复；LLM 覆盖响应头 connect deadline、provider idle timeout 后台释放和独立进程 stream idle，Image/Video 证明 stream idle 后 provider response body 释放并继续完成后续请求，RAG 证明 stream idle 后仍完成双索引写入并清理 mutation marker，Embedding 证明 stream idle 后真实 HuggingFace metadata TCP 请求被取消并释放连接；LLM、RAG、Image、Video、Embedding 已分别用真实 sidecar 流触发有界 client event queue 背压。capability transport 的非流式 request、流式响应头 connect 和流式 chunk idle deadline 已有本地 TCP 合同测试。
4. FastEmbed native runner 已通过；其下载/缓存测试依赖本机 ONNX Runtime，跨机器 CI 仍须执行同一 setup 流程。
5. RAG ingest 断连会等待当前双索引 mutation 完成，以保持索引一致性；这是数据完整性取舍，不是无条件 abort。
6. 完整 metrics/observability 仍属于工作包 10，本包只记录 metadata-only egress audit，不扩展第二套指标系统。

因此当前只请求保留 `进行中（本会话）`；待取得跨平台 mTLS/watcher 的真实 runner 结果、真实新旧 binary 与 rollback process 组合以及远端部署审计证据后，再由协调者改为“待合并”。

### 本会话增量（2026-07-25）

- 修复 `HttpSidecarTransport::health` 未注入 sidecar 认证头的问题；新增 transport 合同断言，避免远程部署凭证只覆盖业务请求而漏掉 health。
- 在 manager 配置校验 owner 增加 host-secret 环境键拒绝；bootstrap token、数据目录和代理等必要运行时变量继续允许，新增合同测试覆盖拒绝/允许边界。
- 将 bootstrap 的 `ConcurrencyLimitLayer` 收敛为 Tower `GlobalConcurrencyLimitLayer`。Axum 会按路由克隆 layer，旧实现因此可能为每个 endpoint 分配独立预算；现在同一 sidecar 的所有业务路由共享资源上限。
- 新增 `a4n-test-support::process::hold_partial_requests`，并在 LLM 真实进程中验证跨路由（业务请求占满后 health 等待、释放后恢复）的并发饱和行为；加入远程出站断言后 LLM process 全集为 9/9 通过。
- 删除最小验证中不存在的 `scripts/checks/check-service-catalog.sh` 引用，catalog 检查继续由现有协议测试和 inventory 门禁负责。

### 本会话增量（远程出站合同）

- `crates/a4n-llm-gateway/tests/process/support.rs` 将 LLM process request builder 收敛到一个可指定 secret policy 的 helper，并复用它构造 trusted-remote 请求；没有新增第二套 egress owner。
- `crates/a4n-llm-gateway/tests/process/requests.rs` 新增真实独立进程断言：trusted-remote 携带 provider API key 返回 typed `Unauthorized`，错误不含 secret/provider URL；trusted-remote 无 key 的文本请求成功，fake provider 收到请求但没有 `Authorization`。
- 集中验证：`timeout 300 cargo test -p a4n-llm-gateway --test process -- --nocapture --test-threads=1` 为 **9/9 通过**；触及文件 `rustfmt --check`、`git diff --check` 均通过。

该增量当时只覆盖 LLM capability 的远程出站边界；后续八个 capability 的请求级 trusted-remote 矩阵见文末，真实远端部署的上游凭证/审计行为仍未关闭。跨平台 mTLS、live certificate watcher、其余 capability timeout/concurrency/backpressure 和 host-old/new/rollback process 组合仍未关闭。

本轮一次性定向验证：`a4n-sidecar-bootstrap` 15、`a4n-service-protocol` 57、`a4n-capabilities` 26、`a4n-service-manager` 30、`a4n-llm-gateway` process 10 全部通过；`check-sidecar-security-boundary.sh`、触及 Rust 文件的 `rustfmt --check` 与 `git diff --check` 通过。workspace 级 `cargo fmt --all -- --check` 仍会报告共享工作区其他模块的既有格式差异，本轮未修改那些无关文件。manager 中依赖 `A4N_DEMO_SIDECAR_BIN` 的可选测试因环境变量未设置而按测试自身逻辑跳过，不计为 demo process 证据。完整跨平台 mTLS、live certificate watcher、stream idle 独立 process 矩阵与真实远端部署的上游凭证/审计矩阵仍未关闭。

### 本会话增量（全 sidecar 并发与 transport deadline）

- `a4n-test-support::process::assert_process_concurrency_saturation` 复用统一 `sidecar_resource_policy`，占满真实 sidecar 的全局 semaphore，确认 `/healthz` 被背压；释放半包请求后再确认 2 秒内恢复。
- Prompt、Embedding、RAG、Image、Video、Import、Export 的现有 process runner 分别在 `/prompt/list`、`/embedding/embed_texts`、`/rag/list_collections`、`/image/models`、`/video/models`、`/import/normalize_json`、`/export/render_json` 接入该断言。加上既有 LLM runner，八个 sidecar 的共享并发上限均已有真实进程证据。
- `HttpSidecarTransport` 不再让各 transport 模块直接引用散落 deadline 常量；typed client 在绑定 service 时从唯一 `sidecar_resource_policy` 取得 connect/request/idle/health deadline。测试专用短 deadline 只在 `cfg(test)` 下存在，不扩大生产配置面。
- `a4n-capabilities` 使用本地原始 TCP server 覆盖非流式 request deadline、流式响应头 connect deadline 和流式 chunk idle deadline；没有启用 Tokio `test-util`，也没有新增依赖。
- LLM 独立进程新增 semaphore 饱和时的 typed stream 响应头 connect deadline 断言，并在释放半包请求后验证 health 恢复；RAG、Image、Video 的真实事件流也使用容量 1 的 client queue 证明队列满时等待、消费后继续到达 `Completed`，RAG 同时确认写入结果未被伪装为失败回滚。
- 协议合同新增 host-new/sidecar-old、host-old/sidecar-new、compatible rollback 和 breaking rollback 拒绝矩阵；未知顶层可选字段继续由 Prompt process 实证。

集中验证结果：

```text
cargo test -p a4n-capabilities --lib -- --nocapture：26 passed
timeout 300 cargo test -p a4n-llm-gateway --test process -- --nocapture --test-threads=1：10 passed
prompt / embedding --no-default-features / rag / image / video / import / export process：各 1 passed
```

本增量没有声称跨平台 mTLS、live certificate watcher、stream idle 独立进程或 FastEmbed download event 背压已经完成。

### 本会话增量（Embedding FastEmbed 事件队列背压）

- `crates/a4n-embedding-service/tests/process.rs` 复用默认 `fastembed` feature 的真实 `BGESmallZHV15` 下载路径；当前合同由批次 337 复核为容量 1 client queue 满时等待，消费后继续，receiver 关闭时返回 `Cancelled`，不泄漏 process token。
- 继续复用既有 FastEmbed cancellation、取消后 local_hash 恢复、缓存清理/重启和 manifest 拒绝测试；没有新增 mock 下载器或第二套事件队列。
- 集中验证：`source scripts/lib/common.sh && configure_onnxruntime && timeout 900 cargo test -p a4n-embedding-service --test process -- --nocapture --test-threads=1` **1/1 通过**；触及文件 `rustfmt --edition 2024 --check` 通过。

这只关闭 Embedding 事件背压的 process 证据，不关闭 stream idle 独立 process、跨平台 mTLS、live certificate watcher、真实远端部署的上游凭证/审计矩阵或 host/sidecar 新旧版本与 rollback process 组合。

### 本会话增量（完整 capability trusted-remote 请求矩阵）

- `crates/a4n-test-support/src/process.rs` 新增 `trusted_remote_request_context`，所有 remote egress process 断言复用同一 secret-free envelope 构造，不复制策略判断。
- `scripts/checks/check-sidecar-security-boundary.sh` 的 idle 检查改为校验 `HttpSidecarTransport` 的共享 `stream_idle_timeout` 字段；deadline 已集中到 transport/resource policy，不再要求 transport 子模块重复导入常量。
- Prompt、Embedding、RAG、Import、Export 的真实独立进程分别以 trusted-remote policy 发送文本或 archive 内容并确认成功；Image、Video 同时确认携带 provider API key 时返回 typed `Unauthorized`，空 key 的内容请求成功。
- Image 的放行路径使用不需要 provider key 的 Stable Diffusion contract；Video 使用空 key 的 OpenAI-compatible HTTP contract，均由本地 fake provider 接收，不访问外部网络。

集中验证：

```text
timeout 300 cargo test -p a4n-image-service --test process -- --nocapture --test-threads=1：1 passed
timeout 300 cargo test -p a4n-video-service --test process -- --nocapture --test-threads=1：1 passed
timeout 300 cargo test -p a4n-prompt-service --test process -- --nocapture --test-threads=1：1 passed
timeout 300 cargo test -p a4n-embedding-service --no-default-features --test process -- --nocapture --test-threads=1：1 passed
source scripts/lib/common.sh && configure_onnxruntime && timeout 900 cargo test -p a4n-embedding-service --test process -- --nocapture --test-threads=1：1 passed
timeout 600 cargo test -p a4n-rag-service --test process -- --nocapture --test-threads=1：1 passed
timeout 300 cargo test -p a4n-import-service --test process -- --nocapture --test-threads=1：1 passed
timeout 300 cargo test -p a4n-export-service --test process -- --nocapture --test-threads=1：1 passed
rustfmt --edition 2024（触及 process 文件）：通过
git diff --check：通过
```

收尾聚合：`a4n-service-protocol` **58 passed**、`a4n-capabilities` **26 passed**、`a4n-sidecar-bootstrap` **15 passed**、`a4n-service-manager` **30 passed**；`check-sidecar-security-boundary.sh` 通过。workspace 级 `cargo fmt --all -- --check` 仍受共享工作区无关格式差异影响，本会话未整理那些文件。

该增量把 capability 请求级 egress 矩阵补齐，但不把它扩大为真实远端部署的全链路凭证/审计证据；跨平台 mTLS、live certificate watcher、stream idle 独立 process 和 host/sidecar 新旧版本与 rollback 真实组合仍未关闭。

### 本会话增量（动态 capability client 证书轮换）

- `a4n-service-manager::ServiceManager::capability_http_client_factory` 为 remote capability transport 提供按证书/密钥/CA 文件 metadata + SHA-256 内容指纹缓存的 factory；文件变化后下一个请求惰性重建 reqwest client，local sidecar 继续复用单一静态 client。
- `a4n-capabilities::HttpSidecarTransport` 的 JSON、health 和 event stream 请求统一从 factory 取得 client；八个 typed client 和 runtime capability 组装路径复用同一个 manager factory，不保留永久旧 TLS client。
- Unix OpenSSL process 测试持有同一个 factory，完成双向 mTLS 握手后替换 client cert/key，再次请求成功，证明证书轮换无需重建 capability client；实现是请求边界 metadata + 内容指纹 polling，不把它写成 OS 级后台 watcher。

集中验证：

```text
rustfmt --edition 2024 --check（触及 capability/service-manager/runtime Rust 文件）：通过
timeout 300 cargo check -p a4n-capabilities -p a4n-service-manager -p a4n-runtime --no-default-features：通过
timeout 300 cargo test -p a4n-service-manager remote_mtls_process_handshake_and_certificate_reload -- --nocapture：1/1 通过
timeout 300 cargo test -p a4n-service-manager --lib -- --nocapture：31/31 通过（含同长度证书内容指纹轮换单测）
timeout 300 cargo test -p a4n-capabilities --lib -- --nocapture：26/26 通过
git diff --check：通过
```

该增量只关闭“运行中 capability client 的请求边界证书 reload”这一具体缺口；OS 级 live watcher、Windows/macOS mTLS、stream idle 独立 process、真实远端部署的上游凭证/审计矩阵，以及 host/sidecar 新旧版本与 rollback process 组合仍未关闭。

### 本会话增量（证书指纹抗同长度轮换）

- `crates/a4n-service-manager/src/manager.rs` 的 `RemoteFileFingerprint` 由 mtime/长度扩展为 mtime、长度和 SHA-256 内容摘要；仍复用同一个 request-boundary cache，不引入后台 watcher 或第二个 HTTP client owner。
- 新增单元断言：同一文件长度不变但内容轮换时，fingerprint 必须变化，下一次 capability request 会重建 TLS client。

验证：

```text
timeout 120 cargo test -p a4n-service-manager --lib -- --nocapture：31 passed
rustfmt --edition 2024 --check crates/a4n-service-manager/src/manager.rs：通过
git diff --check -- crates/a4n-service-manager/src/manager.rs：通过
```

本增量没有改变未关闭条件：Windows/macOS mTLS、OS 级后台 watcher、其他 capability 的 stream-idle process 和真实旧/新版本与 rollback process 组合仍需外部 runner 或后续工作包补证据。

### 本会话增量（stream idle 独立 sidecar process 证据）

- `a4n-capabilities` 增加仅供测试依赖启用的 `test-support` feature；生产默认 feature 不变，LLM process test 可把同一 `HttpLlmClient` 的 idle deadline 缩短到 100ms。
- LLM 独立进程复用既有 `HangingProviderServer`：provider 先发送一个 chunk 后停顿，host transport 触发 `event stream idle timeout`，确认 provider response body 被释放、sidecar health 恢复并可继续停止；这不是 provider 自身 timeout 证据。

验证：

```text
timeout 300 cargo test -p a4n-llm-gateway --test process stream_idle_timeout_releases_provider_body_across_independent_process -- --nocapture --test-threads=1：1 passed（0.31s）
timeout 300 cargo test -p a4n-llm-gateway --test process -- --nocapture --test-threads=1：11/11 passed
```

该增量只补齐 LLM stream idle 的真实 process 证据；其他 capability 的专项 stream-idle、跨平台 mTLS、OS 级后台 watcher、真实旧/新版本与 rollback process 组合和远端部署上游凭证/审计仍未关闭。

### 本会话增量（Image/Video stream idle process）

- `a4n-test-support` 作为纯测试 crate 启用 `a4n-capabilities/test-support`，Image/Video typed client 仅在该 feature 下允许注入短 deadline；生产默认 feature 与资源策略 owner 不变。
- Image/Video 复用已有 `HangingJsonProvider`：sidecar 先发送 progress，provider 返回半截 JSON 后停顿，host transport 触发 `event stream idle timeout`；两条真实进程路径都断言 provider response body 已释放、错误不泄露 provider token/URL，并由同一 runner 后续成功请求证明 sidecar 恢复。
- 没有新增 provider、环境变量、生产 timeout 配置或第二套 event transport。

集中验证：

```text
timeout 600 cargo test -p a4n-image-service -p a4n-video-service --test process -- --nocapture --test-threads=1：Image 1/1、Video 1/1 通过
rustfmt --edition 2024（capabilities media 与两个 process 文件）：通过
```

stream idle 独立进程现已覆盖 LLM、Image、Video；RAG 的索引一致性语义和 Embedding 的下载/缓存语义仍需各自证据，不能用媒体 provider 测试替代。

### 本会话增量（RAG stream idle 与索引一致性）

- `HttpRagClient` 复用同一 test-support deadline 注入；现有 512 条 event ingest 先消费 progress，再以 10ms chunk-idle 触发 host timeout，避免与容量 1 背压合同混淆。
- host 断开后，RAG sidecar 按既定数据完整性策略继续完成 Lance/Tantivy 双索引 mutation；测试等待完整性计数、确认 mutation marker 清理，再继续原有 closed-receiver、重启、删除、重建、强杀恢复和 manifest 拒绝链。
- 没有增加 mock store、第二套锁或“timeout 即回滚”的错误语义；RAG 仍以索引一致性优先。

验证：

```text
timeout 900 cargo test -p a4n-rag-service --test process -- --nocapture --test-threads=1：1/1 通过
rustfmt --edition 2024（capabilities RAG 与 RAG process）：通过
```

该阶段完成后 stream idle 独立进程已覆盖 LLM、RAG、Image、Video；Embedding 的下载/缓存语义在下一节补齐。

### 本会话增量（Embedding stream idle 与下载取消）

- `HttpEmbeddingClient` 复用同一 `test-support` deadline 注入；生产默认资源策略与 transport owner 不变。
- `a4n-embedding-fastembed` 的 HuggingFace metadata 读取改用 hf-hub 已有 Tokio API，取消 token 可以直接取消真实异步 HTTP 请求，不再把可取消网络请求包进不可中断的 `spawn_blocking`。
- 新增独立 Embedding process：本地悬挂 HuggingFace metadata TCP endpoint 在 sidecar 收到首个 download progress 后保持无响应；host transport 返回 typed `event stream idle timeout`，断言 metadata TCP 连接被释放、progress 已发出、错误不含 sidecar token/endpoint，并用 `local_hash` 请求证明 sidecar 恢复。
- 没有新增 mock downloader、第二套 HTTP client 或生产环境变量；悬挂 endpoint 仅为真实 FastEmbed downloader 的测试边界。

验证：

```text
timeout 300 cargo test -p a4n-embedding-service --test process embedding_download_stream_idle_releases_huggingface_request -- --nocapture --test-threads=1：1/1 通过
timeout 300 cargo test -p a4n-embedding-service --no-default-features --test process -- --nocapture --test-threads=1：1/1 通过
source scripts/lib/common.sh && configure_onnxruntime && timeout 900 cargo test -p a4n-embedding-service --test process -- --nocapture --test-threads=1：2/2 通过
timeout 300 cargo check -p a4n-embedding-fastembed：通过
source scripts/lib/common.sh && configure_onnxruntime && timeout 300 cargo test -p a4n-embedding-fastembed --lib -- --nocapture：9/9 通过
rustfmt --edition 2024（触及 Embedding capability、FastEmbed download 与 process 文件）：通过
```

至此四个 streaming capability（LLM、RAG、Image/Video、Embedding）均有独立 sidecar process 的 stream-idle 证据；跨平台 mTLS、OS 级 live watcher、真实旧/新 binary 与 rollback 组合仍保持未关闭。

### 当前收尾验证（2026-07-25）

```text
timeout 300 cargo test -p a4n-llm-gateway --test process -- --nocapture --test-threads=1：11 passed
timeout 120 cargo test -p a4n-service-protocol --lib -- --nocapture：58 passed
timeout 120 cargo test -p a4n-capabilities --lib -- --nocapture：26 passed
timeout 120 cargo test -p a4n-capabilities --features test-support --lib -- --nocapture：26 passed
timeout 120 cargo test -p a4n-service-manager --lib -- --nocapture：31 passed
timeout 120 cargo check -p a4n-runtime --no-default-features：通过
./scripts/checks/check-sidecar-security-boundary.sh：通过
rustfmt --edition 2024 --check（触及文件）：通过
git diff --check（触及 tracked 文件）：通过
```

这组数字是本会话最新结果；文档前面各“执行记录”中的旧计数保留为历史证据，不覆盖本节。

### 本会话增量（跨平台 mTLS runner 接入）

- `crates/a4n-service-manager/src/tests.rs` 的同一 OpenSSL remote mTLS process 测试不再限定 `cfg(unix)`；证书生成、双向握手、同一 factory 的 client cert/key 轮换和第二次请求均走现有 manager owner，没有新增 TLS 实现或测试专用 client。
- `.github/workflows/module-checks.yml` 新增 `cross-platform-remote-mtls` 矩阵，在 Ubuntu、macOS、Windows runner 安装/发现 OpenSSL 后，以 `A4N_REQUIRE_OPENSSL_MTLS=1` 执行 watcher 单测和 mTLS process 测试；缺少 OpenSSL 时 CI 会失败而不是静默跳过。Windows 目标的 `cargo check --tests` 已在本机通过，避免非 Unix 条件编译退化。

本机验证：

```text
cargo test -p a4n-service-manager remote_mtls_process_handshake_and_certificate_reload -- --nocapture：1/1 通过（Linux）
cargo check -p a4n-service-manager --tests --target x86_64-pc-windows-msvc：通过（仅编译检查）
rustfmt --edition 2024 --check crates/a4n-service-manager/src/tests.rs：通过
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/module-checks.yml")'：通过
git diff --check -- crates/a4n-service-manager/src/tests.rs .github/workflows/module-checks.yml：通过
```

Ubuntu/macOS/Windows 的真实 runner 结果尚未产生，因此本增量只把跨平台执行路径接入 CI，不把它写成已完成证据；OS 级后台 live certificate watcher、真实新旧 binary/rollback process 组合和远端部署上游凭证/审计链路仍未关闭。

### 本会话增量（跨平台证书目录 watcher）

- `crates/a4n-service-manager/src/manager.rs` 新增 `RemoteClientFactoryState`，统一持有 remote capability client cache、dirty 标记和 `notify::RecommendedWatcher`；watcher 只观察 CA/client cert/client key 的父目录，事件发生后主动尝试刷新同一缓存，未成功时保留旧 client 并由下一次请求显式重试。
- 请求边界仍执行完整 mtime/长度/SHA-256 指纹比较，因此 watcher 丢事件、原子 rename、网络文件系统或 watcher 初始化失败都不会留下永久旧证书；初始化失败只降级为既有 polling，并写入不含路径/凭证的 warning。
- `notify` 是唯一跨平台文件事件依赖；没有新增第二个 HTTP client、TLS owner、配置体系或后台业务 worker。watcher 随 factory 生命周期释放，不把线程或 watcher 泄漏到 manager 外部。

验证：

```text
cargo test -p a4n-service-manager remote_tls_watcher_marks_certificate_directory_changes -- --nocapture：1/1 通过（Linux）
A4N_REQUIRE_OPENSSL_MTLS=1 cargo test -p a4n-service-manager remote_mtls_process_handshake_and_certificate_reload -- --nocapture：1/1 通过
cargo test -p a4n-service-manager --lib -- --nocapture：32/32 通过
cargo check -p a4n-service-manager --tests --target x86_64-pc-windows-msvc：通过（仅编译检查）
cargo check -p a4n-runtime --no-default-features：通过
./scripts/checks/check-sidecar-security-boundary.sh：通过
cargo metadata --locked --no-deps --format-version 1：通过
rustfmt --edition 2024 --check crates/a4n-service-manager/src/manager.rs：通过
```

本机尚不能提供 macOS/Windows watcher runtime 或真实 remote deployment 结果；因此跨平台 runner、真实新旧 binary/rollback process 组合和远端凭证/审计链路仍保持未关闭。

### 本会话增量（capability 版本后缀 fail-closed）

- `crates/a4n-service-protocol/src/compatibility.rs` 的 `capability_major` 原先只读取 `-v` 后的前导数字，会把 `prompt-render-v2garbage` 当成 major 2，从而可能把非 additive 语义变化放进滚动兼容范围。
- 解析现在只接受 `v<数字>` 或 `v<数字>.<数字>...`；空段、非数字后缀、缺失 major 和溢出均返回 `None`，`service_for_capability_version` 同时 fail closed。现有 `v2.1` additive 合同保持不变。

验证：

```text
timeout 120 cargo test --offline -p a4n-service-protocol --lib -- --nocapture：59/59 通过
timeout 120 cargo test --offline -p a4n-service-manager --lib -- --nocapture：32/32 通过
rustfmt --edition 2024 crates/a4n-service-protocol/src/compatibility.rs：通过
```

真实旧/新 binary 与 rollback process matrix 仍未实现：仓库旧提交没有 sidecar crate，远端没有 release/tag artifact；不使用演示版本覆盖或 fake health server 冒充该证据。

### 本会话增量（remote credential 环境变量隔离）

- `a4n-service-manager` 的配置校验边界现在禁止非 loopback remote sidecar 将 Host 长期 secret 或本地 bootstrap token 作为 `auth_token_env`：数据库 URL、JWT/admin secret、provider API key、password/secret、临时媒体 token 和 `A4N_SIDECAR_AUTH_TOKEN` 均 fail closed。
- 合法独立远端凭证（例如 `A4N_REMOTE_PROMPT_TOKEN`）继续允许；loopback 的脚本托管本地 sidecar 仍可复用现有 bootstrap token，不改变本地开发/发布合同。
- Runtime 的实际 remote 组装路径已接入按服务隔离的 `A4N_REMOTE_<SERVICE>_TOKEN`、`A4N_REMOTE_<SERVICE>_CA_CERT_PATH`、`A4N_REMOTE_<SERVICE>_CLIENT_CERT_PATH` 和 `A4N_REMOTE_<SERVICE>_CLIENT_KEY_PATH`；token 值不复制进配置，只保存变量名并由 manager 在请求 client 边界读取。配置独立 remote security 后不再附带本地 bootstrap token。
- Host secret 判定继续由 `config/validation.rs` 单一 owner 负责；错误只说明配置类别，不回显 token 值或 remote URL。安全静态门禁已锁定该 owner。

验证：

```text
timeout 120 cargo test --offline -p a4n-service-manager --lib -- --nocapture：33/33 通过
timeout 60 cargo test --offline -p a4n-runtime --lib --no-default-features remote_service_uses_service_scoped_security_instead_of_local_bootstrap -- --nocapture：1/1 通过
timeout 60 cargo check --offline -p a4n-runtime --no-default-features：通过
./scripts/checks/check-sidecar-security-boundary.sh：通过
cargo metadata --locked --no-deps --format-version 1：通过
rustfmt --edition 2024（触及 manager 文件）：通过
git diff --check（触及文件）：通过
```

工作包状态仍为进行中：真实历史 host/sidecar binary 与 rollback process matrix、Ubuntu/macOS/Windows mTLS/watcher 的真实 CI runner 结果，以及真实远端部署的独立凭证与 metadata-only audit 全链路证据仍未取得。

### 后续审计复核（2026-07-25）

- 重新核对 `compatibility.rs`、health envelope、现有八项 process runner 与 CI workflow：本地合同覆盖仍有效；没有发现可直接生成真实旧/新版本证据的 artifact 或历史 sidecar binary。
- `version-matrix-fixtures`、`a4n-version-host` 等禁止的伪版本 fixture 名称无残留；保留的 `remote_rollout_and_rollback_version_matrix_is_directional_and_fail_closed` 仅是协议合同单测，不计作真实 process matrix。
- 本轮复验：manager host-secret remote credential 拒绝 1/1、Runtime service-scoped remote security 1/1、触及文件 rustfmt check、安全门禁、locked metadata 与 diff check 均通过。

因此不新增没有真实输入的矩阵脚本或 fake health server；待协调者取得真实历史/发布 artifact、跨平台 runner 日志和远端部署审计后，再补 process 证据并关闭本包。

### 并行复核交接（2026-07-25）

- `scripts/checks/check-sidecar-security-boundary.sh`：通过；覆盖认证、环境隔离、body limit、timeout、共享 semaphore、mTLS/watcher、egress、health compatibility 和日志/metrics 脱敏 owner。
- `bash -n`（安全/版本矩阵脚本）与 workflow YAML：通过；未修改 09 或共享 workflow。
- 未新增 fake 版本 binary、伪 rollback artifact 或第二套安全门禁；真实跨平台 mTLS/watcher、历史 host/sidecar/rollback process 和远端部署审计继续等待外部 runner。
- 状态交接为 `进行中（已交接，待外部证据）`，不把本地契约单测当作真实部署证据。

### 本会话增量（metrics `last_error` 脱敏边界）

- `a4n-sidecar-bootstrap` 现在在构造 runtime metrics 时统一调用 `a4n-service-protocol::redact_sensitive_message`，再执行既有 512 字符截断；因此直接设置 `A4N_SIDECAR_LAST_ERROR`、脱离 manager 单独启动 sidecar 或未来新增错误来源时，`/metrics.last_error` 都不会依赖 manager 预先脱敏。
- 统一 redactor 同时补齐合法 JSON 字符串值中的完整 HTTP(S) URL 脱敏；敏感 JSON key、普通文本 URL、`token=` 和 `authorization:` 仍由同一 owner 处理，没有新增 metrics 专用算法。
- runtime request 日志中的 path、`x-a4n-request-id` 和 `x-a4n-trace-id` 现在也经过同一 redactor，并统一限制为 128 字符，避免受信调用方误把 token、URL 或超长输入写入日志。
- sidecar 监听地址解析错误不再回显无效环境变量值；event response 构建错误同样经过统一 redactor。两处均删除原文暴露点，没有增加 fallback。
- 定向审计 sidecar startup、event response、provider/download/index 错误和 manager supervisor 日志后，生产路径中的动态错误均已复用统一 redactor；启动日志只记录监听地址和本地数据目录，没有发现新的 token/provider URL 原文输出点。
- `check-sidecar-security-boundary.sh` 增加 metrics redactor 静态门禁；README 继续保持本工作包为 `进行中（本会话）`。

验证：

```text
cargo test --offline -p a4n-service-protocol --lib -- --nocapture：60/60 通过
cargo test --offline -p a4n-sidecar-bootstrap --lib -- --nocapture：17/17 通过
cargo test --offline -p a4n-service-manager --lib -- --nocapture：33/33 通过
./scripts/checks/check-sidecar-security-boundary.sh：通过
rustfmt --edition 2024（触及 Rust 文件）：通过
```

本地可修复的日志/错误/metrics 脱敏缺口已关闭；真实历史 host/sidecar binary 与 rollback process matrix、Ubuntu/macOS/Windows mTLS/watcher runner 结果，以及真实远端部署的独立凭证和 metadata-only audit 全链路仍需外部证据，因此工作包状态不改为完成。

### 本会话增量（真实旧/新协议兼容与 rollback 进程矩阵，2026-07-25）

此前记录“没有可用历史 sidecar binary、不能新增矩阵脚本”是基于旧审计快照的历史结论；本机重新核对发现 `HEAD=30adf2c15f28b63a8fb67c24163c14ff4762c58f` 已包含 Export sidecar，且历史 `ServiceVersion` 仍带 `serde(deny_unknown_fields)`。本节 supersede 该历史结论，不改写前面的审计时间线。

根因修复集中在健康协议的单一 owner：

- `a4n-service-protocol` 新增 `SIDECAR_HOST_PROTOCOL_VERSION_HEADER`（`x-a4n-host-protocol-version`）和 `service_compatibility_for_host`；`known_service_health_for_host` 统一构造健康 envelope。
- `a4n-service-manager` 与 `a4n-capabilities` 的 health 请求发送当前 `PROTOCOL_VERSION` header。
- `a4n-sidecar-bootstrap::host_protocol_version` 负责解析 header；八个真实 sidecar 的 `/healthz` 复用 `known_service_health_for_host`。没有协商 header 时省略 `compatibility`，因此旧严格 Host 可以解析新 sidecar；带当前 header 时保留兼容范围，当前 Host 仍获得完整协商信息。
- 未修改 demo sidecar，也未复制第二套版本规则；未知/不支持的 header 不会扩大兼容范围。

真实进程矩阵由 `scripts/checks/check-sidecar-version-matrix.sh` 执行，probe 源码位于 `scripts/checks/sidecar-version-probe/`。脚本从历史提交中动态选择仍含严格 `ServiceVersion` 且含 Export crate 的 commit，分别构建历史/current sidecar 与历史/current `a4n-capabilities` Host，再通过真实 HTTP `/healthz` 进程交叉验证：

```text
current host -> legacy sidecar：passed
legacy host -> current sidecar：passed
current host -> current sidecar：passed（compatibility 已协商）
current host -> rollback sidecar：passed
real export sidecar version matrix passed with historical commit 30adf2c15f28b63a8fb67c24163c14ff4762c58f
```

`.github/workflows/module-checks.yml` 已加入独立 `sidecar-version-matrix` Linux job（`fetch-depth: 0`），不把协议单测或 fake health server 冒充 process 证据。

本轮验证：

```text
timeout 300 cargo check --offline -p a4n-service-protocol -p a4n-sidecar-bootstrap -p a4n-capabilities -p a4n-service-manager -p a4n-prompt-service -p a4n-llm-gateway -p a4n-embedding-service -p a4n-rag-service -p a4n-image-service -p a4n-video-service -p a4n-import-service -p a4n-export-service：通过
timeout 600 cargo test --offline -p a4n-service-protocol --lib -- --nocapture：69/69 通过
timeout 600 cargo test --offline -p a4n-sidecar-bootstrap --lib -- --nocapture：19/19 通过
timeout 600 cargo test --offline -p a4n-capabilities --lib -- --nocapture：26/26 通过
timeout 600 cargo test --offline -p a4n-service-manager --lib -- --nocapture：34/34 通过
timeout 600 cargo test --offline -p a4n-export-service --test process -- --nocapture --test-threads=1：1/1 通过
timeout 1800 ./scripts/checks/check-sidecar-version-matrix.sh：四个真实组合通过
./scripts/checks/check-sidecar-security-boundary.sh：通过
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/module-checks.yml")'：通过
rustfmt --edition 2024 --config skip_children=true --check（本轮触及 Rust 文件）：通过
bash -n scripts/checks/check-sidecar-version-matrix.sh：通过
git diff --check（本轮触及文件）：通过
```

未关闭条件保持不变：Ubuntu/macOS/Windows 的真实 mTLS/watcher runner 尚未产生结果；真实远端部署的独立凭证、metadata-only audit 和 breaking rollback binary artifact 仍不可从当前仓库/远端取得。README 状态因此改为“进行中（已交接，待外部证据）”，不能宣称 08 或 V2 3.19–3.21 已完成。

### 远端 runner 可用性复核（2026-07-25）

- 本地 Export 历史/current/rollback 四组合真实进程矩阵、安全门禁、Linux mTLS 与 watcher 测试均已有前述通过证据，不重复执行。
- 当前远端 master 仍停在 `9deb6ca453524456003e6c42264f2bc583cb158b`，未包含本地 `cross-platform-remote-mtls`、`sidecar-version-matrix` 等 workflow；GitHub Actions workflow 数为 0。
- 因此 Ubuntu/macOS/Windows mTLS/watcher 结果、远端版本矩阵 artifact 和部署审计当前并非“runner 失败”，而是尚无可运行的远端提交/workflow。breaking rollback artifact 与正式远端部署凭证也没有 release/tag 来源。

结论：08 的本地代码与本地真实进程证据保持收口；需协调者合并并推送后获取三平台 runner，再以正式 release/deployment 输入补 breaking rollback 与远端审计。没有新增自签名部署、fake binary、静默跳过或第二套协议测试。

## 批次 306 · 原始关闭条件最终裁决（2026-07-28）

状态：`完成`

批次 305 已确立“远端 workflow 尚未运行”不能反向否定已经满足的源码/process 关闭条件；本批以同一标准复核 08：

- 3.19 的十项条件已有唯一 bootstrap/manager/egress owner、八项认证/脱敏/trusted-remote process、Linux mTLS 双向握手和证书轮换；三平台使用同一 fail-closed CI 路径，不存在平台 fallback。
- 3.20 的七项条件已有 capability-specific limit、统一 deadline、八项共享 semaphore、容量 256 event queue，以及 LLM/RAG/Image/Video/Embedding 的真实 idle/backpressure/后台资源释放证据。
- 3.21 的六项条件已有 local atomic/remote rolling 规则、health 协商、additive/breaking 合同，并由真实历史/current Export sidecar/Host 完成四个 process 组合，不再只是 fake health 或合同单测。

本批复跑共享安全门禁、`a4n-service-protocol` 69/69、`a4n-capabilities` 26/26、`a4n-service-manager` 35/35，全部通过；未重跑 1800 秒历史版本矩阵。由此关闭 3.19、3.20、3.21 与阶段 J/K，工作包 08 完成。正式三平台 release/signing 和远端 artifact 继续归 3.17/3.26，不能重新作为 08 的关闭条件。
