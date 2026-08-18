# Sidecar 响应、事件与请求上下文单一事实源清理记录

## 搜索覆盖范围

本轮以 Sidecar 的请求、响应和生成事件三条生产路径为起点，按 BFS 逐层追踪：

1. L0 入口：Prompt 读写、图片/视频生成、Embedding 模型管理与下载、RAG 写入与查询、项目包导入导出、LLM Sidecar 调用。
2. L1 编排与适配：`a4n-orchestrator::prompt_records`、`a4n-runtime::sidecar_clients` 的 image、video、embedding、rag、package_io、llm 调用者。
3. L2 能力与依赖注入：`A4nCapabilities`、全部 `*Client` trait、HTTP client 实现、Runtime `runtime_capabilities` 装配。
4. L3 传输：`a4n-capabilities` 的 JSON response 反序列化、typed response 转换、事件流解析和 artifact/error 清洗。
5. L4 协议：`ServiceRequest`、`ServiceRequestContext`、`ServiceResponse`、`CapabilityTaskEvent`、状态枚举、Serde 的显式 null/缺失 payload 语义、请求上下文校验。
6. L5 服务端生产者：prompt、image、video、embedding、rag、import、export 服务通过 `ok_response`、`error_response`、`ServiceResponse::from_result` 生成信封的路径。
7. 隐式依赖：Cargo crate 依赖方向、trait 注入实现、Runtime profile 的 Sidecar 能力装配、HTTP 路由返回类型、测试替身实现和协议严格合同。
8. 事件合同专项 BFS：从 `CapabilityTaskEventKind` 的每个枚举值和每个字段出发，横向覆盖 LLM、image、video、embedding、RAG 五类生产者，NDJSON 序列化/反序列化、终态短路、Runtime 各消费者、测试替身、脚本、前端与仓库配置。全仓未发现反射注册、动态事件路由或外部配置生成这些事件种类。
9. Sidecar 认证专项 BFS：从 `external_sidecar_auth_token` 出发，覆盖 `scripts/run.sh`、CLI local start、Runtime profile/config 装配、`ServiceManager::auth_token`、Remote HTTP health/client、managed sidecar 进程环境、Sidecar Bootstrap 认证中间件、发布启动脚本、Git 历史及忽略文件。确认 `data/services/auth-token` 的唯一写入者是已删除的 `scripts/dev/run-services.sh`；当前生产入口只使用环境变量或 Runtime 生成的进程 token。
10. 跨平台发布 smoke 专项 BFS：从 Linux、macOS、Windows 三套 Host + Sidecar 生命周期 smoke 出发，覆盖服务名、二进制名、地址环境变量、URL 环境变量、健康路径、端口分配、PID 数量、发布清单生成与 Tauri externalBin；三平台现均从 catalog 派生服务拓扑。
11. 发布库存专项 BFS：从 `service-catalog.json` 出发，逐层覆盖协议注册表、Runtime profile 配置、源码启动、portable/Tauri 打包、release manifest 生成与校验、CycloneDX SBOM、CI matrix、篡改检查和发布文档。确认剩余分叉包括多份 catalog 二进制投影、可选 catalog 覆盖参数、固定 `8/9` 数量断言和注册表内联服务数组。
12. 发布工具库存与完整性专项 BFS：从随包发布的 `a4n`、`a4n-server`、`a4n-migration` 三个二进制出发，覆盖 Cargo package 到 binary 的映射、portable 构建/复制/安装、release manifest 生成与消费、`A4N_RELEASE_MANIFEST_PATH` 注入链、ServiceManager 启动前完整性校验、Tauri externalBin 准备与动态配置、Linux/macOS/Windows smoke、篡改检查、CycloneDX SBOM、CI step 和发布文档。横向搜索 Shell、Ruby、PowerShell、Rust、Tauri JSON 与 workflow 后，确认 `deploy-cloud.sh` 中对 `a4n-server` 的单服务部署属于明确业务入口，不是三工具库存分叉；未发现反射、动态代理、AOP、DI 容器或其他配置生成工具库存。
13. 本地启动入口专项 BFS：从 `scripts/run.sh` 的每个参数分支出发，覆盖 README、CLI local 生命周期、frontend npm scripts、migration、doctor、Tauri dev、Server、Runtime profile、managed sidecar 环境和全仓脚本调用。确认 `--frontend`、`--migrate`、`--check` 只有脚本内部转发，无文档、CI 或其他调用；短参数、`--embedded`、`--no-reload` 同样无活跃消费者。
14. 数据库启动迁移专项 BFS：从 `run_startup_migrations`、`a4n local start`、`scripts/run.sh` 与 portable/Tauri 发布入口出发，覆盖 Server/Tauri Cargo feature、独立 `a4n-migration`、CI 预迁移、云部署和 Host schema 检查。确认 full Server 原先在 CLI/脚本外置迁移后再次内嵌迁移，而默认 Tauri portable/.app 构建反而只做 schema 检查。
15. 构建入口专项 BFS：从 `scripts/build.sh` 的全部参数、状态变量与执行分支出发，覆盖 `module-checks.yml`、README、CONTRIBUTING、CLI 测试说明、部署缺失产物提示、公共脚本库、Cargo workspace/target 配置和全仓 Shell 调用。确认 `-r/-c/-w` 仅脚本自述，`-p` 仅两个 CI 调用；`EXTRA_FLAGS`、`mode_label` 与成功后的尾部输出无活跃路径，未发现反射、动态命令拼接或外部配置生成这些入口。
16. 自适应 Cargo 构建专项 BFS：从 `scripts/dev/cargo-build-adaptive.sh`、四个 `A4N_CARGO_*` 配置和 OOM 重试日志路径出发，覆盖全仓脚本、CI、README、CONTRIBUTING、AGENTS、重构文档与 Git 历史。该脚本自加入以来没有任何外部调用、文档入口或配置消费者，只有文件内部自引用；生产、CI 和开发构建均使用 `build.sh`、直接 Cargo 或现有 build-feedback 脚本。
17. 本地临时目录专项 BFS：从全仓 `mktemp`、`TMPDIR` 和 `/tmp/` 字面量出发，覆盖合同生成、Qdrant 安装、release manifest fixture、Linux/macOS 发布 smoke、build-feedback 自检、Sidecar 版本矩阵、ONNX 安装和 LTX 脚本。确认前七类本地 fixture/下载可统一归属仓库 `target/tmp`；ONNX 临时目录与最终安装目录同父级以保证原子移动，LTX `/tmp` 字面量属于远端 ComfyUI 主机日志，不是本地代码或构建产物。
18. 构建缓存守卫专项 BFS：从 `guard_build_cache` 出发，覆盖 build、test、run、portable deploy、Tauri bundle、externalBin 准备入口，继续追踪 `guard-build-cache.sh` 的阈值、环境变量、Linux `/proc` 扫描、flock、target-* 删除、incremental 删除、cargo-sweep 和最终 cargo clean。全仓无独立用户入口或文档消费者；当前主 `target` 为 36 GiB，已超过其默认 30 GiB 软阈值，下一次普通入口会自动破坏增量缓存。
19. Rust 测试入口专项 BFS：从 `scripts/test.sh` 的 `--crate`、`--nocapture`、`--release` 和 ONNX 前置条件出发，覆盖 README、AGENTS、CI 直接 Cargo 调用、Cargo feature/dependency 图及 Git 历史。确认 `--release` 仅脚本自述，无活跃调用；测试入口维护的五 crate 正则与构建入口的真实 `cargo tree` 判定为同一业务的两套实现。
20. 云部署入口专项 BFS：从 `deploy-cloud.sh` 的参数、环境变量、Docker 构建目录、部署包缓存、SSH 预检、远端备份替换、`.env` 更新、迁移、重启和 smoke 顺序出发，覆盖 `.env.example`、README、CI、Git 历史及全仓变量引用。确认 `target-noble`、仓库外 cache、Nginx 配置备份、提前写入 `SSH_OPTS[1]` 均只有脚本内部消费者；Nginx 配置从未被该流程修改。
21. 临时媒体与视频 Provider 输入专项 BFS：从 `A4N_VIDEO_IMAGE_INPUT_TRANSPORT`、公网媒体 URL、远端临时上传 URL/token 和 `TempMediaUploadRequest` 出发，逐层覆盖章节/Canvas/参考媒体生成入口、Host Port 实现、图片/视频/音频 MIME 与 base64 校验、Runtime 上传及响应核验、Axum upload/serve/delete 路由、文件存储与过期清理、cleanup outbox 删除、云部署 `.env` 更新与 smoke、进程测试和 API 合同。确认不存在反射、动态代理、AOP、DI 容器或 XML/路由表之外的隐藏注册；Canvas 的“按远端变量存在性自动选模式”、请求 TTL 与服务端固定 TTL、两套公网入口变量和 data URI 兼容解析均是可删除的第二事实源。
22. 跨语言发布库存投影专项 BFS：从 `service-catalog.json`、`release-tools.json` 和 `RELEASE_MANIFEST_SCHEMA_VERSION` 出发，横向覆盖 Ruby manifest 生成/校验、Shell 公共投影、Windows PowerShell 生命周期 smoke、Linux/macOS smoke、Tauri bundle、portable deploy、CI matrix 与 SBOM。确认 Shell 和 PowerShell 原先分别复制了空值、格式、唯一性校验，Windows/macOS workflow 及浏览器截图仍写系统临时目录；这些路径没有反射或外部配置生成器。
23. 公网应用入口专项 BFS：从唯一 `A4N_PUBLIC_APP_URL` 出发，覆盖 core 环境读取、Runtime `public_url` 图片传输、JWT 媒体路由、Server 临时媒体响应 URL、Axum `/media` 与 `/temp-media` 路由、云部署写入、进程 fixture 和 URL 探测错误提示。确认 Server 原先用 `http::Uri`，Runtime 原先用 `reqwest::Url`，对 loopback、unspecified、fragment 和凭证的接受规则不同；无反射、DI 或外部路由生成器参与该地址派生。
24. managed Sidecar 生命周期专项 BFS：从 `AppState.managed_sidecars` 出发，覆盖 `AppState::clone` 的 cleanup worker 与各生成后台任务、Tauri managed state/Exit、Server `Arc<AppState>`/Router/优雅信号、CLI-owned Server 进程、`ManagedSidecarProcess` Supervisor command/Drop/status watcher。确认 `Arc<Mutex<Vec<_>>>` 是跨克隆共享且可重复关闭的必要 owner；真实缺口是 Server 未调用唯一关闭入口，以及监听失败前已提前初始化 Runtime/Sidecar。
25. Image/Video provider request preview BFS：从 `/image/request_preview`、`/video/request_preview` 路由出发，覆盖 provider factory、全部活跃 provider body builder、Capability Client、Runtime profile 分流、进程夹具和上层 `request` 字段装配；确认 Sidecar 与 Embedded 均应直接返回 provider body，旧的单调用信封和上层重复嵌套没有活跃消费者。
26. Image/Video model discovery BFS：从图片、视频、LLM 三个模型探测命令反向覆盖 Runtime config action、Sidecar Capability Client、请求上下文/secret policy、三个 Sidecar 路由、共享 `a4n-model-discovery` HTTP/parser、前端 API 和测试替身；确认网络探测业务只有 `a4n-model-discovery` 一个 owner，Runtime 原先三份 profile/请求/响应分支属于同一实现。
27. Provider request validation BFS：从各 Sidecar provider 请求入口覆盖 envelope service/capability 校验、API key secret policy、Image/Video/Embedding/LLM 的生成、轮询、探测与模型发现路由，并反查协议测试和错误映射；确认“已知服务 + provider secret policy”是统一协议不变量，不含服务特有的尺寸、project scope 或业务参数校验。
28. Provider body builder 横向 BFS：从四类简单 preview alias 追踪到真实生成请求、factory dispatch、provider module visibility 与 provider tests；确认 HTTP Video、New API、Seedance 2.0 和 Stable Diffusion 的 preview 与真实 body 完全相同，删除别名并让 factory 直接调用唯一 body builder，保留 Gemini/ComfyUI/编辑与参考图 preview 的真实差异。
29. Video embedded preview 输入 BFS：从 `VideoProviderRequestInput` 的三类 HostPort 调用反向覆盖 config_state、视频配置校验、provider factory 与所有 provider base URL 构造器；确认 `base_url` 是正式配置必填项，`unwrap_or_default()` 只把缺失配置伪装成空字符串，删除该默认回退并让缺失配置显式失败。
30. Embedding API base URL BFS：从 Runtime Embedding Sidecar payload 与动态 provider factory 出发，覆盖 `EmbeddingBackendKind::Api` 的配置读取、Sidecar contract、HTTP provider 构造、Connectivity 流程和测试夹具；确认 API backend 的 `base_url` 缺失只应在配置边界显式报错，两个空字符串 fallback 没有独立业务语义。
31. OpenAI-compatible Image request validation BFS：从 embedded generate、Sidecar preview、OpenAI edit 无参考图分支和 provider factory 反向覆盖同一 generations 合同；确认参考图拒绝规则完全相同，删除 generate/preview 两份条件分支并保留一个 provider 校验 owner。
32. Embedding required-text BFS：从动态 provider、Sidecar payload、model identity 和本地测试调用反向检查所有必填字符串消费者；确认 `required_text` 应同时承担缺失与空白校验，删除调用者各自制造空字符串的路径。
33. Embedding capability injection BFS：从模型列表、下载信息、删除、连通性测试和下载事件五条 Runtime 路径向 `A4nCapabilities.embedding` 反查；确认所有路径只有同一 Client 缺失错误和同一 Arc 实例来源，删除三份重复 Client 获取逻辑。
34. Embedding 模型缓存与下载专项 BFS：从 `EmbeddingTab` 的列表、大小查询、下载、轮询、取消和删除动作出发，覆盖 generated Host 合同与 command manifest、Server/Tauri 命令入口、任务注册与取消令牌、Runtime model action/queue、`A4nCapabilities.embedding` 注入、HTTP Client 路由、Embedding Sidecar handler/event stream、FastEmbed cache/download/provider、managed service 环境装配、独立进程环境和 Cargo feature/dependency 边界。确认生产路由与缓存环境键没有反射、动态代理、AOP、XML 或外部路由表生成；模型身份、模型摘要、下载信息和 Sidecar DTO 原先存在多套结构镜像及 JSON 中转。
35. FastEmbed 下载代理专项 BFS：从 metadata、文件传输和缓存模型初始化三个 `ScopedHfHubEnv` 调用出发，覆盖显式大小写代理环境、Runtime `env_clear` 后的 managed Sidecar 环境投影、Linux `gsettings`、macOS `scutil`、Windows Internet Settings、HF Hub/reqwest client 构造和平台代理夹具。确认不存在配置文件生成、反射、动态代理或 AOP 注册；原实现同时在 merge 与环境投影两层交叉回退协议，并会用系统 HTTP/HTTPS 覆盖显式 `ALL_PROXY`。
36. 设置页模型探测专项 BFS：从 LLM、图片、视频三个编辑器的“检测”按钮出发，覆盖前端本机/服务器密钥解析、generated Host 合同、command manifest、Server dispatch 与宏、Runtime 管理员已保存密钥读取、embedded/sidecar profile 分流、`A4nCapabilities` 注入、三类 HTTP 路由、Service Protocol envelope/secret policy、三个 Sidecar handler、`a4n-model-discovery` 的端点、鉴权、HTTP 和响应解析，并反查 provider registry、默认 Base URL、环境配置、文档与夹具。确认不存在反射、动态代理、AOP、XML 或外部路由表生成模型探测路径；生产 OpenAI-compatible Base URL 合同均将版本路径视为 base 的一部分，双 URL 重试和多套旧响应形状没有独立活跃场景。
37. 设置页配置连通性专项 BFS：从 LLM、图片、Embedding 的“测试连接”和请求预览入口出发，覆盖前端 server/local scope、个人配置与系统模板、`apiInvoke` 自动快照、Host command manifest、Server dispatch、Runtime snapshot scope、管理员数据库配置、队列、Runtime profile、LLM embedded chat/Sidecar probe、图片 embedded/Sidecar 请求体、Embedding 本机桌面与服务器执行、Capability DI/HTTP、Sidecar 路由、provider 实现和全部 `LlmProvider` trait/test-double 实现。确认未发现反射、动态代理、AOP、XML 或外部路由表生成隐藏调用；LLM `test_connection` 只有实现与测试消费者，已不属于任何活跃生产路径。

全仓横向搜索确认，生产代码中对 `ServiceStatus` 的业务解释原有两套：Runtime 通用解析与 Orchestrator Prompt 解析；图片与视频生成事件也各自维护同构实现。

## 优化记录

1. 在 `a4n-capabilities` 建立唯一 `sidecar_response_payload`，统一解释 `status`、`payload`、`error` 的合法组合。
2. 删除 Runtime 三套重复的 response 状态分派、缺失 payload 分支和“未知错误”回退。
3. 删除 Prompt 专有 response 状态分派，改用同一信封解释 owner；保留并统一“成功带 error / 失败带 payload”非法合同检查。
4. `service_response_payload` 并入 `response_payload`，删除只转发一次的死 Facade。
5. 删除 `service_response_unit`；显式 null 的 unit 合同由 `response_payload::<()>` 唯一处理。
6. 删除只服务于旧 unit 解析器的重复测试模块。
7. 合并图片与视频生成事件的 progress、completed、failed、cancel、忽略事件解析，建立唯一 `parse_generation_event`。
8. 图片和视频模块只保留各自的进度投递方式，不再复制协议字段校验和 completed payload 反序列化。
9. 将 `ServiceErrorCode::Cancelled` 到 `AppError::Cancelled` 的映射收敛到 `service_error_app_error`，删除 RAG 专用别名和 Embedding 内联分支。
10. 将 String 错误路径的取消语义收敛到 `service_error_string`，事件失败和传输失败使用相同取消结果。
11. 请求构建只序列化 payload 一次；删除 Value payload 在通用构建器中的二次 JSON 转换。
12. typed transform 请求直接保留原始 typed payload，仅用一次 JSON 投影检查项目作用域，删除 typed→Value→typed 的逐字段重建。
13. Runtime 请求构建复用协议层 `ServiceRequestContext::with_payload`，删除本地重复的 `ServiceRequest` 装配和上下文校验调用。
14. Prompt 请求构建同样复用 `ServiceRequestContext`，删除第二套 schema、secret policy、字段装配与校验实现。
15. 删除仅供旧测试路径使用的 `pure_transform_service_request`，现有静态测试路径直接覆盖生产使用的 typed builder。
16. 删除 `SidecarRequestContext.schema_version` 死配置；该值从未可变，协议层 `ServiceRequestContext` 现在唯一提供 protocol-only schema。
17. 请求 ID 统一使用 UUID，删除纳秒时间戳及其微秒回退分支，并与 Prompt 请求 ID 生成方式一致。
18. 删除未知 capability version 直接沿用现有 egress policy 的放行分支；生产请求必须命中协议注册表。
19. 删除 LLM trace 的 `unknown` 和缺省 `call` 占位符；空 workflow、step key 或显式空 call key 现在直接失败。
20. RAG 已完成递归作用域扫描后的 Value payload 直接进入 Value builder，删除第二次 JSON 转换。
21. LLM 完整响应收集改为一个 select/drain 循环，删除传输结束前后两段完全相同的事件聚合。
22. LLM `CancelAck` 在完整响应和流式响应中统一映射为 `AppError::Cancelled`，不再被忽略或降级为普通流错误。
23. 建立唯一 `parse_sidecar_llm_event`，统一 Chunk、Usage、Completed、Failed、CancelAck 和忽略事件的语义。
24. LLM Chunk 只接受网关生产的 typed payload，删除缺少 payload 时由零索引和零散字段重建 `ChatChunk` 的兼容回退。
25. 完整响应路径现在要求真实 `Completed` 事件，并读取 `final_usage`；不再仅凭先前 chunk 的 finish reason 将提前关闭判为成功。
26. LLM 完整响应与流式响应统一使用协议事件队列容量，删除本地硬编码 `64`。
27. LLM Sidecar request 的 stream usage、payload、trace context、secret policy 和错误映射收敛到唯一构建函数。
28. `CapabilityTaskEventKind` 从九类收敛为生产环境实际使用的 `Progress`、`Chunk`、`Completed`、`Failed` 四类。
29. 删除无生产者的 `StructuredProgress` 事件及 Runtime 的兼容分派；结构化进度仍由 Host generation 领域事件负责，不再占用 Sidecar 传输合同。
30. 删除无生产者的独立 `Usage` 事件及 LLM Runtime 解析分支；用量只随 typed `ChatChunk` 传输。
31. 删除无生产者的 `Checkpoint` 事件和 `checkpoint_key` 死字段；Sidecar 传输层不再伪装 Host 工作流断点能力。
32. 删除无生产者、无消费者的 `Artifact` 事件和事件级 `artifact_refs`；artifact 安全校验继续由实际使用该字段的 `ServiceResponse` 唯一承担。
33. 删除从未生产的 `CancelAck` 第二套取消表达；所有 Sidecar 取消统一为 `Failed + ServiceErrorCode::Cancelled`。
34. 删除 `chunk_index` 重复字段；索引只存在于 typed `ChatChunk.index`。
35. 删除 `chunk_delta` 重复字段；普通文本、推理文本和工具调用只由完整 `ChatChunk` payload 表达。
36. 删除 `usage_delta` 重复字段；用量只存在于 `ChatChunk.usage`。
37. 删除 Completed 上重复的 `final_usage`；最终用量来自已发送的真实 chunk，不再二次复制。
38. 删除事件信封上的重复 `finish_reason`；结束原因只存在于 `ChatChunk.finish_reason`。
39. 删除可由 finish reason 直接判定的 `is_truncated_finish_reason` 派生字段，避免两份状态不一致。
40. LLM 网关 Chunk 统一调用协议层 typed `CapabilityTaskEvent::chunk` 发送完整 `ChatChunk`，删除拆字段投影和专用 usage JSON 序列化函数。
41. LLM Completed 收敛为纯终态标记，删除网关流状态中仅为重复 Completed 数据服务的 usage/finish 缓存。
42. LLM Runtime 终态只确认真实 Completed；usage 和 finish reason 均由同一批 Chunk 聚合，删除 Completed 二次覆盖路径。
43. LLM Failed 事件缺少 error 时直接报告非法合同，删除“未知失败事件”兜底错误。
44. `CapabilityTaskEvent` 启用 `deny_unknown_fields`，退役字段和旧事件格式不再被静默忽略。
45. 测试替身与协议样例统一使用事件构造器和 typed payload，删除大量手写空字段及 CancelAck 旧格式样例。
46. 图片 Sidecar 删除零逻辑 `failed_generation_event`，失败事件直接由协议层 `CapabilityTaskEvent::failed` 唯一构造。
47. 视频 Sidecar 删除第二套同名失败事件包装，生成入口与异步执行路径统一调用协议构造器。
48. RAG Sidecar 删除 `failed_ingest_event` 转发层，校验失败、序列化失败和写入失败共用协议构造器。
49. Embedding Sidecar 删除 `failed_download_event` 转发层，下载失败与进度序列化失败不再经过本地别名。
50. LLM Gateway 删除 `failed_event` 转发层，连接、流读取、Chunk 序列化和入口校验失败统一由协议构造器表达。
51. 删除图片服务仅供测试使用的 `event_line` 重导出及重复 NDJSON 换行测试；该合同由 `a4n-sidecar-bootstrap` 的生产函数和专属测试唯一负责。
52. 删除 Embedding 服务同构的 `event_line` 测试别名及重复序列化测试，避免跨 crate 重测同一实现。
53. 图片生成终态发送删除空 `is_err` 分支；接收端已关闭时直接结束生产任务，不再保留无作用条件语句。
54. 视频生成终态发送删除同构空分支，断流后的唯一行为仍是结束已完成的生产任务。
55. RAG 写入终态发送删除同构空分支；持久化完成后的客户端断开不再触发空逻辑。
56. 图片服务测试删除手工构造失败信封的协议层重复断言，仅保留本服务拥有的进度与完成 payload 投影。
57. 视频服务测试删除同一份失败信封构造器断言，仅验证视频提交和完成 payload。
58. RAG 错误脱敏测试直接覆盖本 crate 拥有的 `service_error` 映射，删除借失败事件包装间接验证的第二路径。
59. Embedding 错误脱敏测试同样直接覆盖 `service_error`，不再将协议事件构造混入服务错误映射测试。
60. Embedding 下载终态发送删除“失败后 break、成功后再次 break”的重复控制流，终态发送结果显式忽略后唯一退出。
61. Embedding 实时进度序列化失败后，无论终态事件是否成功送达都只执行取消、等待下载任务和退出；删除两份相同收尾分支。
62. Embedding 下载完成后的积压进度序列化失败同样统一为“尝试发送失败事件后退出”，删除结果无差异的条件分叉。
63. 图片事件入口删除仅调用一次的 `failed_response` Facade，校验失败直接组装协议终态响应。
64. 删除 LLM Gateway 整个单调用 `stream::projection` 模块；Chunk 序列化与 Completed 终态直接在唯一流状态机中调用协议构造器。
65. 视频服务测试将三个通配符导入收敛为实际使用的合同、事件投影和校验函数，避免隐式扩大依赖面。
66. Runtime 动态 LLM 删除 `sidecar_failed_event` 测试式别名；能力传输失败直接写入协议层 `CapabilityTaskEvent::failed`，不再经 streams 模块二次导出。
67. NDJSON 事件序列化删除“失败事件再序列化、再次失败改用硬编码 JSON”的两级回退；`CapabilityTaskEvent` 仅含 JSON 兼容字段，违反该不变量时直接显性失败。
68. NDJSON 响应构建删除静态合法 Content-Type 对应的不可达 500 回退，固定响应头不再伪装运行时可恢复错误。
69. 删除仅服务于上述响应构建回退的 `event_context` 参数及五类 Sidecar 调用点中的重复标签字符串。

70. 图片/视频生成消费端不再静默忽略 LLM 专属 Chunk 事件；跨能力事件现在直接报告合同错误，避免非法流被伪装成继续处理。
71. parse_generation_event 删除仅为“忽略 Chunk”服务的 Option 层，进度和完成事件改为确定的共享解析结果。
72. 图片与视频事件处理器删除对应的 Some/None 分叉，唯一事实源只保留各自真正支持的 Progress、Completed、Failed 路径。

73. LLM 消费事件将模糊的 Ignored 变体改为显式 Progress；心跳/进度被忽略是调用场景的明确行为，不再以泛化占位名称隐藏协议语义。

74. HTTP Sidecar Transport 删除 request_id/trace_id header 转换失败时的静默丢弃；两个追踪 header 现在在传输边界显式校验并返回配置错误。
75. 删除 Runtime 手写的 Sidecar URL、端口与 IPv6 解析器；HTTP(S) loopback 判定统一由 `a4n-service-manager` 基于标准 URL 解析提供，密钥转发与远端服务安全校验共享唯一主机语义。
76. 删除仅被请求组装调用一次的 `request_id` UUID 转发函数；请求上下文 owner 直接生成 UUID，不再跨 trace 模块保留零逻辑别名。
77. 删除仅替 `SidecarRequestContext` 写入 secret policy 后继续转发的 `service_request_value_with_context_and_policy` Facade；provider policy 直接进入唯一 Value 请求构建器。
78. 全局无密钥请求删除重复写入 `deny_all`；`SidecarRequestContext::current_global` 已由唯一构造器保证默认拒绝密钥转发。
79. 纯转换请求删除同构的 `deny_all` 二次赋值；stateless 上下文的安全默认不再由调用点重复声明。
80. RAG 请求删除第三处 `deny_all` 重复赋值；仅保留项目作用域和可选业务 trace 的差异配置。
81. 删除随 RAG 重复策略赋值一并失去用途的 `SecretForwardingPolicy` 导入。
82. Provider 请求删除与 `capability_version` 并行传递的 `service_name` 第二身份；服务名现在只由协议注册表解析，杜绝字符串与 capability 配错。
83. 全局 Provider 请求复用带上下文的唯一策略构建器，删除第二次服务 URL 查询与重复 secret-policy 分派入口。
84. capability 到 service 再到有效 URL 的解析收敛为服务管理层唯一函数；通用 egress 策略与 Provider 密钥策略不再各自重复查注册表和环境配置。
85. 删除 `sidecar_url_for_service` 已失去生产调用后的两层重导出，并将其可见性收回环境解析模块内部。
86. 从 Sidecar catalog 删除可由 `default_addr` 完全派生的八份 `default_url`，默认端点只保留一个配置事实源。
87. 默认 URL 解析删除 `addr == default_addr` 特判；无显式 URL 时统一由已校验地址构造 `http://{addr}`。
88. Bootstrap 新增 catalog 驱动的 `service_socket_addr` 唯一监听地址入口；服务启动只提交协议服务身份。
89. Prompt、LLM、Embedding、RAG、Image、Video、Import、Export 八个生产 Sidecar 删除各自硬编码的地址环境变量与默认端口；demo sidecar 因不属于生产 catalog 保留显式配置。
90. 八个生产 Sidecar 的资源策略查询删除重复服务名字符串，统一复用协议服务常量，与监听、健康检查和请求校验共享同一身份。
91. 删除仅绑定旧手写 URL 解析器、且与标准 URL/现有 Sidecar URL 配置解析语义冲突的空端口断言；loopback 安全判断只关心解析后的真实主机与 HTTP(S) scheme。
92. Prompt 服务删除仅转发一次的 `service_addr` 函数及随之失效的 `SocketAddr` 导入，启动入口直接调用 catalog 地址 owner。
93. RAG 服务删除同构 `service_addr` Facade 和死导入，八类生产 Sidecar 的地址解析形态保持一致。
94. 删除 Runtime 对共享 loopback URL 判定的本地重命名重导出；生产代码与合同测试直接引用服务管理层唯一函数名。
95. Minimal profile 的内嵌服务选择删除 `prompt/import/export` 字符串副本，改用协议服务常量。
96. Runtime 能力装配将 Prompt/LLM/Embedding/RAG/Image/Video/Import/Export 八套同构 HTTP client 构建函数收敛为一个生成规则。
97. 删除只转发 `ServiceManager::auth_token` 与 `capability_http_client_factory` 的两层本地包装；共享装配规则直接调用真实 owner。
98. 八类能力 URL 查询和 client 装配删除服务名字符串副本，统一使用协议服务常量。
99. HTTP 能力构建函数内部按自身服务身份读取 URL，删除调用层八个临时 URL 和“某服务 client 接收另一服务 URL”的可分叉参数面。
100. Runtime Remote Sidecar 配置改为复用 `ManagedServiceConfig::remote`，删除 mode、args、URL、env、启动超时和重启策略的第二套手写默认。
101. 删除上述手写配置消失后失去用途的 `HashMap` 与 `ServiceMode` 导入。
102. 删除已失去生产者的 `data/services/auth-token` 文件回退；外部 Sidecar 不再依赖已退役启动脚本遗留的磁盘兼容路径。
103. 外部 Sidecar 的本地认证 token 收敛为 `A4N_SIDECAR_AUTH_TOKEN` 唯一来源，并复用共享的非空环境变量严格解析；Remote 配置路径不再读取 `AppConfig.data_dir`。
104. Remote Sidecar 配置不再把同一 token 复制进 `ManagedServiceConfig.env`；配置只声明凭据环境变量名，由 `ServiceManager` 从唯一进程环境读取。
105. 删除 Remote 模式下无消费者的本地 Sidecar `binary` 发现与赋值；二进制路径只属于真正会启动进程的 `ServiceMode::Sidecar`。
106. 删除 CLI local start 自动生成随机 `A4N_SIDECAR_AUTH_TOKEN` 的伪安全逻辑；managed 模式使用 Runtime 为每个进程生成的 token，外部模式则必须由操作者显式提供双方共享凭据。
107. 修正 CLI 与 README 对 `--no-sidecars` 的分叉说明：该选项表示连接外部 Sidecar，而非禁用能力或启动一个可独立工作的“核心 Server”。
108. 全局 Sidecar token 的必填检查只作用于 loopback 外部进程；非 loopback 服务继续由服务管理层强制使用独立部署 token 或完整 mTLS 身份，不再收到误导性的本地 token 提示。
109. 在现有 shell 公共库建立唯一 `sidecar_catalog_rows` 读取入口，Linux 与 macOS 发布 smoke 不再各自维护 JSON 字段投影。
110. Linux 发布 smoke 删除八份服务名、地址变量、URL 变量和固定数量 `8`；健康路径、端口上限、Server 端口、PID 数量及输出全部由 catalog 派生。
111. Windows 发布 smoke 删除同构的三套硬编码数组与固定数量 `8`，改由 `service-catalog.json` 派生服务身份、地址、健康路径和生命周期断言。
112. Release manifest 校验始终读取仓库唯一 catalog，删除 `binaries.length == 9` 和仅在显式传参时才校验 catalog 的分叉。
113. 删除无活跃定制消费者的 `--catalog` 参数及 deploy、CI、manifest 检查中的重复 canonical path 参数；生成和校验均固定使用协议 crate 的 catalog。
114. Release manifest 的服务库存校验与 Runtime 一致按完整服务集合判断，删除数组顺序作为额外隐式合同的耦合。
115. 扩展现有 Shell catalog owner，新增唯一 `sidecar_catalog_binaries` 投影，并在 owner 内统一拒绝非数组或空 catalog。
116. `scripts/run.sh` 删除依赖 JSON 排版的 `sed` 正则解析，源码 managed-sidecar 构建列表改用共享投影。
117. `scripts/deploy.sh` 删除本地 Ruby catalog 解析器、catalog 路径副本和重复 Ruby 前置检查，发布打包只消费共享二进制投影。
118. Tauri externalBin 准备脚本删除第二套 Ruby catalog 解析器及空目录重复检查，构建包列表由共享 owner 保证。
119. Tauri bundle 配置生成删除第三套 catalog JSON 读取，直接接收共享投影的二进制列表。
120. CI sidecar matrix 删除 workflow 内的 catalog 字段解析，复用同一 Shell 投影后仅负责转换为 GitHub matrix JSON。
121. Release manifest 篡改检查删除三份 fixture catalog 解析及硬编码 `a4n-prompt-service` 目标，fixture 和首个篡改对象均由 catalog 派生。
122. CycloneDX SBOM 删除自身的 catalog 解析器，Host 与 Sidecar 二进制库存统一来自共享发布投影。
123. SBOM 删除固定“Host + 八个 Sidecar / 9 份”断言；每个二进制必须唯一映射 workspace package，每个选中 package 必须生成实际 SBOM，空输出无法再由固定数量掩盖。
124. SBOM 的 package 与 generated-file Ruby 查询由进程替换改为显式命令赋值，解析失败会立即终止，不再被 Bash 静默折叠为空数组。
125. 协议注册表将 `service_for_capability_version` 内联服务数组提升为唯一 `REGISTERED_SERVICE_NAMES`，注册表遍历不再维护第二份列表。
126. Catalog 合同测试删除固定 `entries.len() == 8`，改为逐项对齐注册表唯一服务清单，同时保留 capability 合同核对。
127. Runtime Full profile 测试删除固定 `configs.len() == 8`，改为核对完整 catalog 服务身份及全部 Embedded 模式。
128. README、CLI 手册、CLI 验收记录、发布说明和 CI step 名删除固定 `8/9` 描述，统一表述为 catalog 驱动的 Sidecar 库存。
129. Release manifest 的 catalog 读取边界统一拒绝空数组，避免空清单被生成成伪合法 Host-only 发布包。
130. Portable 发布包内生成的 README 删除八服务名称枚举，目录说明同样只引用 service catalog。
131. 新增 `crates/a4n-service-protocol/release-tools.json` 作为跨 Rust、Ruby、Bash、PowerShell 与 Tauri 的唯一发布工具库存，Cargo package 与最终 binary 名不再散落在各脚本。
132. 协议层新增 `ReleaseToolCatalogEntry` 与 `release_tool_catalog`，Rust 消费者直接读取同一嵌入式 catalog，不再维护本地工具数组。
133. `ReleaseManifest.tools` 删除 `serde(default)` 与空数组省略兼容；缺少工具清单的旧 manifest 现在直接反序列化失败。
134. 删除无生产引用的 `ReleaseManifest::binary` 查询方法，避免为已不存在的单项查找路径保留死 API。
135. Shell 公共库新增唯一 `release_tool_rows`、`release_tool_binaries` 与 package-to-binary 查询；catalog 在边界统一拒绝空数组、非法标识符、重复 package 和重复 binary。
136. `release_manifest.rb` 删除 `TOOL_BINARIES` 常量，生成与校验均从唯一工具 catalog 派生名称、版本、平台路径与哈希，并统一校验 catalog 必填字段和唯一性。
137. Release manifest 生成删除 `host = package`、默认输出路径和 verify 接受 `--output` 的回退；BFS 确认全部活跃调用者均显式提供 `--host`、`--output` 或 `--manifest` 后，将参数合同收紧为唯一明确入口。
138. `scripts/deploy.sh` 删除 `TOOL_PACKAGES`、`tool_binary_name` 与三工具固定输出；构建、缺失检查、打包复制、系统安装、dry-run 和数量说明全部由工具 catalog 派生。
139. Tauri externalBin 准备脚本删除硬编码 `a4n-cli:a4n` 等三项映射，工具 package 与 binary 由共享 catalog 行投影唯一提供。
140. Tauri bundle 配置删除静态三个工具 `externalBin`，构建时由 service catalog 与 release tool catalog 合成唯一列表，并显式拒绝跨 catalog 重复 binary。
141. Release manifest fixture 的 Linux、Windows、macOS 工具文件和篡改对象全部从 catalog 派生，删除三平台固定工具循环及硬编码 `a4n-server` 篡改目标。
142. Linux 与 macOS 发布 smoke 的工具存在性和 CLI binary 路径改由 catalog 派生；Windows smoke 同步删除三工具数组，并拒绝空字段、非法标识符、重复 package 或重复 binary。
143. CycloneDX 发布 SBOM 将 release tool catalog 声明的工具 package 纳入真实发布库存，修复此前随包工具没有独立 SBOM 的覆盖缺口。
144. CI 与发布文档的 SBOM 描述统一为 Host、release tools 与 sidecars，不再把实际发布库存误写成仅 Host + Sidecar。
145. ServiceManager 对 manifest tools 执行 catalog 精确集合匹配，统一校验工具名、`bin/` 路径、target、版本字段、SHA-256 形状与重复项；多余、缺失或旧工具清单均直接拒绝。
146. ServiceManager 在存在 package root 时实际哈希全部工具文件，工具缺失或被篡改不再只依赖 manifest 自述；Sidecar 启动前重复校验 ONNX Runtime 的 TOCTOU 安全边界继续保留。
147. ServiceManager 测试 fixture 改为 catalog 驱动，并增加缺失任一发布工具即拒绝的合同检查；测试代码已随生产合同更新，但遵照资源约束尚未执行。
148. `tools` 从兼容缺省改为必填属于破坏性合同变化，release manifest schema 因此由 v2 升为 v3；版本值收敛到协议层 `RELEASE_MANIFEST_SCHEMA_VERSION`，Rust 校验、Ruby 生成/校验和 portable Windows 启动脚本均从该 owner 派生，不再继续复用旧 schema 号。
149. 删除 `scripts/deploy.sh --install` 及整套 `/usr/local` 直拷贝实现；全仓 BFS 确认该入口只有脚本自引用，且其分叉布局不生成/安装 release manifest 与启动脚本，会绕过 portable 包的完整性合同。发布交付现在只保留唯一 portable package 实现。
150. `scripts/run.sh` 删除 `--frontend` 模式；前端独立开发唯一使用既有 `npm --prefix frontend run dev`，启动总入口不再包装一条零逻辑命令。
151. 删除 `--migrate` 与 `--check` 两个纯转发分支及对应状态变量；迁移和诊断分别由 `a4n local migrate`、`a4n doctor` 唯一负责，也移除了 remote API 下迁移参数静默成功的空路径。
152. 删除无活跃调用的 `-r/-s/-f/-m`、`--embedded` 与 `--no-reload` 临时别名；开发启动参数只保留文档化的单一名称。
153. `--no-services` 收敛为与 CLI/Runtime 术语一致的 `--no-sidecars`，不保留旧名称兼容分支。
154. 启动目标从可分叉字符串 `MODE=tauri|server|frontend` 收敛为单一 `SERVER` 布尔状态；删除 frontend 模式后不再保留伪枚举。
155. ServiceManager 将 release tool 与 ONNX Runtime 两套同构文件哈希循环合并为唯一 `verify_manifest_files`；文件类型只作为错误标签传入，Runtime 的重复校验调用时机不变。
156. `a4n local start` 删除无活跃调用的 `--no-migrate` 逃生开关与 `StartOptions.migrate` 双态；本地 Server 启动现在始终先调用独立 `a4n-migration`，不再允许跳过后等待 Host schema 检查失败。
157. `a4n-server` 的三个 embedded feature 删除 `a4n-runtime/migration`；Server 无论 minimal/full 都只校验 schema，迁移统一由 CLI 或部署编排调用独立 migrator，消除 full Server 的双重迁移。
158. `a4n-tauri` 新增默认 `migration` feature，minimal 与 embedded 桌面 Host 均复用 `a4n-migration` 唯一实现执行启动迁移，portable 与直接 `.app` 启动不再依赖未说明的人工预迁移。
159. `scripts/run.sh` 只在 Web Server 模式调用独立 migrator；Tauri 开发启动交由桌面 Host 自身的启动迁移 owner，删除桌面模式连续执行两次迁移。
160. Tauri bundle 构建删除默认 `--no-default-features`，确保真实发布 bundle 包含桌面启动迁移；full 模式只额外启用 embedded capabilities。
161. Tauri bundle 参数删除无活跃调用的 `--embedded` 同义别名，只保留 `--full` 唯一名称。
162. `a4n-migration` 删除“位置参数优先、`DATABASE_URL` 回退”的双接口，只读取项目统一的 `A4N_DATABASE_URL`；CLI 通过子进程环境传递连接串，不再把数据库凭据放进进程参数。
163. CI 数据库迁移调用删除连接串位置参数与独立 `DATABASE_URL` 别名，统一依赖 job 的 `A4N_DATABASE_URL`。
164. Linux、Windows、macOS 桌面发布 smoke 删除构建前人工迁移步骤，真实 fresh database 由 Tauri 启动迁移 owner 处理，避免测试继续掩盖默认桌面包缺少迁移能力。
165. 云部署构建与发布包纳入 `a4n-migration`，Server 与 migrator 共用同一 Ubuntu 24 构建、GLIBC 校验、上传和原子替换流程；云端 Server 不再隐式依赖人工迁移。
166. 云部署在替换二进制前显式停止服务，从远端 `.env` 严格读取唯一 `A4N_DATABASE_URL`，迁移成功后才重启；删除“运行旧 Server 时直接替换后碰运气重启”的分叉时序。
167. `scripts/run.sh` 对 Server + remote API、full + no-sidecars、remote API + full/no-sidecars 的无效组合改为解析后立即失败；后续分支不再静默覆盖用户参数。
168. Release manifest 生成删除与 `--package` 永远同值的 `--host` 第二身份及全部调用参数；Host binary 名和 Cargo version 只由 package 派生。
169. Ruby verifier 与 ServiceManager 同步强制 manifest package、Host name、Host path 和 package version 一致，删除“package 元数据与实际 Host 可分叉但仍通过”的合同空洞。
170. 删除 macOS manifest layout 中已由通用文件名校验和 package-derived Host 合同覆盖的 `host` 二次投影；ServiceManager 同时将 Host 路径从宽泛 `bin/` 前缀收紧为精确 `bin/{host}`，并校验 Host/manifest version 一致。
171. Shell `sidecar_catalog_rows` 从“仅非空”收紧为统一校验服务身份、binary、地址/URL 环境变量、健康路径及四类唯一性；所有 Bash 消费者共享同一失败边界。
172. Windows release smoke 删除对原始 Sidecar catalog 的宽松直读，补齐与 Shell owner 对等的必填、格式和唯一性校验，三平台不再各自接受不同的坏 catalog。
173. `scripts/build.sh` 删除无活跃消费者的 `-r/-c/-p/-w` 短别名，CI 改用 `--package`；构建入口不再维护两套参数名称。
174. 删除从声明后从未读取的 `EXTRA_FLAGS` 死变量。
175. 删除三个分支只赋值却从未消费的 `mode_label` 死状态。
176. Debug、Release 与 check 三个分支只负责选择 Cargo 子命令和标题，实际执行统一收敛为一次 Cargo 命令调用，删除三套同构状态捕获。
177. Cargo 工作目录切换从三个模式分支提升到唯一执行点，删除重复 `cd`。
178. 删除 `run_cargo_capture` 的逐行 warning 过滤；该逻辑会连同错误位置、源码片段和 help 一起吞掉，Cargo 诊断现在完整显性输出。
179. 构建产物提示改用公共库唯一导出的 `CARGO_TARGET_DIR`，删除脚本内第二份 `target` 路径常量。
180. 删除只为上述过滤服务的 `--warnings`、`SHOW_WARNINGS`、warning 计数及临时捕获文件；构建入口不再写系统 `/tmp` 或维护两套诊断模式。
181. 删除 `do_build` 必然 `exit` 后永远不可达的“全部构建完成”尾部输出。
182. `--frontend` 前置检查不再要求 Rust/Cargo，也不再进入后端依赖图判定；前端构建与 Rust 工具链解除无效耦合。
183. 缺失前端依赖时统一使用仓库既有的 `npm ci` 锁文件安装合同，删除构建脚本中的 `npm install` 第二策略。
184. 公共构建缓存说明删除 `/tmp` 实验目录建议，临时和计时产物统一指向仓库现有 `target/`。
185. `should_build_frontend` 的两段早退与固定成功返回收敛为单一布尔谓词。
186. `cargo_target_args` 将 `--no-default-features` 和 `--features` 从 workspace/package 两个分支收敛到唯一输出点。
187. 删除 `needs_onnxruntime` 中已由调用边界保证、永远不会再触发的 frontend-only 分支。
188. Node/npm 前置检查复用公共 `require_command`，删除两套手写 command lookup 与退出分支。
189. 删除主流程中 `BACKEND_ONLY` 的提前 `do_build` 分支；最终唯一 `do_build` 已覆盖该模式，原分支只依赖函数内部 `exit` 阻止第二次调用。
190. 部署脚本删除只调用一次、仅转发两个 `require_command` 的 `check_node` Facade。
191. Portable 与云部署删除各自只调用一次的 `install_frontend_deps` / `ensure_frontend_deps` Facade，不再保留脚本私有包装层。
192. README、开发构建、portable 部署与云部署统一使用 lockfile 严格的 `npm ci`，删除 `npm install` 第二套依赖解析策略。
193. 删除 229 行无任何外部调用的 `cargo-build-adaptive.sh` 整模块，以及其 OOM 猜测重试、隐式 ONNX warning fallback、仓库外 `.claude` 日志和 `/tmp` attempt log；构建只保留实际使用的入口。
194. 构建参数边界显式拒绝多构建模式、重复 package、`--all + --package` 以及 frontend-only 与后端参数混用；删除“后出现者覆盖前者”或参数被静默忽略的隐式回退语义。
195. 公共脚本库新增唯一 `make_target_temp_dir`，所有本地临时工作目录统一创建在已有 `CARGO_TARGET_DIR/tmp` 下。
196. generation contract 生成器复用公共项目根与 target 临时目录，删除自身 `ROOT` 解析和系统默认 `mktemp -d`。
197. generation contract 总检查入口同样复用公共项目根，删除第二套仓库定位逻辑。
198. Qdrant 安装脚本复用公共项目根，并将下载/解压工作区从系统 `/tmp` 移入 `target/tmp`。
199. Release manifest 篡改 fixture 改用公共 target 临时目录，删除 `TMPDIR`/`/tmp` 双来源。
200. Linux portable 与 macOS Tauri 发布 smoke 的安装前缀、日志和运行数据统一落入 `target/tmp`，保留原有失败留存开关但不再散落系统临时目录。
201. Build-feedback 工具自检复用公共项目根及 target 临时目录，删除自身根路径和 `TMPDIR` fallback。
202. Sidecar 版本矩阵复用公共项目根，其历史源码、probe 和两套 Cargo target 均收敛到仓库 `target/tmp`。
203. 开发构建、portable 发布与云部署的前端依赖检查/安装收敛到公共 `ensure_frontend_dependencies`；三处不再复制 `node_modules` 判断和 `npm ci` 流程。
204. 删除 268 行自动构建缓存守卫整模块；缓存容量管理不再隐藏在每次普通构建、运行、测试或发布入口之前。
205. Build、test、run、portable deploy、Tauri bundle 与 externalBin 准备删除全部 `guard_build_cache` 调用，主流程只执行用户明确请求的工作。
206. 删除超过软阈值即自动移除 debug/release/交叉目标 incremental 的逻辑，避免制造重复编译。
207. 删除可选 `cargo-sweep` 探测及两种参数形态的吞错 fallback，不再依赖未声明工具猜测清理旧 fingerprint。
208. 删除硬阈值与 `A4N_BUILD_CACHE_FORCE_CLEAN` 触发的自动 `cargo clean`；任何完整冷编译现在都必须由操作者明确发起。
209. 删除仅服务于自动清理的 `/proc` 编译进程猜测、`/tmp` flock、target-* 年龄扫描及六个 `A4N_BUILD_CACHE_*`/保留开关。现有 `target/` 内容未被删除。
210. 公共脚本库建立唯一 `cargo_graph_uses_onnxruntime`，由真实 Cargo normal/build/dev 依赖边决定是否需要本地 ONNX Runtime。
211. Build 与 test 入口统一复用依赖图 owner，删除测试脚本中容易随 feature 漂移的五 crate 硬编码正则。
212. 删除无活跃消费者且会显著放大编译成本的 `scripts/test.sh --release` 包装参数；如确有发布模式测试需求，唯一入口仍是显式 Cargo 命令。
213. 云部署删除解析 `--ssh-key` 时对尚未成形、随后必被整体覆盖的 `SSH_OPTS[1]` 死赋值。
214. 删除云部署私有 `check_command`，本地依赖检查统一复用公共 `require_command`。
215. API base 只在参数解析完成后由最终 public base 派生，删除 `--base-path`/`--api-base-url` 顺序导致后参数静默覆盖前参数的分叉。
216. 云端 Server 与 migrator 构建从 `target-noble` 第二缓存收敛到仓库唯一 `target/release`，并删除 `A4N_DEPLOY_TARGET_DIR` 可分叉配置。
217. Dockerfile、部署 staging 与归档从 `$HOME/.cache` 收敛到 `target/deploy-cloud`，所有生成物留在用户指定的仓库可见构建目录。
218. 删除远端 Nginx 配置路径、参数和“只备份但从不修改”的无效分支，也移除其中吞掉 sudo 失败的 `|| true`。
219. 远端预检显式要求后续 `.env` 更新和迁移实际使用的 `python3`，依赖缺失不再部署到一半才失败。
220. 云部署删除覆盖 workspace release profile 的 LTO/codegen-units 第二策略，发布二进制统一服从根 Cargo profile。
221. 显式拒绝 `--skip-build` 与 rebuild/online 配置混用，删除被静默忽略的构建参数。
222. 公网 smoke 不再找不到 JS asset 就跳过，上传包清理也不再吞掉 SSH 失败；发布验证和清理结果均显性化。
223. `a4n-core` 新增唯一 `required_non_empty_env_string`，必填字符串环境变量统一复用已有非空解析，不再由各模块重复拼装“缺失/空值”分支。
224. Runtime baseline 配置删除自身的 optional/required 字符串环境变量解析器，LLM、图片和视频公共配置统一调用 core owner。
225. 临时媒体上传合同删除旧 `source_image_id` 与可选 `source_media_kind/source_media_id` 双身份，所有图片、视频和音频上传只保留必填 `source_media_kind + source_media_id`。
226. Server 临时上传从图片专用 MIME/magic/扩展名实现收敛为一套媒体规则，参考图片、视频和音频不再走平行上传实现。
227. 临时媒体公网地址只由 `A4N_PUBLIC_APP_URL + /api/temp-media` 派生，删除 `A4N_TEMP_MEDIA_PUBLIC_BASE_URL` 第二入口及其优先级回退。
228. `A4N_TEMP_MEDIA_UPLOAD_TOKEN` 改为显式必填，删除缺失 token 的可选解析和二次空值兜底。
229. `A4N_TEMP_MEDIA_TTL_MINUTES` 改为显式必填，删除内置 360 分钟默认值。
230. `A4N_TEMP_MEDIA_MAX_BYTES` 改为显式必填，删除内置 20 MiB 默认值。
231. 云部署不再自动生成临时媒体 token，也不再向远端 `.env` 写入 TTL、大小或第二公网入口默认值；部署只更新唯一 `A4N_PUBLIC_APP_URL`，业务配置缺失会显性失败。
232. Runtime `public_url` 传输只由 `A4N_PUBLIC_APP_URL + /api/media` 派生，删除 `A4N_PUBLIC_MEDIA_BASE_URL` 分叉及“双变量均缺失”兼容提示。
233. 删除 `TempMediaUploadRequest.expires_in_seconds`；客户端提交的 TTL 原先只用于与服务端固定策略比较，最终从不决定过期时间，现在 TTL 唯一归属 Server 配置。
234. 临时媒体 24 小时上限提升为 API contract 的 `TEMP_MEDIA_MAX_TTL_SECONDS`，Server 配置边界与 Runtime 响应核验不再各自维护 `24 * 60 * 60`。
235. 临时媒体 `data_base64` 只接受合同声明的裸 base64，删除自动剥离 `data:` URI 的旧格式兼容转换。
236. 上传文件名改为必填非空；删除空字符串被当作“无提示文件名”静默接受的分支。
237. 文件过期查询显式区分“不存在”和“存在且过期”；serve 不再把缺失文件判为过期、随后删除失败并返回 400，删除竞态同样统一返回 404。
238. 临时媒体 TTL owner 直接输出秒数，upload 过期时间与 storage 清理共享同一结果，删除请求 TTL 计算器和分钟/秒二次策略分派。
239. Canvas 视频图片输入改为复用章节视频的唯一 `resolve_image_transport`；不再根据远端上传变量是否碰巧存在，另行自动选择 RemoteTempUpload/InlineDataUrl。
240. 删除仅服务于上述 Canvas 自动分派的 `remote_temp_upload_configured`、值组合判定、两层重导出及重复测试。
241. 删除 `A4N_VIDEO_IMAGE_INPUT_TRANSPORT=auto` 临时别名；默认值仍明确归属未设置配置，显式配置只接受 `inline_data_url`、`public_url`、`remote_temp_upload` 三种真实模式。
242. 远端临时上传 URL/token 删除本地 `std::env::var` 解析器，统一复用 core 必填非空环境变量 owner。
243. `public_url` JWT 媒体 token TTL 改为显式必填 `A4N_VIDEO_MEDIA_TOKEN_TTL_MINUTES`，删除 360 分钟默认值和只为默认分支存在的解析测试 Facade。
244. 云部署将数据库 URL 预校验、停服、包替换和迁移合并为单次远端事务；数据库配置在停服前验证，删除两个独立 SSH Facade 及“服务已停后才发现数据库配置错误”的分叉时序。
245. Ruby release manifest 扩展为服务库存、工具库存和 schema version 的唯一脚本投影 owner；新增 `service-rows`、`tool-rows`、`schema-version` 三个只读入口。
246. Shell 公共库删除两套内联 Ruby catalog 解析器，Sidecar 与 release tool 行投影统一调用 release manifest owner。
247. Shell 删除第二份 `RELEASE_MANIFEST_SCHEMA_VERSION` Rust 源码正则解析，portable 启动脚本版本只消费 Ruby owner 的 `schema-version`。
248. Windows release smoke 删除约六十行 PowerShell catalog 空值、格式和唯一性重复校验，改为消费同一 Ruby 行投影。
249. 发布库存 owner 新增 Sidecar binary 与 release tool binary 跨 catalog 冲突检查，避免 macOS `.app/Contents/MacOS` 中两个库存映射到同一文件。
250. Windows release smoke 工作目录从系统 `GetTempPath()` 移到仓库 `target/tmp`，生成包、日志和 CLI 状态不再散落系统临时目录。
251. Windows CI 的 Qdrant 下载与解压目录从 `RUNNER_TEMP` 移到 `target/tmp/qdrant`。
252. macOS CI 的 Qdrant 下载/解压和 release package 复核目录同样收敛到仓库 `target/tmp`。
253. CLI 验收记录删除 `/tmp/a4n-cli-target` 第二 Cargo target，文档化命令复用仓库现有 `target/`。
254. real-postprocess 与 video-scrub 浏览器截图从系统 `/tmp` 移入 `target/tmp` 下的专项目录。
255. Windows smoke 删除仅为查找 `a4n-cli` 构建的第二份 package-to-binary 哈希表，直接从已校验工具投影读取唯一条目。
256. `a4n-core` 建立唯一 `required_public_app_endpoint`，统一读取、解析并追加 `A4N_PUBLIC_APP_URL` 的固定 API 路径。
257. 公网应用入口统一拒绝非 HTTP(S)、缺失主机、URL 凭证、query、fragment、localhost、loopback 与 unspecified 地址，Server 和 Runtime 不再接受不同配置集合。
258. Server 临时媒体删除自身的 `http::Uri` URL 解析器、环境变量常量和专属重复测试，只请求 core owner 派生 `/api/temp-media`。
259. Runtime `public_url` 删除自身的 `reqwest::Url`、IP 主机判定、URL 归一化与重复测试，只请求同一 owner 派生 `/api/media`。
260. URL 合同检查移动到 owner 同文件，测试直接调用生产解析函数，不再复制一套测试专用判定实现。
261. CycloneDX SBOM 入口删除第二份仓库根 `ROOT`；脚本只保留定位公共库所需的 `SCRIPT_DIR`，工作目录与 Cargo manifest 统一使用 `PROJECT_ROOT`。
262. Sidecar 版本矩阵删除 `repo_root="$PROJECT_ROOT"` 零逻辑别名，历史 probe 复制、当前源码链接与工作目录直接引用公共根 owner。
263. Tauri externalBin 删除不可达的 `CARGO_TARGET_DIR` fallback、相对路径兼容和 `target_root` Facade；目标目录唯一使用公共库强制导出的仓库 `target/`。
264. Sidecar 版本矩阵删除私有 `command -v` 错误分支，七项 CLI 前置检查统一复用公共 `require_command`。
265. Linux 发布 smoke 删除第二套命令探测与重复 Ruby 预检；直接命令复用 `require_command`，Ruby 可用性由 catalog 投影 owner 在首次读取时保证。
266. macOS Tauri smoke 同样删除私有命令探测和重复 Ruby 预检，命令失败边界收敛到公共脚本库。
267. Core 依赖边界检查删除自身仓库根解析，统一 source 公共脚本库并使用 `PROJECT_ROOT`。
268. Generation 安全边界检查删除同构根路径实现，所有源码扫描从公共项目根执行。
269. Sidecar 安全边界检查删除第三套同构根路径实现，路由、传输和远程安全合同扫描共享唯一根 owner。
270. Cargo audit 例外检查删除自身 `ROOT` 及配置路径拼接，工作目录与 `.cargo/audit.toml` 均从 `PROJECT_ROOT` 派生。
271. Qdrant 开发启动入口删除独立仓库根解析，安装与启动脚本现在共享公共项目根；已有本地端口和存储业务配置保持不变。
272. Build-feedback 测量与预算入口删除各自的仓库根解析，统一 source 公共脚本库；默认测量产物只从公共 `CARGO_TARGET_DIR` 派生。
273. Build-feedback 测量方法删除 `package-rebuild-v1` 脚本常量，直接读取 baseline TSV 的唯一 `# method` 合同。
274. 测量脚本删除八项 package 固定数组，默认冷/热编译库存由 baseline TSV 数据行唯一决定；显式 `--package` 仍只承担局部测量选择。
275. Baseline 刷新删除硬编码 `20%` 回归阈值；新测量值保留现有 baseline 中每个 package 的唯一阈值，消除与当前 `25%` 策略的分叉。
276. 预算脚本删除同一八项 package 集合；baseline 非空且其全部条目存在测量结果即为完整库存合同，不再要求同步修改 Python 集合。
277. Build-feedback 工具自检删除固定 `33` timing 目录、`24` reset 日志和 `8` dependency 文件断言，全部由 baseline package 数与本次 runs 派生。
278. 自检原始样本断言删除硬编码 `a4n-prompt-service`，改为读取 baseline 首个 package 并按 runs 核对。
279. 错误 method fixture 删除当前方法名副本，直接将 baseline 的 method 行替换为无效值；方法升级不再要求同步修改自检脚本。
280. 删除 `docker-compose.yml` 中无脚本、CI 或生产调用的第二套本地 Qdrant 服务及其 volume；本地索引后端只保留官方 release 二进制方案。
281. Qdrant 安装版本改由已跟踪的 `tools/qdrant/VERSION` 唯一声明，删除 `A4N_QDRANT_VERSION` 覆盖入口和脚本内 `v1.18.3` 默认副本。
282. 删除安装后写入但从未读取的机器架构 `tools/qdrant/TARGET` 与空 `.qdrant-initialized` 标记；仓库不再保存本机安装状态。
283. Qdrant 安装脚本删除内嵌 `config.yaml` 生成器，启动与安装都要求仓库已跟踪配置存在，端口、存储、CORS、cluster 与 telemetry 只保留一个配置事实源。
284. 安装脚本删除结果被 `|| true` 吞掉的 `qdrant --help` 伪验证和仅重复最终路径的 `ls` 输出；安装完成只报告实际写入的 binary 与固定版本。
285. Qdrant 手工启动入口删除 storage、snapshot、host、HTTP 与 gRPC 五项环境默认值副本，不再覆盖或复制 `config.yaml`。
286. Qdrant 启动删除配置不存在时直接执行二进制的内置默认 fallback；缺失跟踪配置现在显性失败，唯一启动路径始终传入 `--config-path`。
287. Qdrant 文档删除可变版本安装别名、当前机器架构状态和 Docker 并行方案说明，只描述固定 VERSION、跟踪配置及唯一 release 启动入口。
288. 新增 `tools/onnxruntime/VERSION` 作为跨 Shell、Rust 与文档的 ONNX Runtime 唯一版本事实源；`1.24.2` 字面量只保留在该数据文件。
289. Shell 公共库新增最小 `onnxruntime_version` 读取边界，默认安装目录、已安装 `VERSION_NUMBER` 校验和缺失错误统一消费同一版本。
290. ONNX Runtime 安装脚本删除自身版本常量，archive 名、下载 URL、临时目录、安装结果与平台错误均从公共版本 owner 派生。
291. CLI local 生命周期删除默认目录、版本比较与错误文案中的三份版本字面量，编译时直接嵌入同一 VERSION 文件。
292. Build 与 portable deploy 的前置检查输出删除版本副本，统一调用公共版本 owner。
293. README 与 `.env.example` 删除固定版本和带版本目录副本，文档只指向 VERSION owner 及 `<version>` 派生路径。
294. 新增 `tools/qdrant/releases.tsv`，以 GitHub v1.18.3 官方 asset digest 固定五个平台的文件名与 SHA-256；版本、asset 和校验和不再散落在 setup 与 CI。
295. Shell 公共库新增唯一 `qdrant_release_metadata` 读取边界，统一校验 VERSION、平台唯一性、asset 名和 SHA-256 格式。
296. Qdrant 安装脚本的平台 case 只负责将 `uname` 映射为规范平台键，删除 Linux/macOS 四份 release asset 名副本。
297. Qdrant 安装补齐 release SHA-256 强校验；下载内容与 release 清单不一致时显性失败，不再直接解压未验证资产。
298. Shell 公共库建立唯一 `sha256_file`，Qdrant 与 ONNX Runtime 安装删除各自的 `sha256sum`/`shasum` 分派副本。
299. Windows release smoke 删除固定 Qdrant URL、asset 和 SHA，PowerShell 进程生命周期直接消费公共 release metadata 的 Windows 条目。
300. macOS release smoke 删除 workflow 内下载 URL、SHA、解压和二进制启动实现，统一调用仓库 `setup-qdrant.sh` 与 `run-qdrant.sh`。
301. RAG Sidecar 进程测试删除 `docker run qdrant` 第二实现，在同一测试 step 内使用固定 release 安装/启动并显式回收进程。
302. Linux release smoke 删除 GitHub Actions Qdrant service 第二实现，改为与开发和 macOS 相同的 release binary/config 启动路径。
303. 三条 Shell CI 路径的 HTTP/gRPC 轮询收敛为公共 `wait_for_qdrant`；就绪前退出和超时统一输出实际日志并失败。
304. macOS Qdrant 启动与 bundle smoke 合并为单一 step，EXIT trap 负责回收进程，不再依赖跨 step 后台进程或 runner 最终清场。
305. CLI local 删除全仓无配置、文档或测试消费者的 `A4N_QDRANT_BIN` 二进制覆盖入口，Qdrant 不再绕过固定 release 安装。
306. CLI local 删除从系统 PATH 自动接纳任意版本 `qdrant` 的兼容 fallback；自动启动只接受当前源码 workspace 的 `tools/qdrant/bin`，发布包必须连接显式 `A4N_QDRANT_URL`。
307. 删除仅服务于上述备用来源的单调用 `resolve_qdrant_binary` Facade，唯一 workspace 路径检查直接位于启动决策点；通用 `find_binary` 仅保留 Server/migrator 发布工具发现职责。
308. CLI local 删除 `a4n-server` 与 `a4n-migration` 的系统 PATH 混装 fallback；源码模式只从当前 workspace 构建，发布模式只接纳与 `a4n` CLI 同目录、由同一 release manifest 固定的工具。
309. `find_binary` 收紧并重命名为 `sibling_binary`，删除环境 PATH 拆分、逐目录扫描和任意同名文件接纳逻辑；doctor 与实际启动共享同一发布布局判定。
310. managed sidecar 自动发现删除 `/opt/a4n/lib/a4n/services` 系统安装布局；生成该布局的 `/usr/local` 安装器已在第 149 项删除，当前发布合同只保留 portable `bin/services` 与 macOS 同目录布局。
311. 删除单调用 `installed_sidecar_directories` 包装及仅验证 `Path::join` 的对应单测，两个活跃候选目录直接位于 sidecar 解析决策点，不再维护第三份布局清单。
312. sidecar 发布布局解析删除“校验失败后静默尝试下一目录”的吞错探测；现在先由 `services/` 目录存在性唯一选择 portable 或 macOS 布局，再执行一次完整 catalog 校验，损坏包直接失败。
313. CLI Prompt 安装探测删除 `share/a4n/prompts` 旧系统布局；portable 根 `prompts/` 与 macOS `Resources/prompts` 继续承担全部活跃发布场景。
314. Runtime 唯一 Prompt 资源定位器同步删除同一 `share/a4n/prompts` 候选及对应错误文案，避免 CLI 与 Runtime 各保留一份已无生成器的系统安装兼容路径。
315. Runtime sidecar 二进制发现将显式 `A4N_SIDECAR_BIN_DIR` 收紧为唯一目录；配置存在时不再在失败后继续扫描其他布局，错误配置或缺失文件直接暴露。
316. 删除以 `A4N_PORTABLE_HOME/bin/services` 推断 sidecar 的分叉路径；该变量只拥有应用数据与 Prompt 根，portable 启动器已通过专用 `A4N_SIDECAR_BIN_DIR` 声明二进制目录。
317. 删除 Runtime 对 macOS `Resources[/services]` 与系统 `lib/a4n/services` 的 sidecar 扫描；Tauri externalBin 固定落在 Host 同目录，portable 固定落在 `bin/services`，两组旧候选均无打包生成器。
318. `sidecar_binary_dirs` 多目录库存收敛为单一 `sidecar_binary_dir`：无显式配置时只按当前 Host 同级 `services/` 是否存在选择 portable 或同目录布局，每个服务不再重复遍历一组备用目录。
319. Runtime sidecar 发现删除九个平台/架构的 Tauri target-triple 运行时矩阵；target 后缀只属于 bundler 输入命名，最终发布合同与 manifest 均使用平台无后缀二进制名。
320. 删除对目录中任意 `a4n-*-<suffix>` 文件的全量扫描、排序和“唯一即接纳”兼容逻辑，Runtime 不再运行未知 target 或任意后缀的同名前缀文件。
321. 删除 `sidecar_executable_candidate_names`、去重 helper、扩展名剥离与 Tauri 后缀匹配整条旧实现；运行时名称只由 service catalog binary 加当前平台 `.exe` 规则派生。
322. sidecar 二进制查找从双层候选循环收敛为精确文件名查找，并删除三项仅维护 target 后缀兼容的测试；保留一项 catalog 到平台文件名的最小生产合同检查。
323. Runtime sidecar discovery 删除私有 `non_empty_env_path`，复用 `a4n-core::config::env::optional_non_empty_env_path`；环境路径的未设置、空值、纯空白与非 Unicode 行为重新回到全仓唯一解析 owner。
324. `sidecar_binary_path` 改为沿 managed service 配置链返回解析错误；空白 `A4N_SIDECAR_BIN_DIR` 不再被当作缺省值后继续自动发现，错误配置现在显性阻止启动。
325. CLI `local.rs` 删除第二份 `non_empty_env_path`，ONNX Runtime、sidecar、源码根与 Cargo target 路径统一复用 `config.rs` 的单一 CLI 环境路径 owner。
326. CLI 路径 owner 改为直接读取 `OsString`，保留非 Unicode 系统路径并拒绝纯空白；不再借字符串解析器裁剪路径或让 `local` 与配置文件位置采用不同空值语义。
327. Runtime sidecar 名称解析删除单调用 `sidecar_package_name` Facade，service catalog binary 直接投影为当前平台可执行文件名。
328. 删除仅为测试注入多目录而存在的泛型 `sidecar_binary_path_in_dirs`；生产逻辑直接组合唯一目录与唯一文件名并检查实际文件。
329. 删除 `TestSidecarDir`、时间戳目录、手工清理 Drop、`touch` 和对应路径拼接单测共整套测试脚手架；真实 portable/Tauri release smoke 继续覆盖最终发布布局。
330. `sidecar_platform_executable_name` 收回 names 模块私有边界，测试模块不再暴露或依赖生产内部命名 helper。
331. macOS bundle smoke 删除将用户数据目录伪装成 `A4N_PORTABLE_HOME` 的错误覆盖；测试已显式设置 `A4N_DATA_DIR`/cache/tmp/log，包根继续由 `.app` 布局唯一解析。
332. CLI local 发布模式不再把配置目录注入为 `A4N_PORTABLE_HOME`，改为只声明绝对 `A4N_DATA_DIR`；源码模式仍以真实 workspace 作为 portable 根。
333. Core 新增唯一 `apply_storage_env_overrides`，统一读取 data/cache/tmp/log 四项环境路径、解析相对 portable 路径及 data 覆盖后的默认子目录派生。
334. Tauri 配置删除自身 `data_dir_overridden` 状态、四路径读取、三项 clone 与 cache/tmp/log 派生分支，直接复用 Core 存储环境 owner。
335. Server 删除四次 `apply_optional_path_env` 及单调用闭包 helper，改用同一 Core owner；`A4N_DATA_DIR` 覆盖后不再让 cache/tmp/log 停留在旧 portable 根。
336. Tauri 与 Server 的存储路径环境值统一采用 Core 的 OS 路径解析语义：纯空白显性失败、非 Unicode 路径保留，不再由 Tauri 字符串裁剪和 Server 私有转换各自决定。
337. CLI local 删除启动 Server 前重复注入 `A4N_PROMPT_SERVICE_PROMPTS_DIR` 的分支；显式用户配置继续由进程环境继承，自动定位只由 Server Runtime 执行。
338. 删除 CLI 私有 `bundled_prompts_dir` 及 portable/macOS 双候选扫描，Prompt 资源安装布局不再由 CLI 与 Runtime 各维护一套。
339. Runtime Prompt owner 删除从可执行文件同目录读取 `prompts/` 的无生成器候选；源码、portable 与 macOS 打包均不产生该布局。
340. Runtime Prompt 自动定位删除通用候选 `Vec` 与“首个 Registry 即接纳”回退，已识别 macOS bundle 和 portable package 时只校验各自唯一随包目录。
341. 发布包 Prompt 解析顺序前移到 DATA_DIR/CWD 之前，外部可写目录中的同名 `prompts` 不再覆盖或劫持随包模板；源码进程才保留 DATA_DIR 上级与 CWD 两个开发入口。
342. `a4n-service-protocol` 新增 managed sidecar、sidecar binary dir 与 Prompt resource 三项跨进程环境键，连同既有 release manifest 键成为 Rust 侧唯一协议常量 owner。
343. Runtime specs 删除本地 `MANAGED_SIDECARS_ENV` 与 `SIDECAR_BIN_DIR_ENV` 字面量声明，仅在模块边界重导出协议常量供现有调用链使用。
344. CLI local 与 Tauri setup 删除 managed/bin-dir/release-manifest 三类环境键硬编码，doctor、启动、自动配置和错误文案统一消费协议常量。
345. Runtime Prompt locator、managed sidecar 环境、Prompt service 入口及进程测试删除四份 Prompt 目录键副本，生产者、消费者和测试夹具共享同一协议名称。
346. Tauri setup 配置删除私有 `env_string` 与 `validate_env_value`，数据库、JWT 和 OpenAI 环境覆盖直接复用 Core `optional_non_empty_env_string`。
347. 删除只被配置文件路径调用一次的 Tauri `env_path` Facade；`A4N_DATA_DIR/config.toml` 定位直接使用 Core OS 路径解析与 portable 相对路径 owner。
348. Tauri 早期日志目录删除第二份字符串裁剪/Unicode 分支，直接复用 Core `optional_non_empty_env_path`，与正式 AppConfig 日志路径采用同一输入合同。
349. 删除两组只验证上述 Tauri 私有解析副本的四项单测、测试专用 re-export、`mod tests` 与独立 13 行 `src/tests.rs`；Core 已拥有对应空值和路径解析合同测试。
350. Tauri 配置文件、早期日志和正式存储覆盖现在一致保留非 Unicode OS 路径、拒绝纯空白，不再因启动阶段不同而出现三套路径语义。
351. Server 删除私有 `required_env` 与 `optional_env` 两套字符串解析，数据库、JWT、OpenAI 与 bootstrap admin 全部复用 Core required/optional owner。
352. 删除仅包装 optional 读取和闭包赋值的 `apply_optional_string_env`；LLM 与 Embedding API key、base URL 在 owning 配置函数内直接赋值。
353. Server 必填、可选、路径和存储环境配置现全部走 Core 统一空值、Unicode 与错误合同，不再与 Tauri/Runtime 保留第四套环境解析语义。
354. Core `config::env` 新增唯一 Rust PostgreSQL scheme 校验 owner；`postgres://` 与 `postgresql://` 的接受规则和按环境键生成的错误合同不再散落于各宿主。
355. Tauri 配置文件加载、环境覆盖与最终配置校验统一调用 Core owner，删除 `file_io` 中第二份 PostgreSQL URL validator；文件与环境两条输入路径不再各维护协议规则。
356. Server 删除私有 `validate_postgres_url` 及两项仅验证复制实现的单测，Runtime 启动配置同步删除本地 scheme 分支，二者直接复用 Core 合同。
357. Migration 正式入口与隔离测试库入口删除各自 `starts_with` 双分支，统一调用 Core owner，并分别保留 `A4N_DATABASE_URL` 与 `A4N_TEST_DATABASE_URL` 的准确错误上下文。
358. 横向搜索确认 Rust 生产代码已无第二份 PostgreSQL scheme 判定；CLI local 的完整 URL/host/端口解析与远端部署 Shell 的替换前预校验属于更强边界和跨语言边界，继续保留而不伪合并。
359. Service Manager 发布清单入口改为直接读取 `OsString` 路径，删除 `std::env::var` 的 Unicode 限制与 `VarError` 分叉；有效的非 Unicode 文件系统路径不再被误判为“不可读取”。
360. 发布清单包根解析删除无依据的当前目录 `.` 回退；清单可读后仍不存在父目录时现在显式报错，不再把完整性校验悄悄重定向到进程工作目录。
361. Core portable home 删除私有宽松 `non_empty_env_path` 对 `A4N_PORTABLE_HOME` 的读取，统一复用严格 OS 路径 owner；纯空白显式配置不再被当作相对包根继续启动。
362. Linux `APPIMAGE` 包根推导同步复用同一严格路径 owner，删除空字符串与纯空白值的第二套语义，同时继续保留非 Unicode 系统路径。
363. `AppConfig` 默认数据根删除对宽松 helper 的最后一个调用，`A4N_DATA_DIR` 与正式存储覆盖共享同一空值合同；Core 内该复制解析器已完整移除。
364. `AppConfig::default` 改为一次确定 `data_dir`，再直接派生 cache/tmp/log；删除同一配置构造中四次环境读取与四次 portable home 解析，保证四个默认目录来自同一快照。
365. Core 环境配置新增唯一布尔开关 owner，统一接受 1/0、true/false、yes/no、on/off，并对未设置、空值、未知值和读取错误采用同一严格合同。
366. Tauri 远程 API 模式删除私有 `env_flag_enabled`、`parse_env_flag` 和镜像单测，`A4N_REMOTE_API_ONLY` 直接消费 Core owner；真假词表不再由桌面宿主维护。
367. Runtime managed Sidecar 删除 `parse_managed_sidecars_env` 及其重复单测，`A4N_MANAGED_SIDECARS` 与 Tauri 开关共享 Core 唯一解析规则。
368. Runtime Sidecar URL/监听地址配置删除私有 `optional_env_string`，复用 Core 严格非空字符串 owner；环境读取、Unicode 和空白语义不再与其他宿主分叉。
369. managed Sidecar 监听地址装配改为一次读取对应环境变量，再在“显式地址”与“自动保留 loopback 端口”间单次决策；删除先探测、后再次读取的竞态与重复错误分支。
370. 删除仅包装一次环境读取和地址解析的 `sidecar_addr` Facade，配置装配直接调用唯一 `parse_sidecar_addr_env` 合同。
371. managed Sidecar 二进制定位直接消费当前 catalog entry 的 `binary` 字段，删除按 `service` 名再次加载并遍历 service catalog 的重复映射。
372. `sidecar_executable_name` 从可能失败的服务查找改为对已校验 binary stem 的确定性平台投影，删除不可能的 `None` 与回退到服务名的伪安全路径。
373. 自动二进制目录解析不再吞掉 `current_exe` 错误并伪装成“文件未找到”，现在保留实际 Host 可执行文件解析错误。
374. Host 可执行文件缺少父目录时不再静默返回无候选目录，改为显式配置错误；managed 模式的目录合同不存在隐式空路径。
375. Sidecar 子进程透传的 PATH、动态库、Qdrant、代理、证书与日志环境统一复用 Core 严格字符串 owner；读取失败、非 Unicode 或显式空白不再被静默丢弃。
376. Sidecar 路径环境序列化删除 `Path::display().to_string()` 的有损转换；当前 String 协议无法表示非 Unicode 路径时显式失败，不再把替换字符后的错误路径交给子进程。
377. data/cache/tmp/log 四项公共目录直接借用 `AppConfig` 路径生成环境值，删除四份无意义 `PathBuf` clone。
378. RAG、Embedding 与 Prompt 的派生路径先确定一次再严格编码，`configured_path` 改为借用默认根；路径装配不再依靠名为 fallback 的所有权复制。
379. Runtime Prompt locator 不再吞掉 `current_exe` 失败后继续尝试 DATA_DIR/CWD；无法判定当前进程是否属于发布包时直接暴露 Host 路径错误，避免放宽到可写模板来源。
380. Prompt 发布布局识别对“当前可执行文件缺少父目录”改为显式失败，不再把异常进程路径误当作普通源码模式。
381. 源码 CWD Prompt 候选不再吞掉 `current_dir` 错误并返回误导性的“已检查 CWD”；工作目录不可解析时保留实际系统错误。
382. RAG Core 新增唯一 `DEFAULT_QDRANT_COLLECTION` 事实源，Embedded Qdrant 配置与管理员存储摘要删除各自的 `a4n_rag` 默认值副本。
383. `QdrantStoreConfig` 的必填 URL 改用 Core `required_non_empty_env_string`，删除把非 Unicode 等读取错误统一伪装成“缺少变量”的私有分支。
384. Qdrant API key 删除 `.ok()` 吞错与空白即未设置逻辑，复用 Core optional owner；显式空值和不可读值现在阻止错误配置继续启动。
385. Qdrant collection 只在变量真正未设置时采用唯一默认名，显式空白不再静默回退到 `a4n_rag`。
386. 管理员存储路径摘要的 Qdrant URL/collection 删除两份 `std::env::var(...).unwrap_or_else`，展示层与实际 Index 使用同一严格环境语义和默认 owner。
387. 孤儿项目目录扫描只接纳 canonical 正整数目录名；`001` 等非规范数字目录不再被折叠成另一个项目 ID 后进入自动删除集合。
388. 孤儿目录删除不再把 API 展示字段 `item.path` 重新解析为文件系统路径，删除非 Unicode 路径经 JSON 字符串有损往返后成为执行依据的分叉事实源。
389. 孤儿目录删除改为使用 Core `ProjectPaths` 由 `data_dir + project_id` 唯一定位，删除 Runtime 中第二份 `projects/{id}` 路径拼接规则。
390. OAuth2 GitHub/Google client ID 与 secret 删除私有 `required_env`，统一复用 Core 必填非空字符串 owner；缺失、空白和读取错误不再被压成同一份复制错误。
391. 删除仅由生产入口和一项测试调用的 `oauth2_provider_from_credentials` 测试 Facade；Provider 行为测试直接构造具体 GitHub 实现。
392. OAuth2 Provider 选择从“环境键分派一次、构造函数再分派一次”收敛为单一 match，GitHub/Google 支持列表不再存在两份可漂移事实源。
393. Runtime 日志模块新增唯一 `build_log_filter_from_env`，统一负责 `RUST_LOG` 的严格环境读取、未设置时使用宿主默认值以及 EnvFilter 解析。
394. Server 删除本地 `env_filter_from_environment`、`initial_log_filter` 和三项镜像单测；Server 仅保留自身默认级别与 stdout subscriber 装配。
395. Tauri 删除第二套 `RUST_LOG` match、空值校验、EnvFilter parser 及三项重复单测；桌面端仅保留其详细默认 filter、滚动文件和 panic hook。
396. Runtime 日志 owner 新增共享 `LogFilterReloader`，统一持有 tracing reload handle、初始化状态、过滤表达式构建与 reload 错误合同。
397. Server/Tauri 删除 `OnceLock<RwLock<Option<Handle>>>` 三层重复状态，改为 `OnceLock<Handle>`；Handle 自身的并发 reload 能力不再被额外读锁包裹。
398. Server 动态日志级别更新删除本地 handle 读取、Option 解包、锁管理和 reload 实现，只调用共享 reloader。
399. Tauri 删除 `reload_filter` 包装函数及同一套锁/Option 分支，命令在管理员鉴权后直接调用共享 reloader；两 Host 的运行时日志级别实现现为同一事实源。
400. `RuntimeProfile` 新增唯一 `configured_from_env`，统一返回“未显式配置”或严格解析后的 full/minimal/sidecar，环境读取不再由 Host 自行探测。
401. `RuntimeProfile::from_env` 收敛为“消费显式 profile，否则应用构建默认”的单一组合，删除私有 `std::env::var`/`VarError` 分支并复用 Core 非空字符串合同。
402. Server runtime profile 选择删除对 `A4N_RUNTIME_PROFILE` 的第二次 `var_os` 读取；默认日志与最终 profile 使用同一配置快照。
403. Tauri runtime profile 选择同步删除直接环境探测；仅在共享 owner 确认变量未设置时启用 bundled managed sidecars，显式空白或无效值不再被默认逻辑遮蔽。
404. Service Protocol catalog 新增唯一 `parse_service_socket_addr`，统一 Sidecar 监听地址的 trim、空值拒绝、SocketAddr 解析和脱敏错误合同。
405. Runtime managed/remote Sidecar 配置删除私有 SocketAddr parser，Host 写入子进程环境前直接消费 protocol owner 并只保留字符串规范化。
406. Sidecar Bootstrap 删除第二份 `parse_socket_addr` 及镜像测试，实际 bind 前复用同一 protocol parser；非 loopback 必须认证的安全规则继续由 bootstrap 独立负责。
407. Sidecar Bootstrap 新增 catalog 驱动的 `service_socket_addr`；Prompt、LLM、Embedding、RAG、Image、Video、Import 与 Export 八个正式 Sidecar 删除各自的地址环境键和默认端口副本。
408. 八个正式 Sidecar 的资源策略查找统一消费各自 protocol service 常量，删除 `"prompt"`、`"llm"` 等第二份服务名字符串事实源。
409. Sidecar Bootstrap 新增单一环境解析 owner，统一可选非空字符串、缺省字符串、OS 路径、`u64` 缺省值与可选 `u32` 的未设置/空白/读取失败语义。
410. Bootstrap 日志过滤与监听地址删除私有 `env_value_or_default`/`resolve_env_value` 及三项镜像测试，统一消费严格环境 owner；默认值仅在变量确实未设置时生效。
411. Sidecar 文件日志删除私有路径与 `u64` 环境解析器；`A4N_LOG_DIR` 改用 `var_os` 保留非 Unicode 路径，日志保留天数和容量继续严格拒绝空白或无效整数。
412. Sidecar Runtime Metrics 删除私有 optional string/u32 解析器；重启次数与上次错误改由 Bootstrap 环境 owner 一次读取，读取失败不再被局部语义遮蔽。
413. Sidecar 认证令牌改在 `serve_sidecar` 启动边界严格解析一次并保存为只读 `Arc<str>` 状态；删除每个请求重新读取环境及 `.ok()` 吞掉非 Unicode/读取错误的路径。
414. 认证层统一挂载在 Bootstrap 合并完成的最终 Router，覆盖业务路由、`/metrics` 与 `/shutdown`；删除九个 Sidecar 入口及两个内部 Router 共十一处重复认证装配。
415. 删除 `require_auth` 的 `cfg(test)` 生产安全旁路和 LLM Router 的条件认证分支；测试直接构造业务 Router，生产认证只由真实 `serve_sidecar` 生命周期拥有。
416. 严格令牌解析保证认证状态只能是“未设置”或“非空值”，因此删除请求期再次 trim/过滤空白 token 的不可达防御分支；非 loopback 校验同步移动到实际监听生命周期并消费同一令牌快照。
417. Sidecar 日志清理器删除“当前无 Tokio Runtime 就静默不启动”的回退；全部九个调用入口均位于 `#[tokio::main]` 生命周期，后台任务现在直接依赖该真实不变量。
418. 日志文件名前缀删除任意环境键的多级 strip fallback，统一要求 `A4N_<SERVICE>_LOG` 合同并在初始化阶段显式拒绝不符合命名规则的键。
419. 日志保留天数删除 `saturating_mul` 静默饱和；无法换算为秒的超大外部配置现在明确返回配置错误。
420. 日志目录扫描删除把 `metadata.modified()` 失败伪装成 Unix epoch 的危险回退，并删除总容量扣减的饱和运算；文件元数据错误和内部记账不变量不再被遮蔽。
421. Service Protocol 新增 `SIDECAR_HEALTH_PATH`、`SIDECAR_METRICS_PATH`、`SIDECAR_SHUTDOWN_PATH` 三个跨进程路由事实源。
422. Service catalog 删除八条完全相同的 `health_path` 字段；健康端点不再伪装成逐服务可分叉配置，catalog 只保留真正按服务变化的名称、二进制、环境键和地址。
423. `ServiceCatalogEntry` 启用 `deny_unknown_fields`，删除字段后的旧 catalog 或拼写错误不再被 serde 静默忽略后继续发布。
424. Release Manifest CLI 的 `service-rows` 改为直接读取 Service Protocol 健康路径常量并投影第五列；Linux、Windows、macOS smoke 继续消费同一 CLI 输出，无需各自维护路径。
425. Runtime Sidecar spec 删除 `health_path` 读取、二次赋值和 `build_remote_service` 参数；managed/remote 配置统一继承 Service Manager 构造器的协议默认值。
426. Service Manager 当时将 serde 默认健康路径改为消费 Service Protocol 常量；该过渡配置字段已在第 450 项后续删除。
427. 八个正式 Sidecar、Demo、Capability HTTP 健康探测、Bootstrap 认证豁免与进程测试支持统一消费 `SIDECAR_HEALTH_PATH`，Rust 生产代码中的 `/healthz` 字面量清零。
428. Bootstrap `/metrics`、`/shutdown` 路由与 Service Manager 优雅关闭请求统一消费对应 protocol 常量，Rust 生产代码中的两条控制路径字面量清零。
429. Sidecar 历史版本矩阵不再硬编码 `/healthz`，启动轮询从 Release Manifest CLI 的 catalog 行读取当前协议健康路径。
430. `ManagedServiceConfig` 新增唯一 `endpoint_url`，健康探测与优雅关闭删除各自的 base URL 尾斜杠裁剪和路径拼接副本。
431. 删除仅包装一次 `endpoint_url(&health_path)` 的单调用 `health_url` Facade；健康请求直接消费通用 endpoint owner。
432. Sidecar 优雅关闭删除每次请求临时创建的 `reqwest::Client`，复用 Supervisor 启动和健康探测已持有的同一 Client 与连接配置。
433. 优雅关闭不再把“Sidecar 配置缺少 URL”伪装成普通关闭请求失败后强制终止；内部配置不变量破坏现在通过 `InvalidConfig` 显式返回。
434. Supervisor 完成路径删除 `task == None` 时返回成功的不可达分支；消费式 `stop/force_stop` 现在明确要求启动时创建的唯一 task 存在。
435. 重启后的健康检查失败不再吞掉 `next.kill()` 错误并继续创建下一进程；清理失败统一返回现有 `StartupCleanupFailed`，避免失控子进程被隐藏。
436. Service Manager 新增唯一 `configured_auth_token`，统一选择远程独立 token 环境或 managed Sidecar token，并优先消费配置内受控环境快照。
437. Token 环境读取删除两处 `std::env::var(...).ok()` 和空白过滤回退；缺失、非 Unicode 与显式空白现在分别收敛为明确配置失败，只有未声明 token 才返回 `None`。
438. `ServiceManager::auth_token` 改为 `Result<Option<String>>`；未知服务不再与“已知服务未配置认证”共同返回 `None`，Capability 装配必须传播配置错误。
439. Capability 客户端、远程 HTTP Client 和 `/shutdown` 请求统一调用同一 token owner，删除三套不同的环境优先级、空值和错误语义。
440. Runtime 删除仅有一个生产调用、主要由测试维持的 `build_remote_service` Facade；远程配置在真实 `remote_service` 决策点直接构造。
441. Loopback 共享 token 不再由 Facade 对所有“空安全配置”隐式补入，改为仅在实际 URL 为 loopback 且缺少部署凭证时显式选择该合同。
442. 删除单调用 `remote_security_env_prefix` 及只验证字符串拼接的测试；服务级远程凭证前缀在唯一环境装配点直接派生。
443. Loopback 凭证判定从 `RemoteSecurityConfig::is_empty` 改为 `has_deployment_credential`；仅配置 CA 根证书不再被误当成客户端身份并绕过 token 要求，完整 mTLS 或 token 才属于有效部署凭证。
444. Service Manager 新增唯一严格 URL owner，统一完成远程 URL 规范化、HTTP/HTTPS 限制、凭证/query/fragment 拒绝、loopback 判定及非本地 HTTPS 强制规则。
445. Runtime 删除私有远程 URL parser、测试 re-export 和镜像测试；Sidecar 环境 URL、配置校验与密钥转发策略统一消费 Service Manager owner。
446. 请求上下文删除手写 scheme/authority/IPv4/IPv6/端口解析器及 `secrets/url.rs`；capability 到 service 与 URL 的映射也收敛为单一查询，调用者不再并行传入可分叉的 service name。
447. `ManagedServiceConfig`、`RestartPolicy`、`RemoteSecurityConfig`、`ManagedSidecarState` 与 `ManagedSidecarStatus` 删除无活跃序列化入口的 serde 兼容派生、默认属性和旧格式合同。
448. 删除仅为 serde 兼容存在的五个默认函数；启动超时与重启策略默认值直接位于唯一 `Default`/构造器，重启计数已由 Supervisor 保证从 1 开始，因此移除不可达的 `saturating_sub` 防御。
449. 删除无生产写入者和无业务消费者的 Sidecar `args` 配置、Debug 计数及 `Command::args` 启动传递；受管 Sidecar 启动参数只剩明确环境合同。
450. 删除 `ManagedServiceConfig.health_path` 及其默认、构造、Debug、测试分支；所有 A4N managed/remote 健康探测只使用 Service Protocol 的固定健康路径。
451. 删除无构造者、无消费者的 `CapabilityHealthSnapshot` 死类型及其序列化合同。
452. 管理员服务健康响应删除 `service_mode` 兼容别名，以及 `port`、`health_path`、`pid_source`、`status`、`restart_count`、`supervisor_state`、`last_error`、`error_code` 等无前端消费者字段。
453. 删除管理员健康摘要第二套 URL 脱敏、端口手写解析与无效 URL 容错，移除整个 `service_health/url.rs`；展示层直接消费已通过 Service Manager 严格校验的 URL。
454. 数据库健康检查收敛为 `Result<()>`；管理员响应删除永远为真的 `database` 与无人读取的 `backend` 字段，健康命令只在数据库失败时显式报错。
455. 前端管理员健康合同改为严格 runtime profile/service mode 枚举和必填字段；删除未知值、旧字段、可选数组、空值拼接及错误码回退。
456. `ManagedSidecarStatus` 删除仅由测试读取的 `service`、`restart_count`、`next_restart_in_ms`、`last_error`，并删除重复状态 getter 与 PID getter；Supervisor 仅发布生产实际消费的 state/pid，重启测试改以 Manager PID 变化验证真实重启。
457. 删除无生产写入者的 Remote Sidecar PID 文件观察链、跨平台进程探测、`ManagedServicePidStatus`、`service_pid_dir` 上下文参数及相关临时文件测试；PID 唯一来自 Service Manager 持有的子进程状态。
458. Linux/Windows release smoke 删除自造过期 PID 文件、PID 文件数量/内容/清理断言和 PID 文件兜底杀进程；Sidecar 生命周期继续由真实健康端点、Host 退出和端口释放验证。
459. 服务健康摘要删除成功/失败两套重复结构体装配，先投影差异字段再构造唯一响应，避免合同字段后续在两条分支中漂移。
460. Sidecar Bootstrap 新增唯一有界事件通道 owner，统一 `SIDECAR_EVENT_QUEUE_CAPACITY`、Receiver 到 NDJSON 字节流的映射和换行序列化；Image、Video、RAG、Embedding 删除四套相同 channel/unfold 实现。
461. Sidecar Bootstrap 新增唯一失败事件响应 owner，统一 sequence=1 的 failed 事件、单事件流和 NDJSON Response 装配；Image、Video、RAG、Embedding、LLM 五个入口删除重复组合。
462. 删除 Embedding `failed_response`、LLM `failed_stream_response` 两个单调用 Facade，并将已无跨 crate 消费者的 `single_service_event_stream` 内联后移除，避免为一行组合保留公共/私有双层 API。
463. `CapabilityTaskEvent` 新增唯一 `progress_message` 构造入口；Image、Video、RAG 不再分别维护 Progress kind 与 `{message}` payload 结构。
464. Image、Video、RAG progress payload 删除生产消费者从未读取、仅由测试夹具维持的 `current`/`total` 字段；RAG 同步删除只为 `total` 存在的 `record_count` 参数和旧夹具字段。
465. `CapabilityTaskEvent` 新增唯一 typed completed 构造入口，统一 Completed kind、payload 序列化和脱敏错误合同。
466. 删除 Image/Video/RAG 六个单调用 progress/completed builder、对应测试 re-export 与镜像单测；Image 的 `handlers/events/projection.rs` 整模块退役，真实事件流直接消费协议 owner。
467. `CapabilityTaskEvent` 删除公共通用 `new`、`with_payload`、`with_serialized_payload` builder，生产者只能通过 Progress、Chunk、Completed、Failed 的语义构造入口装配事件，避免任意 kind/字段组合形成第二合同。
468. typed Completed 收敛为显式 `completed_or_failed`，payload 序列化失败直接生成同 sequence 的 Failed 事件；Image、Video、RAG、Embedding 删除四套重复的 Result match 与失败事件重建。
469. LLM heartbeat、Chunk 和空 Completed 分别改用协议语义构造器，删除生产代码最后三处手写 event kind；Bootstrap NDJSON 测试同步删除单调用 `test_event` helper。
470. LLM `stream_with_provider` 删除外层 `EventLineStream` trait-object alias 和第二次 `Box::pin`；状态机本身已返回静态 `impl Stream`，只保留枚举内部真正需要类型擦除的 provider/chunk Future。
471. `DownloadProgress` 增加严格 `Deserialize + deny_unknown_fields`，Embedding Runtime 直接消费共享类型；删除 `SidecarEmbeddingDownloadProgress` 镜像结构及逐字段转换实现。
472. Embedding 删除两个单调用事件 builder、测试 re-export、两项镜像单测及整个 `handlers/events/projection.rs`；真实下载流直接使用协议构造器。
473. Embedding Completed payload 删除只在 producer 包一层、consumer 立即拆除的 `{models: ...}` 内部信封；事件 payload 直接承载模型数组，不再维护第二层动态字段合同。
474. Image 生成消费链删除单调用 `handle_image_generation_event` 和整个 `sidecar_clients/image/events.rs`；唯一调用点直接消费共享 `parse_generation_event`，并删除只测试该 Facade 的终态镜像测试。
475. Video 删除单调用 `handle_video_generation_event`，生成循环直接消费共享 parser 并保留自身进度通道语义；同步删除与 Image 重复的终态 Facade 测试。
476. RAG ingest Runtime 删除第四套 Progress/Completed/Failed 分派、message 校验、payload 反序列化和取消错误映射，直接复用 Image/Video 已使用的 `parse_generation_event`。
477. Image 与 Video provider 事件执行删除“外层事件任务再 spawn 内层 provider 任务”的双任务结构；`select!` 直接持有 pinned provider Future，断连时丢弃 Future 即完成同等取消，JoinError 分支一并退役。
478. Embedding 下载同样删除额外 Tokio task 与 JoinError 转换，进度循环直接 select pinned 下载 Future；取消 token 后仍等待真实下载 Future 收尾，不新增回退路径。
479. Embedding 模型列表不再提前转成 `serde_json::Value` 再交给事件层；typed 模型列表直接进入统一 completed serializer，删除一次无意义 JSON 中转。
480. Server 在构建 `AppState` 和启动 managed Sidecar 前先绑定监听地址；端口不可用时不再先启动数据库迁移、后台任务和 Sidecar 后依赖进程退出清理。
481. Server graceful shutdown 在 Axum 退出后显式调用 `AppState::shutdown_managed_sidecars`，与 Tauri 共用唯一优雅关闭入口；CLI 的 TERM 生命周期不再只依赖 `Drop` 强制停止。
482. Runtime 将实际被关闭方法读取的 `_managed_sidecars` 更名为 `managed_sidecars`；BFS 确认其 `Arc + Mutex` 分别承担跨 `AppState` 克隆共享和幂等 `mem::take`，因此保留而不做错误简化。
483. `HttpSidecarTransport` 构造器改为必须接收注册服务名，并直接从 Service Protocol Registry 取得 capability version 与资源策略；typed Client 不再先构造匿名传输对象。
484. Transport 的 `expected_service_name`、`expected_capability_version` 从可选状态改为必填静态字段，删除 `with_expected_service` 两阶段装配及“忘记补期望值”的无效状态。
485. 删除只为可选期望值服务的 `validate_health_expectation` Facade 与对应伪安全测试；健康校验只走真实 `ServiceHealth` 合同。
486. 未注册服务不再继续套用全局 timeout/resource 默认值；typed HTTP Client 构造期直接失败，避免未知服务伪装成可用能力。
487. Prompt、LLM、Embedding、RAG、Image、Video、Import、Export 八类 HTTP Client 均直接以协议注册表服务名构造 Transport，服务身份与 capability version 不再由各 Client 重复维护。
488. 删除约百行只镜像八个 Client 服务名/version 的构造器测试；注册表与 Transport 构造边界已成为唯一可执行合同。
489. 收紧 Transport、Client 与健康校验内部字段可见性，只暴露测试和相邻模块真实需要的最小表面，删除跨模块写入内部状态的可能。
490. Service Protocol 新增唯一 `SIDECAR_EVENT_CONTENT_TYPE = "application/x-ndjson"`，删除 Bootstrap producer 与 Capabilities consumer 的重复媒体类型字面量。
491. Sidecar Bootstrap 的 NDJSON 响应装配与 Capabilities 事件流消费共同使用该 Content-Type owner，生产者和消费者不再维护平行协议常量。
492. HTTP 事件消费在读取 body 前严格验证 Content-Type；普通 JSON、缺失类型或变体响应不再被当作 NDJSON 继续解析。
493. 事件流 EOF 不再接受“最后一条 JSON 没有换行”的兼容格式，残余非空白字节直接作为不完整 NDJSON 失败。
494. 事件流在 Completed/Failed 终态前结束时由传输层直接失败，不再让上层根据 channel 静默关闭猜测任务结果。
495. 空白 NDJSON 行不再静默跳过；每一行必须是完整 `CapabilityTaskEvent`，删除宽松输入分叉。
496. `drain_event_lines` 改为单次游标扫描并在批次结束后一次 drain，删除逐行临时 Vec 分配与反复移动剩余 buffer。
497. 删除 `a4n_capabilities::EVENT_QUEUE_CAPACITY` 兼容别名，全仓事件通道统一使用协议常量 `SIDECAR_EVENT_QUEUE_CAPACITY`。
498. 删除 `SidecarResourcePolicy.event_queue_capacity` 无消费者字段；事件队列容量只由传输协议 owner 声明，不再伪装成逐服务资源差异。
499. `ServiceRequest<T>` 与 `ServiceResponse<T>` 增加 `deny_unknown_fields`，Host/Sidecar Envelope 不再忽略未声明顶层字段。
500. `ServiceHealth` 与 `ServiceVersion` 同样改为严格未知字段拒绝，健康握手与业务请求/响应使用一致的封闭合同。
501. Prompt 进程测试删除“未知 Envelope 字段应被接受”的旧断言，改为与全部 payload 严格边界共用唯一未知字段拒绝请求实现。
502. 未知字段进程检查删除“HTTP 成功但 body 内含 error 也算拒绝”的假成功回退；JSON extractor 边界必须返回明确 4xx。
503. `service_for_capability_version` 从 capability 名称/major 解析改为注册表精确值匹配，`.1` 后缀不再自动冒充当前能力合同。
504. 删除整个 `compatibility.rs`、`ServiceCompatibility`、`VersionRange`、major/name parser 及范围协商错误层；仓库不存在第二套版本判断算法。
505. `ServiceVersion` 删除可选 `compatibility` 信封和按 Host header 条件附带该字段的分叉，健康响应固定为 service/protocol/capability 三项版本事实。
506. `validate_service_health_envelope` 删除范围协商、未知服务零版本 fallback 与 advertised range 二次校验，直接执行服务身份、健康状态、精确 protocol 和精确 capability 校验。
507. 删除 `x-a4n-host-protocol-version` 常量、Bootstrap header 宽松解析器及其测试；Host 不再通过请求头触发不同健康响应格式。
508. 八个正式 Sidecar 健康入口删除 `HeaderMap` 参数并共用唯一 `known_service_health` 构造器，健康路由不再按调用方版本分叉。
509. Capability Transport 与 Service Manager 健康请求删除版本头注入，二者只消费同一个严格 `ServiceHealth` 合同。
510. 删除 35 分钟 Historical sidecar version matrix CI job、历史 checkout/build 脚本及 current/legacy probe crate；不再为已退役旧协议构建两套仓库和四套 target。
511. Sidecar security 静态检查删除“必须保留 compatibility 字段”的旧架构 gate，避免脚本反向阻止当前唯一合同清理。
512. 协议、Manager、Capabilities、Demo、Orchestrator 与进程夹具同步删除 compatibility 字段和 additive acceptance 断言；保留的测试合同全部指向精确版本与严格字段。
513. Capability HTTP 普通请求删除 `post_json`/`post_typed` 双入口，Prompt、LLM、Embedding、RAG、Image、Video、Import、Export 全部调用唯一泛型 `post<I, O>`。
514. typed 请求不再先拆 Envelope、把 payload 转成 `serde_json::Value`、再重新装回 `JsonRequest`；完整 `ServiceRequest<I>` 只序列化一次进入 HTTP body。
515. typed 响应不再先解析成 `ServiceResponse<Value>` 再逐字段重建 `ServiceResponse<O>`；reqwest 直接反序列化严格目标类型，删除一次动态 JSON 中转和整份响应复制。
516. 新的唯一 `serialize_request` 在序列化前继续执行共享 request context 校验，并将序列化失败明确映射为不可重试配置错误；未用网络错误掩盖本地请求错误。
517. 删除 `into_json_request`、`from_json_response` 及三项只证明 serde 字段搬运的镜像测试；保留一项真实边界检查验证未脱敏上下文在传输前失败。
518. `ServiceRequest::context` 与 `into_context_and_payload` 经全仓 BFS 仅剩测试调用后退役；Prompt 进程 fixture 在测试边界直接完成类型转换，生产协议不再暴露无消费者拆包 API。
519. Image/Video request preview 直接返回 provider 原始 body，删除 Sidecar 专用的 `{ "request": ... }` 包装和 Image/Video 各自的单调用 preview helper；上层统一 `request` 字段只装配一次。
520. 沿全部 Image/Video provider preview 实现确认没有合法 `null` 生产者，删除 Runtime Sidecar preview 的两处 `Value::Null` 伪安全回退；信封缺失或错误继续由统一 `response_payload` 显式失败。
521. LLM、Image、Video 的 Runtime model discovery 收敛到 `sidecar_clients::model_discovery::discover_models`，profile 分支、请求构造、Transport 错误映射和 typed response 解包只保留一份。
522. `ModelDiscoveryKind` 从 config action 移到 model discovery owner，三类 service capability version、Sidecar 名称和用户标签由同一枚举映射；删除错误挂在 LLM 模块下的通用 `model_discovery_payload` 与三份 service wrapper。
523. `ServiceRequest::validate_known_provider_request` 成为“已知服务 + provider API key secret policy”的协议层唯一校验入口；Image、Video、Embedding、LLM 删除各自重复的组合 helper，LLM 删除只覆盖同一协议规则的三项本地 policy 测试。
524. HTTP Video、New API Video、Seedance 2.0 Video 删除仅转发 `request_body` 的 preview 方法，factory 直接调用各 provider 的真实 body owner。
525. Stable Diffusion Image 删除仅转发 `txt2img_body` 的 preview 方法，Image factory 直接复用真实请求 body builder；参考图拒绝和字段构造不再有别名层。
526. Video embedded provider preview 删除 `base_url.as_deref().unwrap_or_default()`，直接沿配置边界报告缺失 `base_url`；provider 构造器不再接收人为制造的空字符串。
527. Embedding Sidecar payload 删除 API backend 的 `base_url.unwrap_or_default()`，缺失值在唯一 payload owner 直接报告配置错误。
528. Embedding 动态 provider factory 删除 API backend 的 `base_url.unwrap_or("")`，真实 HTTP provider 只接收明确存在且非空的 Base URL。
529. OpenAI-compatible Image 将 generations 参考图拒绝收敛到 `validate_generation_request`，embedded generate 与 request preview 不再维护两份相同输入校验和不同错误文案。
530. `dynamic_embedding::required_text` 改为唯一接收 `Option<&str>` 的必填文本校验 owner，动态 provider 与 Sidecar payload 直接传入可选配置，不再先构造空字符串。
531. Embedding Runtime 的模型操作、连通性和下载事件统一通过 `required_embedding_client` 获取 Sidecar Client，删除重复的可选能力解包和缺失错误构造。
532. 章节生成 RAG 的 query vector 数量、维度和有限数校验改为复用增强搜索的唯一校验 owner，删除一套相同的本地实现及其镜像测试。
533. 章节生成 RAG 的 embedding identity 比对改为复用增强搜索的唯一校验 owner，删除本地等值判断和重复错误格式化。
534. `GenerationRagQueries::is_empty` 经全仓 BFS 无生产或测试调用，直接删除未使用方法；空查询路径继续由实际 `weighted_queries()` 结果控制。
535. `build_generation_rag_context` 删除调用方重复传入的 `project_id` 参数，统一从已准备的搜索合同读取项目身份。
536. 增强 RAG 两条执行路径删除空查询早返回和单用途空结果 helper，统一由已有结果构造器消费空 batch；空结果行为不再有第二份结构装配。
537. RAG 同步的客户端 embedding identity 检查改为调用增强搜索校验 owner，仅在 `RagSyncError` 边界转换错误类型，删除第三套相同等值逻辑。
538. 删除 `OrchestratorContext::embed_and_store_rag_record` 只转发一次的公开别名，内部 `RagSyncContext` 直接调用同 crate 的 `rag_embed_and_store_record` owner。
539. 删除 `OrchestratorContext::embed_and_store_rag_records` 只转发一次的公开别名，批量同步直接调用 `rag_embed_and_store_records` owner。
540. RAG write 的 `embedded`、`provider`、`records` 子模块及低层 store 函数收窄为 crate 内可见；跨 crate 的活跃入口只保留客户端上传所需的 `store_embedded_rag_records` 方法。
541. Provider 写入、客户端向量写入和单条写入统一通过 `write::records::embedding_record` 装配 `EmbeddingRecord` 与规范化 metadata，删除三处重复字段构造。
542. `rag.rs` 的低层写入函数 re-export 同步收窄为 crate 内可见；全仓 BFS 未发现外部生产消费者，公开 API 不再暴露存储实现细节。
543. 增强 RAG 的融合函数与 identity/vector 校验函数收窄为 crate 内可见；Runtime 只需要查询规则与结果合同，不再暴露无外部消费者的实现/校验 API。
544. `a4n_rag_core::metadata::metadata_i64` 成为 JSON 整数元数据的唯一解析 owner，章节生成与增强融合删除两份相同的 `as_i64/as_u64` 解析实现。
545. 删除增强搜索模块中没有调用者的融合函数 re-export，并将实际融合函数与校验函数统一标为 crate 内实现，避免保留无效公开门面。
546. 章节清理触发的伏笔 RAG 重入由逐条“读取上下文 + 单条 embedding”改为一次 `RagSyncContext::for_ids` 与一次 `store_prepared_rag_records` 批量写入；单条实时同步入口仍保留其独立调用语义。
547. `build_generation_rag_context` 继续删除重复的 `chapter_number` 参数，项目与章节身份现在都只从 `PreparedGenerationRagSearch` 读取。
548. 章节 RAG 记录构造、章节实时同步和章节删除过滤统一复用 `chapter_rag_chunk_prefix`，删除三处独立拼接 `chapter:{project_id}:{chapter_id}:` 的分叉实现。
549. `RagSyncContext::sync_record` 改为接收记录所有权，单条同步不再为进入唯一 prepare/store 流程复制完整 `RagRecord`。
550. 章节同步直接以 `records.len()` 作为成功处理数，删除与实际记录集合同步递增的可变计数器。
551. `RagSyncContext::for_ids`、`for_id_prefixes` 与 `sync_record` 经全仓 BFS 仅有 Orchestrator crate 内消费者，收窄为 crate 内 API。
552. 角色状态/属性事件同步、伏笔批量重入以及章节/角色状态/属性事件/主角 RAG 删除方法均无跨 crate 消费者，收窄公开可见性；Runtime 活跃使用的伏笔实时同步和删除入口继续保留。
553. RAG chunk key helper 改为 crate 内事实源，并将 `rag.rs` 的同步模块 glob re-export 替换为显式公共合同与 crate 内 key re-export，避免低层键生成器泄漏为外部 API。
554. RAG search 的 glob re-export 改为显式公开查询合同；`MAX_RAG_TOP_K` 与 identity-scoped filter 按实际调用范围收窄，Runtime 只看到其真实使用的查询类型和规则。
555. 客户端 RAG rebuild 路径不再读取随后必然忽略的旧项目 RAG metadata；`ClientIngestContext` 用是否存在 metadata 唯一表达“增量比对/全量重建”状态。
556. 客户端 ingest context 删除向 `current_embedding_identity` 传递的多余二级引用，沿真实 Orchestrator 引用直接调用。
557. 待上传记录表不再同时保存 map key 与 `RagRecord.chunk_id` 两份相同字符串，value 只保留最终写入需要的文本、metadata、hash 和 outcome。
558. 删除 pending map 插入后的 canonical chunk_id 二次重复检查；`ExpectedRagRecords::add_rag_record(s)` 已在唯一构建边界严格拒绝重复，外部上传 chunk_id 的独立重复校验仍保留。
559. 删除上传循环结束后的“缺少 prepare 记录”不可达分支；上传数量相等、chunk_id 唯一且每项必须命中 pending map 三项边界已共同保证 map 必为空。
560. 客户端增量上传删除 stale ID 的外层空数组分支，统一调用已有 `delete_rag_records` 空输入 owner，删除数量统计继续严格执行溢出检查。
561. `ExpectedRagRecords.ids/records` 改为私有并仅提供消费式 `into_records/into_parts`，使所有调用方都无法绕过唯一重复检测构建边界直接改写 canonical 集合。
562. `RagEmbeddingInput` 经全仓 BFS 仅在 Orchestrator 内部 provider 写入链使用，收窄类型、字段及 re-export；跨 crate 客户端上传仍只暴露必要的 `RagEmbeddedInput`。
563. `RagIngestTaskScope` 与内部 `ExpectedRagRecords` 的 `Clone` 经全仓 BFS 无调用，删除无用深复制能力，任务范围保持单次消费语义。
564. RAG key owner 从 `rag/sync/keys.rs` 提升到 `rag/keys.rs`，删除同步子模块对记录构造规则的错误所有权；顶层模块只按真实调用范围 re-export crate 内 helper。
565. 新增唯一 `rag_chunk_id`/`rag_project_prefix`，统一使用 `RagDataType::as_str()` 生成前缀；Blueprint、Blueprint Metadata、World Setting、Relationship、Character、Outline、Part Outline 与三类 Scene 记录删除手写类型字面量拼接。
566. 章节、角色状态、角色属性事件、伏笔和主角档案的 exact ID/prefix 继续由同一 key owner 派生；记录构造、同步范围和删除范围不再各自维护平行格式。
567. `chapter_rag_records_with_provider` 及 `chapter_records::records` 经跨 crate BFS 无外部消费者，收窄为 crate 内实现；外部只保留真实使用的章节记录入口与 chunking 合同。
568. `DeleteRagRecordsInput.data_type` 删除；所有活跃删除调用已明确提供 canonical exact IDs 或 ID prefixes，不再同时传递一份可能分叉的 metadata 类型提示。
569. `DeleteFilter` 删除 `project_id`、`data_type`、`chapter_number`，Host、Sidecar、Memory 与 Qdrant 共用的严格 serde 合同仅保留 exact IDs 和 ID prefixes；旧字段 payload 不增加兼容解析。
570. Memory 删除匹配删除 metadata 条件分叉，统一执行 exact ID/prefix 并集；项目隔离继续由 collection owner 保证，metadata 缺失或不一致不再阻止 canonical ID 删除。
571. Qdrant 删除与 metadata 读取同步退役 project/type/chapter 条件，统一由 physical collection 加 exact ID/prefix selector 决定记录范围。
572. 章节删除方法删除重复的 `chapter_number` 参数，章节记录构造、实时同步和清理均使用 `chapter_rag_chunk_prefix(project_id, chapter_id)` 唯一事实源。
573. Memory 的空 prefix 不再被解释为“无过滤条件”而删除全部记录；空 selector 仍表示完整 collection，非空但无效的 prefix 只匹配零记录。
574. Qdrant live 删除样例改为 exact ID，RAG 独立进程 fixture 删除已退役的 metadata filter 字段；旧“非正章节号删除过滤器”镜像测试随旧合同退役。
575. `RagSyncError::InvalidDeleteInput` 经全仓搜索只剩枚举声明后删除；空删除输入继续由唯一公开删除入口幂等返回 0，不再保留不可达错误分支。
576. 删除单调用 `delete_rag_records_with_filter` Facade，公开删除入口直接调用 project store owner 并映射唯一 `RagSyncError::Delete`。
577. Qdrant 修复 exact IDs 与 prefixes 同时存在时静默忽略 prefixes 的实现分叉；两类 selector 现进入同一个 OR group，删除与 metadata 读取共用唯一 `delete_filter` 构造器。
578. Qdrant 顶层 `project_id` payload、提取器和 field index 经 BFS 已无查询消费者后删除；canonical `metadata.project_id` 继续用于业务过滤，项目存储隔离由 physical collection 保证。
579. 每个 logical collection 已映射到独立 physical collection，故删除 point payload 中重复的 `logical_collection`、全部永真查询/删除条件及记录构造参数；collection 隔离只保留物理映射一层。
580. 删除从 `VectorStore`、Memory/Qdrant、RAG Sidecar JSON、Runtime Client、Disabled/Test Store 到调用样例整链透传但始终被忽略的 `vector_weight` 假配置；混合检索只声明实际执行的 RRF 语义。
581. 删除 `a4n_rag_core::fusion::rrf_fuse`、`FusedEntry`、公开 `RRF_DEFAULT_K` 及两项自测；该实现无生产消费者，Memory 的 score-only RRF 成为唯一活跃基础混合算法。
582. Memory RRF score map 删除写入后从不读取的原始结果分数字段，三元组收敛为 vector/BM25 两项真实融合状态。
583. Memory RRF 删除唯一调用方固定传入的 `k` 参数，以模块内 `RRF_K` 声明实际算法常量；不再暴露不存在的运行时可调分叉。
584. RAG Core 文档删除已退役 LanceDB/Tantivy 实现描述及与当前后端不符的 prefix 解释，合同注释只保留字段真实语义。
585. 本机增强检索 prepare 响应删除前端从未读取的 `queries`、`top_k`、`pull_k`，仅返回本机 embedding 真正需要的 canonical `query_texts` 与 `embedding_identity`。
586. Host `RagSearchResult` 删除前端及其他生产消费者从未读取的 `vector_score`、`bm25_score`；VectorStore 内部融合仍保留算法实际需要的分项分数。
587. `RagEnhancedQuerySummary` 删除仅用于旧展示合同、前端不读取的 `weight`；查询权重只留在 Orchestrator 融合边界。
588. 增强查询展示从机器 `kind` 与前端二次翻译改为后端唯一生成的中文 `label`，删除 `formatQueryKind` 及未知 kind 的“其他查询”回退。
589. `PreparedEnhancedRagSearch` 与执行链改为消费所有权，最终结果直接移入查询集合；删除 prepared 借用、查询深复制和由此保留的 `EnhancedRagQuery::Clone` 需求。
590. `PreparedEnhancedRagSearch` 持有 canonical `project_id`，两条执行函数删除调用方重复传入的项目参数，collection 与 filter 均从同一 prepared scope 派生。
591. 删除 Runtime `rag_search/api/execution.rs` 及两个单调用执行 Facade；Host 三个命令直接串联 prepare、Orchestrator execute 与 output owner。
592. 删除 `rag_search_result`、`rag_enhanced_query_summary` 两个单调用 DTO converter，唯一 output owner 直接消费 domain result 并装配 Host 合同。
593. Runtime Host 删除一次性 `rag` 局部变量、多余二级引用和 prepare 输出的查询字符串复制；本机 prepare 直接消费查询所有权。
594. `pull_k` 删除为 prepared 派生状态，执行时唯一通过已验证 `top_k` 计算；`enhanced_rag_pull_k` 同步收窄到增强检索模块内部。
595. 删除只剩 API 类型 re-export 的 `rag_search/contracts.rs`；Runtime 顶层模块直接公开实际 Host 合同，不再保留纯转发模块。
596. `RagQuickPanel` 将 results、query summaries、candidate count 三份总是同步写入/清空的 state 收敛为一个 `RagEnhancedSearchOutput` 原子状态。
597. `RagQuickPanel` 的过期请求判断统一使用现有 sequence owner，删除与 sequence 同步更新的 mounted、scope、query 三份镜像 ref 及三组重复比较。
598. 删除禁用按钮已保证不可达的空查询 toast 分支、卸载时无人读取的 busy-ref 赋值和只转发一次的 request scope 局部变量。
599. 客户端 query vectors 只在增强检索/章节生成的共享批量边界校验数量、维度和有限数；删除每条查询进入底层搜索时的第二遍向量校验及重复 identity 参数。
600. 底层 caller-embedding 搜索删除单调用 `execute_hybrid_rag_search` 包装，统一为明确要求前置校验的 `hybrid_rag_search_with_validated_embedding_in_store` 唯一 store 入口。
601. Provider 查询向量在 `validate_embedding_batch(..., 1, ...)` 成功后直接按已验证单元素不变量提取，删除随后不可能触发的第二个“未返回向量”错误回退。
602. 增强融合删除 `chapter_number > 0` 二次容错过滤；Host preparation 已严格拒绝非正章节号，融合层只区分明确章节与项目级 `None`。
603. `EnhancedRagQuery.weight` 收窄为 crate 内融合状态，并删除无调用的 `Clone` 能力；跨 crate 只暴露 prepare/output 实际读取的 label 与 query。
604. 前端 RAG barrel 删除不再被组件直接消费的 `RagEnhancedQuerySummary` 命名导出，改为公开原子 `RagEnhancedSearchOutput` 与结果合同。
605. 删除 `GenerationRagStatistics` 及其全部无生产消费者字段；章节生成只消费最终上下文文本，不再维护 chunk、summary、continuity、query 与字符数镜像统计。
606. 删除单调用 `rag_context/projection.rs`；结果融合后直接进入 formatter，`format_generation_rag_context` 直接返回真实消费合同 `Option<String>`。
607. 删除只包装 `text` 与退役统计的 `GenerationRagContext`；Store、Orchestrator context 与章节生成应用链统一直接传递 `Option<String>`。
608. 删除 `context_text_length` 派生状态及只验证统计镜像的测试断言；字符限制继续由 formatter 的唯一输出边界执行。
609. 章节生成 RAG contracts、query、fusion、rules 与顶层 re-export 按真实调用范围收窄为 crate/module 可见，删除无外部消费者的公共 API 面。
610. `GenerationRagQueries` 与 `WeightedGenerationQuery` 删除无调用的 `Clone`/`Debug` 能力，prepared 查询保持单一所有权和借用视图。
611. formatter 删除与 section 始终同步的 `is_summary` 状态；每个结果只保留唯一 `GenerationRagSection` 分类。
612. 结果 metadata 的 `data_type` 统一通过 `RagDataType::parse` 解析一次，并由该枚举同时决定允许类型、section 与展示标签，删除三套字符串分类。
613. 章节生成搜索 filter 明确只允许 Chapter、Summary、Metadata、KeyEvent 与两类 Character 状态；删除 formatter 中真实查询不可达的 Foreshadowing 分支。
614. 删除未知 RAG 类型的“参考”展示 fallback；未知或不属于章节生成合同的类型在严格枚举解析/section 映射处直接排除。
615. 删除 formatter 的第二次 result ID 去重；共享 weighted fusion 已按 ID 生成唯一结果，投影层不再重复维护集合状态。
616. 删除 formatter 对非有限融合分数静默省略相关度的回退；输出分数唯一来自有限 RRF 常量、查询权重和时间/类型权重。
617. weighted fusion 删除不影响排序结果的原始 source score、`best_source_score` 与“最高原始分结果替换”副本逻辑；重复 ID 只累加真实 RRF contribution。
618. 删除 `ensure_rag_scores_finite` 及只验证旧 source-score 镜像的测试；融合不再消费后端原始 score 数值，只消费其已确定的排名顺序。
619. Core `SearchResult` 删除全仓无生产读取者的 `vector_score`、`bm25_score`，Memory、Qdrant、Orchestrator、Sidecar 与 Host 统一只保留最终排序 `score`。
620. Memory RRF 删除 `FusedScore` 及 vector/BM25 双分量输出，唯一融合结果收敛为 `(id, fused_score)`，调用方直接从 authoritative collection 回填内容和 metadata。
621. Qdrant sparse 搜索删除先写入 `vector_score` 再搬运到 `bm25_score` 的字段互换循环；dense、sparse 与 native hybrid 统一直接映射 Qdrant 最终 score。
622. Memory、Qdrant、Orchestrator 与测试支持中的 `SearchResult` fixture 同步删除退役双分数字段，不保留 serde 兼容字段或占位值。
623. `GenerationRagQueries` 的 main/character/foreshadowing/location 四组分叉状态收敛为唯一 `Vec<GenerationRagQuery>`，每项直接持有 canonical text 与 weight。
624. 删除 `WeightedGenerationQuery` 借用投影及每次 fingerprint、校验、执行时重建加权列表的重复分配；所有消费者直接遍历 prepared 唯一查询序列。
625. 合并 `prepare_chapter_generation_rag_queries`、`prepare_chapter_generation_rag_search` 与 application 纯转发 helper；待处理伏笔、查询构造、identity 校验和 filter 装配只剩一个 preparation 入口。
626. 第一章本机 prepare 分支删除只为 fingerprint 创建的空 `GenerationRagQueries` 占位对象，直接以空查询切片构建严格 fingerprint。
627. 本机 prepare 输出直接消费 prepared 查询所有权生成 `query_texts`，删除每条查询 `to_string` 深复制。
628. prepare 响应与正式章节上下文在最后一次使用 prepared 后直接移出 `embedding_identity`，删除两处为后续输出保留的完整 identity clone。
629. 删除前端 `shouldPropagateGenerationRagPrefetchError` 及按中英文错误片段猜测“可回退/不可回退”的脆弱分类合同。
630. 删除本机章节 RAG 预取的 try/catch 服务端回退；prepare、embedding、identity 或 vector 失败现在沿唯一失败路径显式返回，不再静默切换实现。
631. `activeLocalEmbeddingConfig` 已由本机设置保证至少一个且仅一个默认配置，删除不可达的“无默认 Embedding 则返回 null”分支及 nullable helper 返回类型。
632. 私有本机 RAG helper 只在第 2 章及以后调用，后端 outline 合同保证至少一个 main query；删除空 query list 时伪造 identity/空 embeddings 的旁路。
633. 删除前端 embedding 后重复检查“最多 7 组”；查询 owner 已截断为 7，客户端数量校验与后端 prepared expected-count 继续覆盖真实合同边界。
634. 查询主文本去重的候选数固定不足十项，删除额外 `HashSet` 状态，直接在唯一结果 Vec 上检查重复。
635. provider/vector 执行删除 prepared 空查询的提前返回分支；活跃 preparation 不产生空列表，即使内部收到空列表，空 batches 也由同一 fusion/formatter 路径自然得到 `None`。
636. 增强检索删除恒不生效的 `ENHANCED_RAG_MAX_QUERIES` 与最终 `truncate(8)`；查询结构本身最多只产生主查询 1 条、角色 3 条、伏笔 3 条和场景 1 条。
637. `filter_relevant_foreshadowing` 删除与唯一 query builder `.take(3)` 重复的结果截断；候选过滤只负责相关性，查询数量只由消费边界声明一次。
638. `EnhancedRagQuery` 与 `EnhancedRagSearchResult` 删除无生产或测试消费者的 `Debug` 能力；prepared/result 保持最小公开合同。
639. `required_top_k` 删除唯一调用方恒定传入的 `command` 假泛型参数，增强检索命令名由该规则 owner 的固定错误合同声明。
640. `required_nonblank` 删除唯一调用方恒定传入的 `field` 假泛型参数，查询字段名不再由调用点重复传递。
641. 章节生成客户端 query vector 的期望维度统一读取服务端 prepared embedding identity；客户端提交 identity 只作为待校验实际值，不再成为自身向量的第二个期望来源。
642. `RagIngestResult.chunk_count` 始终等于 `added_count + updated_count`，删除该镜像统计、重复溢出计数和前端“写入”重复展示；结果只保留新增、更新、跳过、删除四项事实。
643. 客户端 ingest prepare 删除结果完全未消费的 `root_text_family_metadata` 改写；metadata 规范化仍在真正持久化前的 upload 边界唯一执行。
644. 共享 `validate_embedding_count`/`validate_embedding_batch` 改为消费 `AsRef<[f32]>`，客户端上传校验只收集向量切片，不再深复制整批 embedding。
645. 上传记录删除与 `pending_by_id.remove` 重复维护的 `seen HashSet`；同一 chunk 第二次出现会由唯一待上传集合直接拒绝。
646. 删除仅为一次导入改名的 `ExpectedSyncStats as IngestStats` 别名，客户端上传直接使用真实统计类型。
647. 删除只包装 `Option<&dyn Fn>` 的 `RagProgressReporter` 类型别名，Server 与 Runtime 直接声明真实 reporter 合同。
648. 删除 `progress_stage`/`progress_advance` 两个可选进度转发门面；同步循环只在 progress 实际存在时构造逐条消息，内部无进度同步不再做无消费者字符串分配。
649. `RagIngestProgress::advance` 删除生产调用恒定为 1 的 `amount` 假泛型参数，进度语义收敛为每处理一条 canonical record 前进一步；同步删除只测试任意大步进的旧夹具。
650. `RagIngestProgress` 的 `stage`/`advance` 收窄为 Runtime crate 内能力；跨 crate 的 Server 只保留真实需要的构造入口。
651. `ExpectedSyncStats` 及字段收窄为 Runtime crate 内状态，不再通过私有模块链保留无外部消费者的公共能力。
652. `SceneRagSyncTraceSource` 删除无生产或测试消费者的 `Debug`、`Clone`、`Copy`、`PartialEq`、`Eq` 能力，枚举只承担一次性 trace source 分派。
653. `RagIngestTaskScope` 删除无消费者的 `Debug` 能力；跨 crate scope 只暴露任务登记和执行所需状态。
654. `ExpectedRagRecords` 删除无消费者的 `Debug` 能力，仅保留实际用于构建集合的 `Default`。
655. 增强检索三个 Host 入口的项目编辑权限校验收敛到唯一 preparation 入口，避免 prepare、服务端搜索和客户端向量搜索分别维护相同 access gate。
656. 客户端 ingest prepare 删除单调用 `_inner` 实现层，operation trace scope 直接包围唯一 prepare 流程，不再透传 state/user/project/rebuild/trace 五项参数。
657. 服务端全量 ingest 删除单调用 `_inner` 实现层，权限复验、rebuild、进度和同步直接位于 operation trace scope 内。
658. `ClientRagIngestPrepareOutput.rebuild` 删除请求原样回显；本机 upload 直接使用 `ragIngestAll` 调用参数，prepare 响应不再维护第二份重建状态。
659. Command manifest 测试删除对 `project_id`、`expected_count`、`skipped_count`、`rebuild` 等历史退役字段逐项“不应存在”的兼容 gate，只验证当前活跃命令合同。
660. `store_prepared_rag_records` 改为消费 prepared 所有权，批量 embedding input 直接移入 chunk_id、正文和 metadata；全量同步与伏笔批量重入不再深复制整批 RAG 记录。
661. `RagRecord` 与 `PreparedRagRecordSync` 删除因旧借用写入接口保留、现已无消费者的 `Debug`/`Clone` 能力。
662. `RagSyncContext::cleanup_stale` 改为消费 context，stale IDs 直接移入删除请求；删除 HashSet 全量 clone 和函数返回前无后续读取者的 `clear()`。
663. `DeletedCountOverflow.deleted` 直接保存真实 `usize`，删除只为错误展示提前创建的十进制 String。
664. `RagRecordSyncOutcome` 删除无集合/排序消费者的 `Eq` 标记，保留实际比较和复制所需能力。
665. `DeleteRagRecordsInput` 删除无生产或测试消费者的 `Debug`/`Clone` 能力，删除请求保持一次性所有权。
666. `ExpectedRagRecords::add_rag_record` 删除同一 chunk_id 的第二次 String clone；唯一 clone 直接进入去重 ID 集合。
667. 删除单调用 `append_story_records` 聚合门面，全量 expected-record builder 直接串联章节、提纲和蓝图三个真实阶段。
668. 角色与场景 expected-record 构造的 ID→名称表改为借用已加载模型中的 `&str`，不再复制全部角色名和场景名。
669. 全量章节 expected-record 构造只读取一次桌面/服务端 chunking mode，不再为每章重复读取并克隆 Runtime 配置快照。
670. `uses_client_desktop_embedding` 直接借用 task-local `RuntimeConfigSnapshot.client_context`，删除仅判断两个字段却克隆完整 snapshot 的路径。
671. 删除单调用 `chapter_rag_chunking_mode` helper，唯一章节记录 owner 直接选择两种明确部署模式。
672. 章节 paragraph metadata 删除无生产读取者的 `chunk_index`、`chunk_count`、`section_index`、`total_sections`、`semantic_chunked`、`sentence_count`、`density_score`、`start_sentence_idx`、`end_sentence_idx`；只保留过滤、展示和来源校验实际消费的字段。
673. `ChapterRagChunkingMode::is_semantic` 及无消费者的 `Eq`/`Debug` 能力随退役 metadata 删除；模式只用于选择真实分块实现。
674. 桌面章节分块用累计字符数替代每句两次扫描，并用 `mem::take` 移出完成 chunk，删除完整正文 clone 与重复 `chars().count()`。
675. `SemanticChunk` 删除无生产消费者的句子区间、句数和密度镜像状态，语义 DP 只输出实际下游需要的分块文本。
676. 删除永久固定 `with_overlap=false`、`overlap_sentences=0` 的配置字段和整条不可达重叠文本分支。
677. 语义分块删除本地 embedding 数量/维度/有限数校验实现，复用 RAG Core 唯一 `validate_embedding_batch` owner。
678. 删除 `chunk_text` 已保证后仍在私有 `chunk_sentences_with_embeddings` 重复检查空句集/最小句数，以及被最小句数判断完全覆盖的 `sentences.len() < 2` 分支。
679. 退役字段删除后 `SemanticChunk` 只剩单个 String，进一步删除该包装类型；Core、Orchestrator 与夹具统一直接传递 `Vec<String>`。
680. 删除只持有一个 config、生产中只构造一次的 `SemanticChunker` 对象；语义分块直接由两个私有函数执行。
681. `generate_chunks` 删除排序去重 cut-points 后不可达的 `start >= end` 分支，以及创建空 String 后立即 `push_str(join(...))` 的中转。
682. 删除全仓只有唯一 Default 值的 `RagChunkConfig`、`chapter_rag_chunk_config()`、`config.rs` 和七项常量镜像测试；跨桌面/服务端只保留共享字符上下限，语义参数归唯一算法 owner。
683. 超长句在找不到逗号等软边界时改为使用已有 hard boundary，删除整句原样返回和 `result.len() <= 1` 两级宽松回退；分句结果现严格受 150 字上限约束。
684. 基于固定句长上限确认任意最小三句最多 600 字、不超过 1200 字后，删除 DP“无合法字符范围候选则忽略字符限制再选一次”的回退分支。
685. DP 字符前缀按构造必然单调，删除不可达 checked-sub 错误路径；`block_score` 仅接收最小句数保证的正区间，删除 `end <= start` 假分支。
686. `split_novel_sentences` 直接使用标准 `str::lines` 处理 LF/CRLF，删除整篇正文 `replace('\r', "")` 分配。
687. 结构增强改为消费并原地更新 similarity matrix，删除原矩阵随后无人读取却完整复制一次的 `n×n` 状态。
688. cosine matrix 只保留每条 embedding 的 norm，直接从原 f32 向量计算点积；删除全部向量转 f64 并规范化后的第二套 embedding 矩阵。
689. similarity matrix 与二维前缀矩阵使用相同的行主序索引规则，删除内容完全相同的 `prefix_index`，统一复用 `matrix_index`。
690. DP backtrack 的 path 只可能是 `-1` 或已写入的非负句子索引，删除不可达的其他负值分支及非负值转 `usize` 的多余 `try_from(...).expect(...)`。
691. DP 字符前缀构造改为维护唯一累计字符数，删除每轮从 Vec 末尾取前值及“零起点必存在”的 `expect`。
692. `split_long_sentence_for_semantic` 只在调用方确认句长超过 200 后进入，删除函数内重复的“未超 150 则原样返回”分支。
693. 语义长句判定 200 字与 hard split 150 字收敛为句子拆分 owner 内命名常量，删除唯一调用方恒定传入的 `max_len` 假参数。
694. 章节分块测试名称与断言同步当前真实合同，删除 `chunk_count`、overlap 和已退役诊断状态的历史措辞。
695. 删除章节 RAG metadata 中无生产读取者、无协议或配置声明的 `source_chapter_content_sha256` 与 `source_chapter_updated_at`，以及逐记录 String clone 和镜像测试；记录更新判定继续只由规范化后的单记录 `content_hash` 与 index/embedding identity 负责。
696. 删除 RAG metadata 中始终等于 `content_hash` 的 `source_generation` 第二事实源、匹配 helper 和同步夹具字段；记录更新唯一使用 canonical `content_hash`。
697. 删除 RAG metadata 中固定为 `sha256` 且无读取者的 `hash_algorithm`；hash 算法由唯一 `content_hash` 实现 owner 决定，不再持久化常量镜像。
698. 删除 RAG metadata 中无生产读取者的 `content_length` 及镜像断言，正文长度不再随每条向量记录重复存储。
699. 蓝图正文与蓝图元数据记录删除已编码于 chunk ID 的 `blueprint_id` metadata，唯一保留实际搜索分类 `data_type`。
700. 世界观分段记录删除 `blueprint_id`、`field`、`section_index` metadata 及无意义 enumerate；蓝图、字段身份已由 chunk ID 唯一表达，字段标签已进入正文。
701. 角色关系记录删除 `relationship_id`、`character_a`、`character_b` metadata；关系 ID 已在 chunk ID，双方角色与关系内容已在可检索正文。
702. 章节大纲记录删除已编码于 chunk ID 的 `outline_id` metadata，保留章节范围过滤实际读取的 `chapter_number`。
703. 分部大纲记录删除 `part_id`、`part_number`、`start_chapter`、`end_chapter` metadata；ID 已在 chunk ID，编号与章节范围已进入正文且没有 metadata 读取者。
704. 蓝图角色与主角分段记录删除 source/profile/character/dimension/section 等镜像 metadata 及 enumerate；记录身份由 chunk ID 唯一表达，展示只保留实际读取的 `title`。
705. 场景档案记录删除 ID、类型、重要性和首末章节镜像 metadata；这些值已在 chunk ID/正文，前端标题只保留实际读取的 `name`。
706. 场景出场记录删除 occurrence/profile ID、场景名与顺序镜像 metadata；保留时间过滤实际读取的 `chapter_number`，其余内容已在 chunk ID/正文。
707. 场景位移记录删除 transition/from/to ID、顺序与 movement type 镜像 metadata；保留时间过滤实际读取的 `chapter_number`。
708. 角色状态记录删除 state/profile ID 与角色名镜像 metadata；保留章节生成实际读取的 `chapter_number` 与 `title`。
709. 角色属性事件记录删除 event/profile ID、角色名、类别、属性键与操作镜像 metadata；保留章节生成实际读取的 `chapter_number` 与 `title`。
710. 伏笔记录删除 ID、重复 setup chapter、payoff、状态、重要度与关联角色镜像 metadata；保留时间融合实际读取的 canonical `chapter_number` 与展示 `title`。
711. 章节段落、摘要和分析记录删除无任何过滤、格式化或展示消费者的 `content_kind` metadata；业务分类统一只使用 `RagDataType`。
712. `AnalysisRagItem` 删除只为嵌入 RAG metadata 构造的原事件/伏笔 metadata；同步删除事件 JSON clone、固定 pending 状态和关联字段复制。
713. `AnalysisRagItem.data_type` 从每项分配的 String 收敛为 canonical `RagDataType` 枚举，记录边界再读取唯一字符串表示。
714. `AnalysisRagItem` 删除无生产或测试消费者的 `Debug`/`Clone` 能力，保持一次性构造和消费。
715. 章节记录直接写项目族 canonical `source_chapter_id`，删除 `root_text_family_metadata` 中临时 `chapter_id`→`source_chapter_id` 别名转换及最终双字段持久化。
716. RAG metadata producer 删除与项目独立 collection 完全镜像的 `project_id`；项目存储范围唯一由 collection 参数表达，规范化边界不再注入第二份项目身份。
717. identity search filter 删除 `metadata.project_id` 等值条件；物理 collection 已隔离项目，filter 只保留 embedding identity、类型与章节约束。
718. 章节生成 formatter 删除 metadata `project_id` 复验及 context/result 两层项目参数透传；搜索已经在目标项目 collection 内完成。
719. Sidecar RAG 请求上下文改为从 top-level numeric collection 同时读取 project/root project ID，删除递归扫描任意 metadata/filter 项目字段、批内项目去重和多项目分叉。
720. 删除 RAG Core `metadata/family.rs`、项目族字段校验和 root/snapshot/derived 固定字段装配；活跃 RAG 记录不再持久化不存在生产者或消费者的项目族身份镜像。
721. 删除 Orchestrator `rag/sync/metadata.rs` 与 `root_text_family_metadata` 包装；同步准备只处理 content hash、索引版本和 embedding identity，不再改写业务 metadata。
722. identity 与章节生成 search filter 删除 root/snapshot/derived family 条件及重复项目参数；collection 成为项目范围唯一事实源。
723. `normalize_metadata`、写入 `embedding_record` 和 provider/embedded 写链删除只透传项目身份的参数，记录规范化只接收正文、调用方 metadata 与 embedding identity。
724. 删除 root/snapshot family metadata 测试夹具、无生产路径的快照搜索镜像测试和固定字段断言，不保留退役合同的兼容样例。
725. 测试支持 `embedding_record` 删除无消费者的 project/data type/chapter metadata 与两个假参数，只构造测试实际需要的 ID、向量、正文和空 metadata。
726. 清理同步、写入和章节生成测试中的旧 import、旧 helper 调用与旧参数数量；静态搜索确认退役 family 符号及旧签名无剩余引用。
727. Runtime RAG 请求构造删除“collection 缺失、非数字或非正数时静默降级为全局请求”的伪安全分支；所有生产 VectorStore 调用均来自正整数项目 collection，错误配置现在显式失败。
728. RAG Sidecar 七个普通 handler 改为直接移动 trace ID 与 payload，删除为响应装配保留 request 而复制整批 embedding、query、filter、ID 列表的做法。
729. 九个 RAG 请求 payload 删除仅由 handler clone 和测试 `expect_err` 支撑的 `Clone`/`Debug` 能力；反序列化测试改为直接检查错误值，不再反向要求业务 DTO 可调试。
730. `service_error` 改为消费已结束生命周期的 `AppError`，八个响应/事件调用点直接传唯一错误 mapper，删除完全相同的借用闭包。
731. ingest event 入口直接移动验证失败请求 ID、stream request ID/payload 与终态 task ID，只保留 progress 和终态两个真实事件之间必需的一次 task ID 复制。
732. mutation marker 删除从未被启动恢复、脚本、文档或测试读取的 operation 文件内容、参数与三处固定字符串；崩溃保护唯一依赖 marker 文件存在和持久化。
733. RAG Sidecar 删除缺少 `A4N_RAG_SERVICE_DATA_DIR` 时写入 CWD `data/` 的默认回退；managed source/portable 与独立进程入口均显式注入路径，漏配现在启动失败。
734. ingest completed 事件删除 `{written}` 单字段 DTO，Sidecar 与 Runtime 直接传递/解析真实 `usize` 写入数量。
735. delete 响应删除 `{deleted}` 单字段 DTO，handler 与 `VectorStore::delete` 直接共享 `usize` 合同。
736. 两条 metadata 响应删除 `{metadata}` 单字段 DTO，Sidecar 与 Runtime 直接共享 `HashMap<String, Value>` 合同。
737. drop-collection 响应删除空对象 DTO，使用协议已支持的 `()` 明确表达无返回数据，不再维护占位结构。
738. 删除 Runtime `rag/store/outputs.rs`、三项反序列化 wrapper 及只守护旧 wrapper 字段的兼容测试；四类 RAG 输出与已有 `CompletenessReport` 一致直接传递领域值。
739. RAG 独立进程和合同夹具改用数字项目 collection，删除记录中退役的 `metadata.project_id` 及旧 wrapper JSON 解包，样例只保留实际过滤字段和当前 wire shape。
740. 将 Provider 与 Host API 各自维护的 `EmbeddingModelIdentity` 收敛到 `a4n-domain::embedding` 唯一纯值合同；`a4n-embedding` 与 `a4n-api-contract` 只重导出 canonical 类型。
741. 删除 Runtime `provider_identity_to_contract`、`contract_identity_to_provider` 及本机 embedding 输出中的字段逐一重建；增强搜索和客户端 RAG ingest 直接共享同一身份类型。
742. 将 Host `LocalEmbeddingModelSummary` 作为 Domain canonical 值供 FastEmbed、Sidecar、Runtime 与 API 共同使用，删除字段完全相同的 `FastEmbedLocalModel` 第二实现。
743. 将 `EmbeddingModelDownloadInfoSummary` 收敛到 Domain，删除字段完全相同的 `FastEmbedDownloadInfo` 及其公开 re-export。
744. Runtime 本机模型列表和下载信息直接返回 FastEmbed 的 canonical Domain 值，删除 `to_local_embedding_model_summary` 与 `to_embedding_model_download_info_summary` 两个逐字段复制器。
745. FastEmbed 删除仅为两项退役镜像 DTO 序列化保留的 `serde` 直接依赖。
746. 将 `EmbedTextsPayload`、`EmbedTextsOutput`、`EmbeddingModelPayload` 收敛到 `a4n-service-protocol`，删除 Embedding Service `contracts.rs` 和 Runtime 中第二套 Sidecar DTO 定义。
747. 共享 Sidecar DTO 只保留真实需要的 Serialize/Deserialize，删除旧 Service DTO 的 `Debug`/`Clone`、Runtime DTO 的 `Debug`、可选字段冗余 `serde(default)` 及临时类型别名。
748. 五条 Embedding HTTP 路由收敛到 Service Protocol 常量，Service Router、Capability Client 和进程夹具不再各自维护路径字面量；删除 Capability 内私有 download-events 路径常量。
749. `A4N_EMBEDDING_CACHE_DIR` 收敛为 Service Protocol 跨进程环境常量，managed service 注入、Sidecar 读取与独立进程夹具共享唯一键名。
750. Embedding Sidecar 删除缓存环境缺失时写入 CWD `data/cache/embedding-models` 的默认回退；managed source/portable 与独立进程入口均显式注入路径，漏配现在启动失败。
751. 模型列表与下载信息 handler 直接返回 `Vec<LocalEmbeddingModelSummary>` 和 `EmbeddingModelDownloadInfoSummary`，删除两次 typed→`serde_json::Value` 序列化中转。
752. Runtime 模型列表和下载信息 Sidecar 调用删除无第二消费者的 `T: DeserializeOwned` 假泛型，函数签名直接表达唯一 Host 合同。
753. Embedding completed 下载事件在唯一 Sidecar Client 边界直接解析为 canonical 模型摘要，删除 action 层第二次 `Value` 解包和重复错误映射。
754. 删除 Embedding Service 单调用 `result_response` 门面；普通 handler 直接使用协议 `ServiceResponse::from_result` 和唯一错误 mapper。
755. 删除 Embedding Service 只转发 `validate_known_service` 的 `validate_service_request`；普通和事件 handler 直接调用协议 owner。
756. Embedding `service_error` 改为消费生命周期已结束的 `AppError`，响应与事件调用点直接传函数，不再保留借用式闭包和错误引用适配。
757. embed handler 在验证后移动 trace ID 与 texts，删除为响应装配保留完整 request 而深复制整批文本的路径。
758. 模型操作校验改为消费 `EmbeddingModelPayload` 并返回唯一规范化模型名，下载信息与事件入口删除 `required_model().to_string()` 的临时借用/复制链。
759. Embedding Service 删除仅由已退役本地 DTO derive 使用的 `serde` 直接依赖。
760. `DownloadEmbeddingModelInput.task_id` 改为 Host 合同必填字段，generated TypeScript 与前端下载 API 同步删除 nullable/optional task ID；设置页唯一生产入口本就始终生成任务 ID。
761. 删除 `normalize_optional_embedding_download_task_id` 及 `Option<String>` 映射链；下载与取消入口统一调用唯一严格 `required_task_id(&str)` 规则。
762. Server Embedding 下载改为必然登记任务、持有取消令牌并启动进度桥接 worker，删除无任务、无进度发送器和无 worker 的三组不可达分支。
763. Tauri Embedding 下载改为必然登记并清理取消令牌，删除无 task ID 时绕过任务表和无取消能力的分支。
764. Runtime action、Sidecar Client 与 FastEmbed 下载入口的 `CancellationToken` 全部改为必填，删除两处 `unwrap_or_default()` 及初始化、HTTP 请求和文件流中的无取消 token 分支。
765. FastEmbed 文件 metadata 读取保留一套无分叉实现，下载路径在整个 metadata future 外层用必填 token 取消；删除内部 `Option<&CancellationToken>`、逐文件 `Some(token)` 适配和有/无 token 双实现。
766. `download_fastembed_model` 直接返回其末尾已经计算并校验的 canonical `LocalEmbeddingModelSummary`，不再丢弃真实下载结果。
767. Embedding Sidecar completed 事件只传目标模型摘要，删除下载后重新枚举全部模型及完整 `Vec<LocalEmbeddingModelSummary>` 的跨进程序列化。
768. Runtime 的 Sidecar、本机队列和 Tauri cache 下载直接贯通目标模型摘要，删除两次下载后全模型扫描、二次名称查找及 `downloaded_model_summary` 门面。
769. `FastEmbedDownloadFile.downloaded` 始终等于 `downloaded_size_bytes >= size_bytes`，删除该镜像布尔状态，下载筛选直接使用唯一字节事实。
770. FastEmbed metadata 收集改为借用 `ModelInfo`，并合并只被外层调用一次的五参数内部函数；删除下载与下载信息路径中的整份模型信息 clone 和单调用 Facade。
771. FastEmbed 下载信息直接在公开入口装配 canonical summary，删除单调用 `collect_fastembed_download_info`、模型名/仓库名 clone 和借用后再 `to_string` 的中转。
772. 模型文件按顺序下载，累计进度不存在并发写入；以单个可变 `u64` 取代 `Arc<AtomicU64>`、`Ordering` 和逐文件 Arc clone。
773. `DownloadProgress` 只承担 Sidecar Serialize/Runtime Deserialize 和 channel 所有权传递，删除无生产或夹具消费者的 `Debug`/`Clone` 能力。
774. FastEmbed/Embedding Service/Runtime 下载链直接移动模型名、缓存路径和下载结果，删除 `info_for_download`、`cache_dir_for_error`、`download_cache_dir`、`model_for_download` 等生命周期内无第二读取者的 clone/别名。
775. 删除 `explicit_proxy_env`、`first_non_empty_env` 和 `merge_download_proxy` 第二套显式代理解析；reqwest/HF Hub 继续直接读取标准环境，系统代理只负责补齐环境中缺失的协议。
776. 删除 HTTP、HTTPS、ALL 三类代理在 merge 层和 env 投影层的双重交叉回退；显式 SOCKS `ALL_PROXY` 不再被系统或推导出的 HTTP(S) 地址覆盖。
777. 系统代理投影按 `HTTP_PROXY/http_proxy`、`HTTPS_PROXY/https_proxy`、`ALL_PROXY/all_proxy`、`NO_PROXY/no_proxy` 四组逐项补缺，任一大小写显式值存在时均保持原值。
778. Managed Sidecar 使用 `env_clear`，公共环境投影补齐此前遗漏的 `ALL_PROXY` 及四类 lowercase 标准代理键，独立 Embedding 进程不再丢失 Host 的真实代理配置。
779. Linux GNOME 代理读取收敛到 `gsettings_proxy_url`，删除 host 缺失时伪造空字符串、port 缺失时伪造 0 的六处默认值；host/port 任一缺失即明确表示该协议未配置。
780. macOS 代理读取收敛到 `scutil_proxy_url`，删除 HTTP/HTTPS/SOCKS 三套 enabled/host/port 分支及空 host、0 port 默认值。
781. 平台代理入口使用 cfg import alias 直接指向 Linux/macOS/Windows 实现，删除三份只调用一次同名函数的包装层。
782. `SystemProxy` 删除仅由退役 merge 逻辑和旧测试支撑的 `Debug`/`Clone` 能力，仅保留 Windows parser 实际需要的 `Default`。
783. GNOME ignore-hosts 数组元素已由 `parse_gsettings_string` 完成 trim，删除紧随其后的第二次 trim/String 分配。
784. `ScopedHfHubEnv` 将代理投影限制在 metadata 与文件 HTTP client 两个真实网络边界；已验证完整缓存后执行的模型初始化只设置 `HF_HOME`，不再重复探测系统代理。
785. 删除两项固化“系统代理覆盖显式 ALL_PROXY”和“HTTPS 覆盖 SOCKS ALL_PROXY”旧分叉的测试，保留一项严格检查显式协议优先、系统协议仅补缺的当前合同。
786. 无系统代理时 `download_proxy_env_vars` 直接返回空 guard 集合，删除 `Option<Vec<_>>.unwrap_or_default()` 对缺失代理的间接表达。
787. 公共 LLM baseline seed 收敛到 `normalize_create_llm_config_input` 唯一规范化 owner，名称、provider、URL、安全传输、正数限制、输出/上下文关系与 extra body 不再由 Runtime 维护第二套规则。
788. 删除字段完全镜像 `NormalizedLlmConfig` 的 `PublicLlmSeed`，环境解析完成后直接生成 canonical LLM 配置值。
789. 删除 baseline Runtime 私有 `required_seed_value` 与 `positive_i32`；LLM 必填值和数值约束统一由现有 LLM 配置合同负责。
790. 删除 baseline Runtime 私有 `app_error_plain_message`，错误文本提取复用 `a4n-core::config` 唯一实现。
791. 删除 baseline Runtime 私有 `normalize_optional_string`，LLM 与图片 fallback 字符串统一复用 Core `normalize_optional_config_value`。
792. 图片 baseline 将已校验的 `CreateImageConfigParams` 直接交给 Orchestrator，删除 Runtime 对八个持久化字段的逐一复制。
793. 视频 baseline 将已校验的 `VideoConfigParams` 直接交给 Orchestrator，删除 Runtime 对十四个持久化字段的逐一复制。
794. Orchestrator baseline 写入复用现有 `llm_record_input`、`image_record_input`、`video_record_input` 三个唯一业务参数→DB record 转换 owner。
795. DB baseline 插入直接接收现有 `LlmConfigRecordInput`、`ImageConfigRecordInput`、`VideoConfigRecordInput`，不再维护仅服务启动 seed 的第二套持久化 DTO。
796. 删除字段分别与三类 record input 完全相同的 `BaselineLlmConfigSeed`、`BaselineImageConfigSeed`、`BaselineVideoConfigSeed`。
797. 三个 baseline seed 镜像类型退役后同步删除仅为 clone/equality 自证而存在的 `Clone`、`Debug`、`PartialEq`、`Eq` 能力与镜像测试。
798. 删除空余 baseline `contracts`/`tests` 模块及 Orchestrator/DB 的 seed 类型公开重导出，启动配置链只暴露真实 canonical 合同。
799. 四类 baseline 存在性查询由 `COUNT(*) > 0` 改为首条记录 `LIMIT 1` 语义，删除为布尔判断扫描计数及 `PaginatorTrait` 依赖。
800. 本机系统默认 LLM/图片/视频 API Key 字段名收敛到 `SYSTEM_API_KEY_FIELDS` 唯一映射，删除三处手写字段选择。
801. 三类系统默认 API Key 的读取与更新收敛到 `getSystemDefaultApiKey`/`updateSystemDefaultApiKey`，保留对外类型化函数但删除重复存储逻辑。
802. LLM/图片/视频是否使用系统默认及切换系统默认的读写规则收敛到 `usesSystemDefaultConfig`/`setSystemDefaultConfig`。
803. 三类本机活跃配置选择收敛到 `activeLocalConfig`，删除相同的 system-source 返回空值与默认项查找分叉。
804. 前端三类服务器默认模板的默认项查找、虚拟 ID、名称前缀、个人密钥状态和当前默认状态投影收敛到 `systemDefaultProviderConfig`。
805. 三类“服务器尚未配置系统默认模板”门禁收敛到 `requireSystemDefaultProviderConfig`，各业务只保留差异化错误文案。
806. Runtime 配置快照四类 override 的 own-property 判定收敛到 `hasRuntimeConfigOverride`，避免各字段各自维护是否显式覆盖的规则。
807. Runtime 配置快照三类系统 API Key 的包含条件收敛到 `systemApiKeyForSnapshot`，include 策略、默认源与显式 override 只保留一套判断。
808. LLM 普通创建与 baseline 创建共享 `insert_llm_config_record` 唯一 ActiveModel 装配，名称唯一性检查仍只留在普通用户创建边界。
809. 图片普通创建与 baseline 创建共享 `insert_image_config_record` 唯一 ActiveModel 装配，`is_default` 成为两条真实写入场景的唯一差异。
810. 视频普通创建与 baseline 创建共享 `insert_video_config_record` 唯一 ActiveModel 装配，十四个 provider 字段不再在 baseline 模块复制。
811. baseline insertion 删除三套 LLM/图片/视频 ActiveModel 字段映射及对应模型 import，只负责空表复查、调用 canonical 插入 owner 和返回是否创建。
812. Embedding 普通创建与固定 baseline 创建共享 `insert_embedding_config_record`，基础本地哈希配置不再维护第二套 ActiveModel 装配。
813. baseline insertion 完全删除四类 provider ActiveModel、时间戳与 SeaORM 字段设置逻辑，只保留四类 canonical record input 与插入 owner。
814. 四类 `*ConfigRecordInput` 删除无生产、夹具或比较消费者的 `Clone`/`Debug` 能力，记录输入保持一次性所有权传递。
815. LLM、Embedding、图片、视频普通创建与 baseline 创建现各自只有一个持久化插入实现，所有 provider 的默认标记差异统一由唯一 `is_default` 参数表达。
816. Runtime 将 LLM、图片、视频三套 `discover_*_models` 的 embedded/sidecar 分流、请求装配、能力缺失错误和响应解析收敛到唯一 `sidecar_clients::model_discovery::discover_models`。
817. 删除 LLM 的 `model_discovery_payload` 以及 image/video routing 中两份完整探测实现、相关公开重导出和重复 import，三类业务差异只保留 `ModelDiscoveryKind`。
818. 模型探测的 service name 使用协议常量，capability version 由 `expected_capability_version_for_service` 唯一注册表派生，删除 Runtime 内三项版本映射副本。
819. LLM、图片、视频三个 Sidecar 探测 handler 在校验后移动 trace ID 与 payload，删除三次 `request.payload.clone()` 和仅为借用 request 保留的响应包装路径。
820. LLM 探测入口删除 `validate_service_request` 与 `validate_provider_secret_policy` 两段式校验，直接复用 `ServiceRequest::validate_known_provider_request` 唯一协议不变量。
821. 图片与视频删除纯转发 `validate_model_discovery_request`，探测 handler 与生成/轮询入口统一调用协议层已知 provider 请求校验。
822. 删除图片、视频各两项重复验证通用 provider secret policy 的模型探测测试及其 `model_discovery_payload` 夹具；协议规则继续由 Service Protocol 专属测试唯一拥有。
823. `ModelDiscoveryRequest` 删除无生产消费者的 `Debug`、`Clone` 和冗余 `serde(default)`，请求保持一次性传输所有权；缺失可选 API Key 继续由 Serde `Option` 原生语义表示。
824. `ModelDiscoveryEndpoint` 删除无消费者的 `Debug`/`Clone`，Auth/Parser 仅保留按值读取所需的 `Clone + Copy`。
825. 端点选择由始终非空的 `Vec<ModelDiscoveryEndpoint>` 收敛为单一 `ModelDiscoveryEndpoint`，删除不可达的空端点错误、循环、错误累积和多端点成功短路。
826. OpenAI-compatible 模型列表只使用 canonical `{base}/models`，删除根地址下额外尝试 `/v1/models` 的兼容回退、URL 去重容器和 `push_unique_url`。
827. 已知生成端点后缀只执行一次精确剥离，删除未使用的 Responses 后缀、大小写兼容投影和可重复剥离循环。
828. Gemini 探测只由活跃 provider `gemini-veo-video` 选择，删除根据任意 Base URL 子串猜测 provider 及接受 `/models` 作为 Base URL 的隐式兼容分支。
829. OpenAI/Anthropic 响应统一为唯一 `data[]` 正式合同并将 parser 命名为 `DataList`，删除顶层 `models[]` 与裸数组两套旧响应入口。
830. DataList 条目严格要求对象与非空 `id`，删除字符串模型项以及用 `name`/`model` 冒充 ID 的兼容转换。
831. 响应字段删除 `owner` 别名、Gemini `display_name` 别名、SD `title` 代替 `model_name` 和 ComfyUI 直接根节点四类旧格式分叉。
832. 四类 parser 由 `filter_map` 静默丢弃畸形条目改为逐项显式错误，非法 provider 响应不再被部分成功掩盖。
833. 重复模型 ID 由静默去重改为合同错误，删除 `dedupe_discovered_models` 及其复制/重建结果列表。
834. Gemini 结果删除响应中不存在的硬编码 `owned_by = "google"` 默认值，只投影上游真实提供的 ID 与显示名。
835. 每次探测只发送一个请求，删除一次性 HTTP Client 无实际作用的连接池 idle timeout 配置。
836. 图片 Service 的 AppError→ServiceError 映射改为按值消费，普通响应、事件失败和模型探测共用唯一 `service_error`，删除调用点闭包。
837. 视频 `result_response` 删除与 `service_error` 完全相同的内联错误组装，事件与探测同样按值复用唯一 owner，并收紧 `service_error_code` 的无用 crate 可见性。
838. `ModelDiscoveryInput` 删除无调用者的 `Debug`、`Clone`、`PartialEq` 与 `Serialize`，仅保留 Host 输入反序列化和 schema 生成所需能力。
839. Runtime config actions 删除无外部消费者的 `DiscoveredModel`/`ModelDiscoveryResult` 重导出，`a4n-model-discovery` 也不再公开内部使用的 `DiscoveredModel` 临时别名。
840. 已保存 API Key 读取删除 `String -> String -> Option<String> -> Option<Option<String>>` 链，改为一次借用规范化后构造唯一结果。
841. 前端删除 `detectServerLlmConfigModels`、`detectServerImageConfigModels`、`detectServerVideoConfigModels` 三个纯转发别名，每类 provider 只保留一个显式 scope 的检测 API。
842. 前端命令调用、服务器 scope 直传和本机运行时密钥解析收敛到 `detectConfigModels`/`resolveModelDiscoveryInput`，三类 API 只提供命令名与密钥来源差异。
843. 三个配置编辑器删除 server/local 检测函数选择分支，共用一次 `scope` 计算，同时驱动 discovery key 与唯一检测 API。
844. `ModelDiscoveryField` 的 `disabled`、`inputDisabled`、`discoveryKey` 改为真实调用者均显式提供的必填属性，删除从未使用的默认值和可选状态。
845. 前端删除对空模型结果的第二次校验及首项 ID 可选回退；共享后端已保证成功结果非空且每个 ID 为非空字符串。
846. 模型 datalist 与计数提示合并为一次非空条件渲染，删除重复状态分支；label 过滤直接使用严格 nullable 合同，不再重复运行字符串类型/空白检查。
847. 模型探测链现只有一套前端命令/密钥解析、一套 Runtime profile 分流、一套 Sidecar 请求构造、一套协议校验和一套 provider HTTP/parser 实现。
848. LLM Gateway probe handler 在协议校验后按值消费 payload，删除整份 `request.payload.clone()` 和只调用一次的 `probe_with_provider` 门面。
849. LLM Gateway 删除无其他消费者的 `result_response`；handler 移出 trace ID 后直接使用 `ServiceResponse::from_result` 与唯一 `service_error` owner。
850. `LlmProbePayload.request` 从可选值收紧为必填 `ChatRequest`，删除 `request = null` 时改走 `provider.test_connection()` 的第二套连通性实现。
851. `LlmProbeOutput` 删除恒真 `ok` 和可选 `response_preview`；空响应由 `from_chat_response` 唯一拒绝，成功合同直接携带非空预览。
852. Runtime 删除 `llm_probe_succeeded` 及其对成功响应的二次解释；Sidecar 协议成功与非空 probe 响应不再被重新折叠成布尔状态。
853. Dynamic LLM probe 改为 `Result<(), AppError>`，删除无调用者的 `probe_with_config_id`、单调用 `probe_resolved_model` 和尾部恒定 `Ok(true)`。
854. `LlmProvider` trait 删除无生产调用者的 `test_connection -> Result<bool, AppError>`；连通性测试只走配置 action 明确构造的真实 chat/probe 请求。
855. OpenAI-compatible 与 Anthropic HTTP provider 删除各自复制的 ping 请求、空响应布尔映射、连接测试 token 常量和关联 import。
856. `DynamicLlmProvider` 删除第三套手写 ping 请求及 embedded/sidecar 分支；默认 provider 不再携带无人调用的隐式连通性入口。
857. `PlanningLlmProvider` 删除永远不会被生产调用的“不能执行连接测试”接口实现，只保留其真实上下文规划能力边界。
858. Core stream fixture、structured stream fixture、Orchestrator fake provider、bootstrap provider及全部 LLM test-support provider 删除纯样板 `test_connection` 实现。
859. `CountingLlmProvider` 删除连接测试计数原子量、构造初始化、读取 API 和自测断言，只保留生产代码实际依赖的 stream 调用统计。
860. LLM Gateway probe 测试删除“旧 fallback 未调用”的镜像断言；该 fallback 已从 trait 与生产代码结构上消失，不再维护第二套负向状态。
861. `UnavailableLlmProvider::new` 删除从未保存的 `_chat_error` 参数，Orchestrator 测试 builder 同步删除死默认常量和重复调用参数。
862. 前端删除 `testServerLlmConfig` 纯转发别名，LLM 测试只保留一个带显式 scope 的 API。
863. 前端删除 `previewServerTestLlmConfigRequest` 纯转发别名，LLM 测试预览只保留一个带显式 scope 的 API。
864. 前端删除 `testServerImageConfig` 纯转发别名，图片测试只保留一个带显式 scope 的 API。
865. 前端删除 `previewServerTestImageConfigRequest` 纯转发别名，图片测试预览只保留一个带显式 scope 的 API。
866. LLM 测试与预览的系统模板、本机配置、服务器配置 ID 解析收敛到 `resolveLlmConnectivityTarget`，删除两套平行分支。
867. 图片测试与预览的系统模板、本机配置、服务器配置 ID 解析收敛到 `resolveImageConnectivityTarget`，删除两套平行分支。
868. 模型配置作用域统一命名为 `ModelConfigScope`，由模型发现、LLM/图片连通性和三个设置编辑器共同使用，不再把同一 server/local 事实限定为 discovery 私有类型。
869. LLM、图片、视频设置 Tab 的默认作用域直接使用 `local`，管理员页显式使用 `server`，删除 UI `user` 与 API `local` 对同一场景的双词汇映射。
870. 三个配置编辑器将 scope 改为调用者必填属性，删除可选 `serverScope`、默认 `false` 和编辑器内部重复 server→local 转换。
871. LLM/图片测试与预览均只调用一次正式 Host 命令：server scope 显式禁止 Runtime 快照注入，local scope只装配被测配置的精确快照；不再通过函数别名选择执行路径。
872. `DynamicEmbeddingProvider::embed_with_identity` 直接调用已解析 provider 的 canonical `embed_with_identity`，删除手写 `model_identity + embed` 分叉。
873. Embedding Service handler 改为一次 `embed_with_identity` 获取同次 provider 调用的身份与向量，删除服务层第二套身份/执行拼装。
874. `DynamicEmbeddingProvider::embed` 复用自身唯一的 identity/vector 队列路径，只在边界丢弃身份，不再重复默认配置解析与低优先级队列装配。
875. `SidecarEmbeddingProvider::embed` 同样复用唯一 sidecar identity/vector 路径，删除第二次默认模型解析和结果拆包实现。
876. Embedded Embedding API 地址删除按 `:11434` 猜测 Ollama 并自动补 `/v1` 的兼容回退；Embedded 与 Sidecar 均严格使用显式 Base URL。
877. HTTP Embedding 删除空输入专用成功分支；合法批量大小下原生分块循环自然返回空结果，非法零批量仍显式失败。
878. 单条连通性向量的“数量为 1、维度匹配、配置维度为正”收敛到 `validate_embedding_connectivity_vectors`，Embedded provider 与 Sidecar 响应共用唯一实现。
879. Sidecar 配置连通性测试直接使用已严格构造 payload 的 `dimension`，删除对同一模型维度的第二次 `positive_usize` 校验。
880. Sidecar 正式 Embedding 调用同样从已验证 payload 读取期望维度，删除请求构造前后的重复正整数转换。
881. `AppConfig::embedding_models_dir` 成为 Embedding 模型默认缓存目录唯一 owner，替换 Core/Orchestrator/Sidecar 环境三处 `cache_dir/embedding-models` 拼接及 Runtime `effective_*` 包装。
882. Orchestrator、AppState、模型列表/信息/下载/删除、连通性和存储摘要统一使用 `embedding_models_dir` 词汇，删除 `default_fastembed_cache_dir` 第二命名体系。
883. 本机 Embedding Runtime 删除单调用 `fastembed_cache_dir_from_config` 转发函数，直接消费 AppConfig 的 canonical 模型目录。
884. `LocalEmbeddingProviderRuntime` 删除无外部消费者的通用 `new(PathBuf)` 构造入口，只允许从完整 AppConfig 建立生产状态。
885. 本机 provider 缓存键改用原生 `(model, dimension, batch_size, path)` 元组，删除换行拼接字符串、自定义字段顺序和潜在分隔符碰撞。
886. 本机桌面 backend 校验不再返回恒定 `FastEmbed` 字符串或把该常量写入缓存键，删除单调用 normalizer 与无信息量缓存维度。
887. `DynamicEmbeddingProvider` 只保存实际使用的 dimension/batch metadata，删除整份 EmbeddingConfig 及 API key、URL、模型、路径等从未读取状态的克隆。
888. 删除无生产调用者的 Dynamic Embedding 专用 `test_connection` override；真实连通性继续由配置 action 和本机桌面 Runtime 对具体 provider 执行。
889. `EmbeddingBackend` 删除无序列化、反序列化或复制消费者的 `Clone/Serialize/Deserialize` 能力，仅保留 identity 构造需要的 `Debug`。
890. 前端 Embedding 测试与预览共用 `resolveEmbeddingConnectivityTarget`，配置 ID、本机配置读取和 Runtime 快照目标不再维护两套分支。
891. Embedding 配置 CRUD、连通性和模型管理 API 全部接收显式 `ModelConfigScope`，本机设置与本机桌面执行分别由两项共享 scope 判定唯一控制。
892. `EmbeddingTab` 与其他模型设置页一致支持 scope：普通设置默认 `local`，管理员 API 配置页显式传 `server`。
893. 管理员 API 总览显式读取 server Embedding 配置，删除桌面模式下误读浏览器本机配置的执行分叉。
894. Embedding 模型列表、大小、下载、取消和删除均受同一 scope 控制；管理员页固定操作服务器模型，本机 Tauri 命令只允许 local scope。
895. Embedded 模型下载信息查询只在 in-cache 边界校验模型名，删除顶层 action 与缓存实现对同一输入的连续两次 trim/必填校验；Sidecar 边界仍独立校验。
896. Embedded 模型删除同样只由 in-cache owner 校验模型名，删除顶层 action 的重复规范化；Sidecar 请求继续使用已验证字符串。
897. FastEmbed 不完整缓存清理删除“目录路径若变成普通文件也自动删除”的 `remove_path` 回退，模型仓库与 lock 路径现在严格要求目录结构。
898. FastEmbed 模型列表用标准迭代收集替换手写 `Vec::new + push` 循环，错误传播与结果构造只保留一条路径。
899. 模型缓存文件大小读取只把 `NotFound` 解释为 0 字节；权限、I/O 和非文件路径不再被 `unwrap_or(0)` 静默伪装为未下载。
900. 已缓存 pointer/blob/part 大于上游声明大小时改为合同错误，删除把超大文件截断为“已完成”或“0 字节”的伪安全状态。
901. 正式下载遇到非零不完整 blob 或超大 `.part` 时显式失败并要求清理，删除自动清空临时文件后继续下载的隐藏恢复路径。
902. 下载取消检查 helper 直接接收 `CancellationToken`，删除五处调用者重复提取布尔状态及可脱离 token 语义的第二入口。
903. 本机 Embedding Runtime 与 Tauri managed state 删除无消费者的 `Clone`，其两个 `Arc<Mutex<...>>` 收敛为直接 `Mutex` 所有权。
904. Tauri 模型下载任务注册拒绝重复 task ID，删除 `HashMap::insert` 静默覆盖旧取消令牌；取消只发送信号并保留注册直到真实任务结束，避免旧任务清理误删新任务。
905. Embedding Service 删除 no-fastembed 构建下的 `let app = app` 无作用自赋值，原 Router 绑定直接进入公共 layer 装配。
906. FastEmbed `ResolvedModel` 删除无调试消费者的 `Debug` 派生。
907. `FastEmbedProvider.inner` 删除 Provider 本身不可克隆情况下多余的 `Arc`，模型实例缓存由单一 Tokio Mutex 直接拥有。
908. 删除单调用 `put_model` 方法，Embedding 执行结束直接在唯一位置归还模型实例。
909. `build_text_embedding` 删除恒为 `false` 的进度参数及单调用 inner 函数；FastEmbed 内建下载进度固定关闭，唯一进度来源仍是正式下载管线。
910. Provider 的下载完整性检查不再构造完整模型摘要或递归统计缓存目录大小，只检查运行所需 snapshot 文件是否齐全。
911. `FastEmbedProvider` 保存创建时已解析的 `ModelInfo`，删除 `model + dimension` 镜像字段以及每次 Embedding 前重新查询模型注册表的路径。
912. task ID 的 trim、非空、160 字节和 ASCII 安全字符合同收敛到 Core `normalize_task_id`，Server、Runtime、Tauri、RAG 与 Embedding 不再各自维护规则副本。
913. `TaskManager::register` 在唯一注册 owner 内再次规范化 task ID，未来新增调用者不能绕过统一身份合同写入内存注册表。
914. 删除 Embedding Runtime 专用 `required_task_id` 别名及其模型下载错误文案分叉，Server/Tauri 下载与取消直接复用 Core task ID 合同。
915. 删除只剩两个常量的 `embedding_model/models/scope.rs` 空壳模块，下载总进度和初始消息回归 Embedding Host action 根模块。
916. RAG ingest operation trace 删除第三套 task ID 长度/字符校验，`task:{id}` 只接受 Core 已规范化身份。
917. `cancel_workflow` 删除私有 trim/filter 身份规则，取消先于注册的路径也只接收 Core canonical task ID。
918. workflow progress 查询目标改为持有 canonical `String`，删除借用原始输入和重复 trim/filter 的第二身份状态。
919. 持久队列 `cancel_task` 在 Runtime 边界统一规范化 task ID，Server 删除仅检查空字符串的弱校验分叉。
920. `TaskManager::register` 改用 DashMap 原子 Entry，重复 task ID 显式返回 `InvalidState`，不再用 `insert` 静默覆盖旧任务。
921. 任务注册错误合同贯通 Orchestrator、Runtime Host API 与 Server `register_web_task`，重复身份稳定映射为 HTTP 409，而不是继续执行新业务任务。
922. Galgame、角色发现/同步/冲突、场景档案、RAG ingest、Embedding 下载、逆向工程、导出、章节分析/场景/剧本共 14 处注册调用（宏展开后 21 个 Web 命令）全部传播注册失败。
923. pending cancellation 从 `task_id -> {user_id, timestamp}` 改为 `(task_id, user_id) -> timestamp`，删除可被另一用户覆盖的单值状态和 `PendingCancellation` 包装结构。
924. 已存在但不属于请求用户的任务不再写入 pending cancellation，错误用户的取消请求不能污染未来同名任务。
925. task ID 首次成功注册时一次消费该 ID 的全部 pending 记录，仅匹配 owner 的记录触发取消，其余记录立即清除而非等待 TTL 后误伤未来复用。
926. pending cancellation 的二次注册竞态检查继续保留，但其状态与清理均由 TaskManager 单一 owner 管理，不再散落到 Host。
927. 任务完成清理从“按字符串删除”改为 DashMap `remove_if + Arc::ptr_eq`，迟到的旧句柄不能删除后来注册的同名任务。
928. `finish_web_task` 及全部 Web 完成入口改为传递真实 `TaskHandle` 身份，删除完成阶段重新依赖 task ID 字符串的弱关联。
929. 精确句柄移除失败由 `TaskManager::remove` 唯一返回 `InvalidState`，Server 不再忽略删除结果或维护第二套生命周期错误。
930. Orchestrator/Runtime 的 remove 链从无人消费的 `Option<Arc<TaskHandle>>` 改为 `Result<()>`，删除无意义返回值和静默失败路径。
931. `AppState` 删除注册、移除、查询、取消、预取消和活跃快照六个纯转发方法，Runtime task actions 直接进入 Orchestrator task owner。
932. Runtime task access、取消、progress 查询和 active workflow 投影全部直接读取 Orchestrator，删除 AppState 的镜像任务 API 面。
933. 删除无任何生产或测试调用者的公开 `get_task_handle` Host action。
934. Server Embedding 取消不再忽略 `user_id`，改由 TaskManager owner 校验任务所有者；其他用户不能仅凭 task ID 取消本机下载。
935. Tauri Embedding 下载结束时要求对应注册项真实存在，删除 `HashMap::remove` 结果被静默忽略的内部状态丢失路径。
936. `TaskManager` 自身已由外层 `Arc` 唯一共享，内部 DashMap 与 pending Mutex 删除重复 `Arc` 包装和两次独立堆分配。
937. `TaskManager` 使用字段 `Default` 派生，删除与 `new` 重复的手写初始化实现。
938. 删除无调用者、仅服务自定义 Debug 的 `active_count` 全表扫描，并删除没有实际格式化消费者的 TaskManager Debug 实现。
939. Server task identity adapter 缩为 Core 调用，删除两组重复验证测试；task ID 规则检查集中在 Core owner。
940. 删除 Embedding Runtime 对 task ID 规则的重复测试，Core 同时覆盖规范化、非法字符、超长、重复注册、用户隔离和旧句柄清理不变量。
941. 两个章节剧本生成入口删除手写“报告进度 + 检查取消”闭包，直接复用现有 `web_progress_reporter` 唯一实现。
942. Galgame、Embedding 下载和两类导出注册删除注册后不再使用的 task ID 克隆，任务身份只保留业务输入与注册句柄需要的所有权。
943. 从十一类生成 worker 捕获的完整 `AppState` 反向 BFS，确认旧 `GenerationExecutionOwner -> JoinHandle -> worker -> AppState -> owner` 强引用链使 `Drop abort` 在 Host 退出路径不可触发。
944. 删除伪安全 `GenerationExecutionOwner`、`generation_executions` 字段、worker `BTreeMap` 和无效 `Drop`，Host worker 生命周期只由 Tokio/Tauri Runtime 拥有。
945. 删除每次 spawn 才执行的 finished-handle `retain`，已完成 worker 不再长期滞留于 Runtime 镜像状态。
946. 删除 Runtime worker map 的 task ID 重复断言；PostgreSQL active-attempt reservation、session registry 与 MemoryTaskQueue permit 已是三层活跃唯一执行约束。
947. 确认 generation retry 创建新 UUID 并只用 `retry_of_task_id` 表达 lineage，删除 Runtime owner 对 task ID/attempt 身份的伪建模职责。
948. 三类 generation spawner 直接 detach 到 Host Runtime，同时保留 request scope 继承与 LLM/Image/Video task-local slot，未新增关闭 fallback。
949. 十一类 worker 的失败记录收敛到统一 spawner，task ID、attempt、worker kind 和 error 不再由各业务入口重复拼装。
950. 十一类执行器接收移动后的 `GenerationTaskHandle`，删除仅为执行后日志保留的 handle 克隆。
951. 章节 generation 启动链改为借用输入，删除 Runtime 到 Orchestrator 的完整 `StartChapterGenerationInput` 克隆。
952. 灵感 generation 启动链改为借用输入，删除消息与上下文 DTO 的完整克隆。
953. 大纲 generation 启动链改为借用输入，删除操作参数和生成选项的完整克隆。
954. 人物立绘正式 Host 与 Galgame 内联两条启动链共用借用式 reservation，删除两处完整 portrait 请求克隆。
955. 图片 generation 启动链改为借用输入，批量目标、风格与引用媒体不再在 reservation 前整体复制。
956. 视频/视频提示词 generation 启动链改为借用输入，章节批量、参考媒体和 provider 参数不再整体复制。
957. 蓝图生成与蓝图优化共用的 mutation reservation 改为借用请求和 request meta，删除两套 DTO/meta 所有权搬运。
958. Optimize chat、analysis 与 patch 三类启动链改为借用输入，消息、权限和 issue 集合只由真实 worker 持有。
959. 从项目删除/重置、用户禁用失败路径继续 BFS 到 destructive continuation，确认其 `JoinHandle` 从未 join、abort 或读取结果，只承担同 key 去重。
960. destructive continuation 的 `BTreeMap<String, JoinHandle>` 收敛为活动 key `BTreeSet`，删除第二套后台任务 handle owner和完成 handle 滞留。
961. `ContinuationPermit` 在正常完成、panic 与 Runtime abort 时统一移除活动 key，不再依赖下一次同 key 调度发现 `is_finished`。
962. permit 在 target lock 之后进入作用域，退出顺序固定为先释放 active key、再释放 target lock，避免新失败请求误判旧 continuation 仍活跃。
963. PostgreSQL lifecycle gate 保持 delete/reset/disable 接续的唯一持久事实源；内存集合只负责当前 Host 的并发去重。
964. 删除全仓零调用的 `GenerationSessionRegistry::remove`；terminal session 只由 TTL cleanup 与容量 eviction 两条活跃 owner 回收。
965. 删除全仓零调用的 `GenerationTaskSession::is_terminal` 镜像查询，生产调用者只从 canonical snapshot 判断状态。
966. session reserved sequence 高水位 getter 收紧为测试私有，生产 API 不再暴露无消费者的内部容量状态。
967. committed event permit 的 events/first/last 三个观察 getter 收紧为测试私有，生产面只保留 snapshot 与 commit。
968. generation session 正常启动的严格 `insert` 与 snapshot restore 的 `insert_restored_or_get` 合并为唯一 `insert_or_get`。
969. 并发重复 start 同时构造同 task/attempt session 时，后到者复用 canonical session 并返回 `should_run=false`，不再制造第二执行器。
970. 删除后到重复请求因“已有 session”触发 `terminate_start_failure`、进而中断先到真实 worker 的错误分叉。
971. 未被采用的候选 session 通过现有 Arc/capacity lease 析构立即释放 admission；不同 attempt 的真实身份冲突继续显式失败。
972. 从 `a4n-orchestrator::generation` 的声明、公开 re-export、Runtime action、destructive lifecycle、Server SSE 与 Tauri Channel 逐层 BFS，确认跨 crate 可命名合同只有 replay、subscription、session error、terminal error 与 terminal write outcome。
973. generation 的 `artifacts/checkpoint/events/stream/terminal` 五个实现子模块统一收紧为 crate 内部，外部调用只从 `generation` 根模块取得真实合同，不再耦合内部目录结构。
974. artifact 预备事件与协调器收紧为 crate 内部，人物立绘、图片、视频、terminal commit 与 session 测试继续复用唯一 artifact 构造入口。
975. checkpoint metadata、write outcome/error、writer 与数据库 sequence lease replenisher 全部收紧为 crate 内部；generation 根进度、LLM stream、视频 provider 与各业务 workflow 的调用闭包保持完整。
976. session config、admission controller、capacity snapshot、permit 与 registry 的公开面按使用层级收紧；配置字段和 admission 统计不再伪装成跨 crate 合同。
977. durable commit、sequence lease update、task session、committed permit、publisher trait 与 session publisher 收紧为 crate 内部；公开 replay/subscription 方法保持 Server/Tauri 活跃消费所需的最小集合。
978. LLM generation request/checkpoint/output、统一 stream driver 与无 session workflow publisher 收紧为 crate 内部，所有章节、灵感、蓝图、Optimize 和视频提示词调用继续走同一 driver。
979. completion descriptor 与 terminal coordinator 及其生命周期方法收紧为 crate 内部；外部只保留可观察的 terminal error/write outcome，不再暴露持久化实现对象。
980. Runtime generation action 与 destructive lifecycle 改从 `generation` 根模块导入公开合同，删除跨 crate 对 `events`/`terminal` 私有实现模块的直接路径依赖。
981. 十一类 generation start 的十套 `ReserveGenerationTaskInput` 装配收敛到唯一 `reserve_root_generation_task`，同时统一数据库 reservation、错误映射与 reserved session 初始化。
982. root generation 的 `root_project/source_snapshot/workflow/generation run/root task/parent/item` 空 lineage 只在共享 reservation owner 设置一次，删除九套重复 None 字段清单。
983. start request ID 与 retry lineage 的边界复制只在共享 reservation owner 执行一次，各业务 start 不再重复维护数据库所有权转换。
984. 各业务仍显式声明 business key、fingerprint、task kind、priority、delivery、batch policy、scope 与 target；共享 owner 只合并完全相同的 root reservation 不变量，没有引入默认执行策略。
985. generation task scope 收紧为静态业务常量，蓝图 generation/refinement 继续由现有共享 mutation owner选择固定 scope，不接受运行时动态配置。
986. 视频 `PromptFrames` 与 `PromptVideos` 重复的 segment-plan/target-segment 提取及目标键拼装收敛为唯一 `video_prompt_unit_target_id`，继续依赖启动前严格校验，不新增缺省值或回退。
987. 已用静态搜索确认跨 crate 的 generation 私有子模块路径清零，Orchestrator root reservation 构造只剩共享 owner 一处，十类调用覆盖十一项 start 功能。
988. 已从 API start DTO、Runtime action、Orchestrator 权限/参数 preflight、fingerprint/operation/target mapper、DB reservation 规范化、active-attempt/retry lineage、Server SSE 与 Tauri Channel 完成本批双向 BFS；未发现 XML、反射、动态代理、AOP 或路由配置中的第二 generation 实现入口。
989. 从视频“插入片段提示词生成”入口执行 BFS，覆盖 `InsertedSegmentTimelinePanel`、`VideoPromptSetupPanel`、`VideoTab`、前端 API/generated schema、command manifest、Server REST/command dispatch、Runtime worker、Orchestrator start/worker/timeline、Core gap、DB timeline 与 workflow run。
990. `VideoTimelineGap::canonical_key` 成为 Rust 侧开放边界 gap 身份的唯一 owner；合法 ID 仍为正整数，`0` 只编码 head/tail，不作为运行时默认或无效 ID 兜底。
991. generation reservation target 与 inserted-segment workflow business key 统一复用 canonical gap key，删除两处 `previous/next.unwrap_or(0)` 手写拼装。
992. 前端 React/select gap key 统一复用 `insertedSegmentGapKey`，head/tail 使用显式文本标识，删除多个组件各自拼接 nullable ID 的分叉。
993. `VideoGenerationOperation` 的 wire kind 与 generation task key 合并为枚举自身的一张元数据表，删除 Orchestrator context、普通视频 worker、提示词 worker 三套重复 variant 映射。
994. `PromptFrames` 与 `PromptVideos` 的 target 分支合并为同一 match arm，并从 canonical operation kind 取得差异标签，不再维护两条结构相同的调用。
995. 四类视频提示词 operation 到 `PromptTaskInput` 的装配改为先提取 `(chapter_id, operation)`、再统一填充 project/retry 字段，删除四份重复结构体构造。
996. 插入片段时长校验直接检查已验证候选集合是否包含选中时长，删除仅为 membership 执行的排序、去重和临时 Vec。
997. 删除单调用 `normalized_duration_options` helper；生成边界只执行一次排序/去重，并将 canonical 候选写入 workflow fingerprint。
998. workflow 提交不再对当前 writer 已 canonicalize 的 duration options 二次排序/去重，直接使用持久 fingerprint，删除旧格式容忍式再规范化。
999. 插入片段 reference image IDs 在校验、guidance 判定与 JSON 装配间改为借用，删除整组 ID 的无效克隆。
1000. `serde_json::json!` 已按引用序列化，payload 与 fingerprint 共享 constraints 时删除多余 `Value::clone`。
1001. 先生成 context manifest、再把 payload 移入唯一 variables map，删除完整 prompt payload 的第二份内存副本。
1002. prompt renderer 名称与 context manifest 名称统一由 `INSERTED_SEGMENT_PROMPT_NAME` 提供，删除同一 prompt 名称字面量副本。
1003. workflow 写入与提交 scope 校验统一复用 `VIDEO_TIMELINE_SCOPE_TABLE`，删除 `chapter_video_timeline` 双份字符串事实源。
1004. 草稿生成与提交的 timeline revision 冲突统一复用 `timeline_revision_conflict`，删除一套重复错误码/参数格式装配。
1005. 前端 segment-plan 与 inserted-draft retry wrapper 直接把 optional retry ID 传给唯一 generation client，删除两个等价 null 分支。
1006. 插入草稿完成结果直接由 `runGenerationProgressTaskThen<number>` 返回，删除可被误读为“成功但仍为空”的可变 `workflowRunId` 镜像状态。
1007. 已静态确认旧 `video_generation_operation_kind`、prompt 私有 `operation_key`、`normalized_duration_options`、手写 gap `unwrap_or(0)`、payload clone、retry ternary 与 nullable workflow result 局部状态均无剩余引用。
1008. 启动 preflight 与 worker 内 options 校验继续保留：前者阻止无效任务 reservation 落库，后者保护可直接调用的执行边界；timeline/gap/reference recheck 继续处理启动后数据变化，不属于重复 fallback。
1009. 本批未发现反射、动态代理、AOP、依赖注入容器或 XML 中的隐藏入口；Tauri/Server 的显式 command manifest、REST route 与 command dispatch 是两种活跃 Host 传输，未错误合并。
1010. 从插入片段移动/删除 API 向 Runtime facade、Orchestrator application/context、DB transaction、Core order 规则和前端刷新链继续 BFS；确认两项 DB 返回 timeline model 只被读取调用参数中已知的同一 `timeline_id`。
1011. move/delete 持久化边界统一改为 `Result<()>`，删除两次 mutation 后仅为返回 ID 执行的 timeline SELECT，以及零剩余调用的 `refreshed_timeline` helper。
1012. 上层继续在 mutation 成功后读取一次权威 snapshot 生成响应，不再先接收冗余 timeline model 再按相同 ID 重读。
1013. 顺序未变化且目标已 active 的 move 不再扫描所有插入 prompt 执行历史漂移修复；正式 commit/reconcile/move/delete writer 已在每次真实顺序变化时同步展示序号。
1014. inserted commit 在事务内只计算一次 gap insertion index；创建 DB item 后直接使用标准 `Vec::insert`，删除第二次相同邻接搜索和校验。
1015. `VideoPromptSetupPanel` 的 gap 按钮删除无效子级 React key；列表身份只由外层 keyed Fragment 的 canonical gap key 管理。
1016. gap aria label 不再在唯一非空 timeline 调用点使用空 items 兜底，显式接收当前权威 timeline items。
1017. 共享片段规划的非空约束收敛到 `shared_plan_segments`；native reconcile 与 inserted neighbor context 统一经过同一严格解析 owner。
1018. DB reconcile 在生命周期 gate 后严格拒绝空 native key 集合，删除“空原生规划时把全部 inserted 项留在末尾并标记”的旧容忍分支及专用测试。
1019. native reconcile 从唯一 `existing` 查询一次构建 inserted ID 集合，再按 current order 投影有序列表，删除逐 ID 回扫和 Vec→HashSet 二次构造。
1020. obsolete native 行只有本次仍非 deleted 时才更新；历史已删除行不再被每次 reconcile 重复写入并污染 `updated_at`。
1021. 从前端 `verifyImageGenerationResult` 的八个图片调用点出发执行 BFS，横向覆盖 portrait/video 校验器、generation client/store/reducer、generated contracts、command manifest、Server REST/command dispatch、Tauri 通用 Host dispatch、Runtime worker/HostPort、Orchestrator terminal/artifact/result ref 与三类 DB 正式记录查询。
1022. 图片、立绘、视频三套“终态 result_ref + artifact + 正式 DB 记录”校验收敛到唯一 `verifyGeneratedMediaResult`，各业务只声明 kind、artifact 前缀、记录 scope 和单记录 revision 差异。
1023. result ref ID 与 artifact ID 的解析统一为严格正十进制安全整数，删除 `Number(...)` 对空白、指数、前导符号等非 canonical 字符串的宽松接受。
1024. 正式 artifact 的筛选、非临时约束及 `revision == sha256` 完整性规则统一由共享校验器执行，图片/立绘不再只检查 artifact ID 是否存在。
1025. 单结果校验严格要求当前 media kind 只有一个正式 artifact，且 artifact ID、正式记录 ID、项目/章节 scope 与 result ref 同时一致；删除三套各自查找逻辑。
1026. 批量校验统一拒绝 malformed/重复 artifact ID，按 canonical 升序 ID 计算 revision，并要求每个 ID 都映射到当前 scope 的正式记录。
1027. 校验成功的返回合同收紧为非空数组；调用方不再把“校验成功但没有正式结果”当作可达状态维护第二层异常分支。
1028. 正式结果失败统一调用 generation client 的 `markGenerationConsistencyFailure`，使用 reducer 专用 `consistency_error` action 同时保存 result ref 并标记 recovery failed，删除三处仅改 `client_state` 的弱化分叉。
1029. JSON revision 的 `JSON.stringify + SHA-256` 统一由 `sha256Json` 提供，图片单记录、立绘单记录与共享批量 revision 不再各自拼装哈希调用。
1030. `verifyPortraitGenerationResult` 改为只返回本任务已验证的正式立绘，与图片/视频结果语义一致；删除返回整个项目立绘列表的混淆合同。
1031. Manga 与人物面板的缺失立绘计数直接取已验证结果长度，删除对原始 `task.artifacts` 的第二套计数解释。
1032. 已验证批量结果必为非空后，删除两处不可达的“成功但生成 0 张”提示分支，不保留伪安全文案。
1033. 两处立绘计数和一处批量图片 ID 通过 `runGenerationProgressTaskThen` 返回值直接传出，删除 callback 外可变镜像状态。
1034. 视频两处调用与帧图片两处调用删除 result verifier 之后的 `row/image` 存在性及正 ID 二次校验，非空和 canonical ID 已由唯一校验 owner 保证。
1035. 已显式拒绝空请求后，图片批量 operation 直接移动现有 `requests`，删除 `[requests[0], ...requests.slice(1)]` 的整数组重建。
1036. Rust 侧 JSON 序列化与 SHA-256 `GenerationResultRef` 构造收敛到 `generation_result_ref`，图片/立绘不再各自拥有摘要 hash 实现。
1037. 图片、立绘、视频三类批量 ID 排序与 revision 构造收敛到 `generation_batch_result_ref`，同一批量身份规则只保留一份。
1038. 删除 `batch_result_ref`、`portrait_batch_result_ref`、`batch_video_result_ref` 与 portrait 私有 `result_ref_from_bytes`，同时移除三文件重复 `sha2` import 和临时 bytes/IDs 所有权转换。
1039. BestEffort batch 的 failed key 排序、去重、256 项截断及 truncated 标记收敛到 `best_effort_batch_summary`。
1040. 图片、立绘、视频 accumulator 统一复用共享 batch summary owner，删除三套等价 `BatchCommitPolicy::BestEffort` 结构体装配。
1041. 图片 Host 文件 inspection 直接调用既有 `reference_image_mime`，删除零业务差异的单调用 `image_file_mime` 别名。
1042. 静态搜索确认退役校验分支、批量 result ref helper、batch summary helper、弱 consistency dispatch、原始 artifact 计数、后置 ID/空结果检查及 MIME 别名均无剩余活跃引用；未发现反射、动态代理、AOP、XML 或 DI 配置中的隐藏入口。
1043. 从图片、立绘、视频三类正式 artifact 事件出发继续 BFS，覆盖 Orchestrator HostPort 合同与测试替身、Runtime HostPort 实现、数据目录 canonicalize、文件读取、MIME、SHA-256、byte size、generation event coordinator、视频参考图输入、播放与装配预览消费者。
1044. 三套完全同构的正式媒体文件事实结构收敛为 `generation::GeneratedMediaArtifactFile`，文件内容类型、摘要与大小只保留一个合同。
1045. 删除 `GeneratedImageArtifactFile`、`PortraitArtifactFile`、`VideoArtifactFile` 及其 media/portrait/video 模块 re-export，不保留旧名别名或兼容层。
1046. 图片、立绘、视频 HostPort trait 与全部 Runtime/测试实现统一返回公共文件事实类型，删除按业务复制的结构体构造。
1047. png/jpeg/webp 路径 MIME 推断迁到 `a4n-core::media::image_mime_from_path`，由跨 crate 共享媒体规则成为唯一 owner。
1048. 删除 video provider 私有 `image_mime_from_path` 与 video 模块 re-export；视频 provider 输入和预览直接依赖公共媒体 owner。
1049. 删除图片 Runtime 私有 `reference_image_mime`，参考图读取与正式图片 inspection 统一复用公共图片 MIME 规则。
1050. 删除立绘 Runtime 私有 `reference_image_mime`，立绘预览/生成参考图与正式立绘 inspection 统一复用同一图片 MIME 规则。
1051. Runtime 新增唯一 `inspect_generated_media_file`，统一执行数据目录边界校验、文件读取、MIME 解析、SHA-256 和 byte size 计算。
1052. 图片 Host 删除一套 canonicalize/read/hash/size 装配，直接调用共享文件 inspection owner。
1053. 立绘 Host 删除一套 canonicalize/read/hash/size 装配，直接调用共享文件 inspection owner。
1054. 视频 Host 删除内联扩展名 match 与 canonicalize/read/hash/size 装配，直接调用共享文件 inspection owner。
1055. 三类正式 artifact 的 ID、kind、content type、sha/revision、byte size、temporary 与 expiry 装配收敛到 `GenerationArtifactCoordinator::generated_media_available`。
1056. 图片、立绘、视频生成结果事件删除三份 `GenerationArtifactRef` 字面量和 sha clone 分叉，统一把公共文件事实移动给 coordinator。
1057. mp4/m4v/webm/mov MIME 推断迁到 `a4n-core::media::video_mime_from_path`，共享媒体规则同时成为播放、装配与正式视频 artifact 的唯一 owner。
1058. 删除 Runtime 视频生成私有 `video_mime_from_path` 及两层 re-export，播放与装配不再反向依赖视频生成模块。
1059. 删除只验证旧 Runtime MIME owner 的独立 `video_generation/tests/reference_media.rs`，图片/视频路径 MIME 合同测试归并到公共媒体规则 owner。
1060. 静态搜索确认三个旧 artifact 类型、两套私有图片 MIME、旧 video MIME owner、三份正式 `GenerationArtifactRef` 字面量及跨功能 MIME 反向依赖均无剩余引用。
1061. 本批未发现反射、动态代理、AOP、XML、DI 容器或框架配置中的隐藏 artifact 入口；动态分派仅来自显式 HostPort trait 实现，全部生产实现和测试替身已纳入替换闭包。
1062. 从前端 `toRuntimeMediaSrc` 出发继续 BFS，覆盖 Server `/media`、临时媒体下载路由、Tauri `a4n-media` protocol、Runtime range response、媒体 token 路径绑定、临时媒体上传扩展名白名单及 Canvas/Galgame 媒体持久化入口。
1063. 通用图片、视频、音频路径 MIME 映射收敛到 `a4n-core::media::media_mime_from_path`，共享基础规则成为 Host 文件响应与生成 artifact 的共同 owner。
1064. 路径扩展名 trim、小写化和缺失扩展名错误收敛到唯一 `media_file_extension`，图片、视频、通用媒体不再各自解析路径。
1065. png/jpeg/webp 与 mp4/m4v/webm/mov 严格子集规则复用同一扩展名到 MIME 表，删除映射值的第二份复制。
1066. 删除 Runtime `content_type_for_path` 及整套图片/视频/音频 extension match，Host 响应不再维护第四套 MIME 表。
1067. 删除未知扩展名静默返回 `application/octet-stream` 的伪安全路径；未登记媒体类型现在在读取正文前显式失败。
1068. `read_media_file_response` 引入明确的 `UnsupportedMediaType` 与 `Read` 错误边界，不再把合同错误和文件系统错误压成同一种字符串结果。
1069. Server `/media` 与 `/temp-media` 对不支持类型返回请求错误，对文件打开、metadata、seek/read 失败返回内部错误，不再把 I/O 故障伪装成客户端参数问题。
1070. Tauri `a4n-media` 对不支持类型返回 400，对读取或响应构建失败返回 500，删除此前全部错误一律 500 的状态分叉混淆。
1071. 删除 Runtime media_file 和 video provider media 中重复验证公共 MIME 表的测试代码，合同断言只保留在公共媒体规则 owner。
1072. 静态搜索确认 `content_type_for_path`、媒体响应 `application/octet-stream` fallback、Runtime MIME extension match 与重复 provider MIME 测试均无剩余引用；Server/Tauri 显式路由注册之外未发现隐藏入口。
1073. 从 Video Studio Canvas 的音频上传按钮继续 BFS，覆盖前端 FileReader/base64、`ImportCanvasMediaInput`、command manifest、Server command route、Runtime media-kind dispatch、Orchestrator 音频持久化、ffprobe、media asset/provenance 与失败清理。
1074. 横向比较 Canvas 音频、Galgame 音频、临时媒体上传、Host 媒体响应和图片/视频 artifact 后，确认 MIME 是跨 crate 基础规则；唯一 owner 下沉到新增 `a4n-core::media`，不引入 Server→Orchestrator 反向依赖。
1075. 删除 Orchestrator `media` 模块中的 MIME 函数与 re-export，图片生成、立绘、视频 provider、播放、装配和 Runtime 响应全部直接依赖 `a4n-core`。
1076. 通用路径 MIME 表补齐 Canvas 已公开支持的 AAC/FLAC，严格媒体响应不再把这两类活跃音频误判为未知类型。
1077. 音频 MIME 别名到规范 MIME/扩展名收敛到唯一 `canonical_audio_mime`，MP3/WAV/OGG/M4A/AAC/FLAC 不再由各功能复制映射值。
1078. Canvas 音频 MIME 校验从“空值、octet-stream 或任意 audio/* 均放行”收紧为规范 MIME 与文件扩展名精确一致。
1079. 上传音频缺失 `mime_type` 不再制造 `application/octet-stream` 默认值，直接在输入边界显式失败。
1080. 删除 Canvas 音频 raw base64/Data URL 双格式解析；正式前端与 CLI 均发送 raw base64，旧 Data URL 兼容分支不再保留。
1081. Canvas 音频来源记录统一保存规范 MIME；本地路径来源从唯一扩展名表推断，上传来源从唯一 MIME owner 规范化，删除可空原始 MIME 镜像。
1082. Canvas 音频路径格式解析收敛为一次 `canvas_audio_format`，同时返回规范 MIME 与扩展名，删除同一路径二次 MIME/extension 解析。
1083. Galgame 音频删除 MP3/WAV/OGG MIME 映射副本，复用共享规范化后只保留自身允许格式集合。
1084. Server 临时媒体删除音频规范 MIME/扩展名映射副本，复用共享 owner；原有 MIME 白名单仍显式限制其支持集合，未扩大协议合同。
1085. Canvas 音频 `file_path` 与 `data_base64` 改为严格互斥；删除“存在 data 时静默忽略 path”的来源优先级分支。
1086. Canvas 视频同样改为严格互斥来源，删除“存在 path 时静默忽略上传内容”的相反优先级分叉。
1087. Canvas 音频/视频 path 模式拒绝遗留 `file_name`/`mime_type` 上传字段，避免同一来源维护两套未使用元数据。
1088. Canvas 图片明确拒绝 `file_path`，只接受其唯一实现支持的上传字段，不再静默丢弃 path。
1089. 前端 `ImportCanvasMediaInput` 收紧为判别联合：图片仅上传，音频/视频为上传或 path 二选一，调用代码无法再构造混合来源。
1090. MIME 合同测试迁到 `a4n-core` owner；Canvas 仅保留格式集合/精确配对和来源互斥的最小断言，不复制共享映射测试。
1091. 静态搜索确认 Canvas 音频 octet-stream 默认、任意 `audio/*` 放行、Data URL 兼容解析、Orchestrator MIME re-export 和 Canvas 混合来源静默优先级均无剩余引用。
1092. 本批显式覆盖前端、CLI `--set-base64`、Server command、Runtime、Orchestrator、ffprobe、DB media asset 与临时媒体路由；未发现反射、动态代理、AOP、XML 或 DI 容器中的隐藏导入入口。
1093. 从 Galgame 音频、图片/立绘/视频素材上传和远端临时媒体上传继续 BFS，覆盖浏览器 FileReader、generated Host input、command manifest、Server/Tauri dispatch、Runtime action、Orchestrator 上传规则、媒体记录和 provider 临时上传。
1094. 正式前端上传统一由 `readFileAsBase64` 去除 Data URL 头，CLI `--set-base64` 直接编码原始文件字节；确认后端接受完整 Data URL 只是一条无生产者的旧兼容路径。
1095. 图片上传删除 Data URL 前缀剥离，只接受 `data_base64` 声明的原始 Base64。
1096. Galgame 音频上传删除相同 Data URL 兼容分支，只接受原始 Base64。
1097. 章节视频、装配插入、参考媒体视频和画布视频共用上传源删除相同 Data URL 兼容分支。
1098. 删除零剩余用途的 `strip_data_url_prefix` 及 Orchestrator `media` 公共 re-export，不保留旧名别名。
1099. 图片上传测试删除“raw Base64 与 Data URL 均合法”的双格式断言，只保留正式 raw Base64 合同。
1100. 前端文件读取删除单调用 `readFileAsDataUrl` 与 `dataUrlToBase64`，Base64 正文提取收敛到唯一 `readFileAsBase64`。
1101. 前端图片上传 MIME 白名单删除非标准 `image/jpg`，只允许标准 `image/jpeg`。
1102. Orchestrator 图片上传边界删除 `image/jpg → image/jpeg` 兼容转换。
1103. Server 临时媒体上传删除同一 `image/jpg` 转换；内部图片 MIME 生产者只来自 Core 标准扩展名映射。
1104. New API 首帧 data URL 校验删除 `image/jpg` 别名；外部 provider HTTP 响应探测仍保留该第三方边界兼容，不与内部上传合同混合。
1105. 静态搜索确认上传链已无 `strip_data_url_prefix`、Data URL 双格式说明或内部 `image/jpg` 接受分支；本批未发现反射、动态代理、AOP、XML、DI 容器或隐藏路由入口。
1106. 从媒体文件落盘后的数据库相对路径功能继续 BFS，覆盖 Runtime file_utils、图片/立绘/视频生成与上传、cleanup outbox、Server `/media`、Tauri protocol、素材图、Canvas 音频和 Galgame 音频消费者。
1107. `CanonicalDataPathError`、数据目录 canonicalize 和越界错误语义下沉到共同基础依赖 `a4n-core::files::paths`。
1108. `canonicalize_path_in_data_dir` 下沉到 Core，删除 Runtime 的同名实现，数据目录边界只保留一个 owner。
1109. `canonicalize_data_file_path` 下沉到 Core，空路径、绝对路径、文件类型和越界校验只保留一个 owner。
1110. `data_dir_relative_file_path` 下沉到 Core，媒体数据库相对路径生成只保留一个 owner。
1111. Runtime 原 103 行路径实现替换为 Core owner 的活跃公共导出；Server、Tauri 和 Runtime 内部调用继续使用同一模块入口，但不再维护第二份逻辑。
1112. 素材图上传删除私有 `data_dir_relative_path`，直接复用 Core 路径 owner。
1113. Canvas 音频导入删除第二份 `data_dir_relative_path`。
1114. Galgame 音频导入删除第三份 `data_dir_relative_path`。
1115. `path_to_db_string` 删除 `to_string_lossy` 有损回退；数据库路径无法表达为 UTF-8 时显式失败，不再写入替换字符形成第二身份。
1116. 成片缓存过期扫描显式传播严格路径字符串错误，删除假成功式有损转换。
1117. `ProjectPaths` 增加已有两个生产消费者的 `audio_dir()`，项目音频目录名称只保留一个 owner。
1118. Canvas 与 Galgame 音频落盘删除两处 `root().join("audio")` 手写路径。
1119. 素材图落盘删除手写 `data/projects/{id}/images`，复用既有 `ProjectPaths::images_dir()`。
1120. 静态搜索确认 Orchestrator 已无相对路径私有副本、音频目录手写拼接或数据库路径 `to_string_lossy` 回退；未发现反射、动态代理、AOP、XML、DI 容器或隐藏文件入口。
1121. 从素材图、Canvas 音频和 Galgame 音频的落盘后失败补偿继续 BFS，覆盖上传/路径输入、文件创建、write/copy、metadata、相对路径、ffprobe、DB 记录、Galgame 绑定及全部清理分支。
1122. 横向比较 Runtime `remove_created_file_on_error(_sync)`、三套 Orchestrator 相对路径删除函数和四处吞掉的 `remove_file` 结果，确认创建阶段与正式补偿阶段需要两种明确语义。
1123. 创建过程失败清理 owner 下沉到 `a4n-core::files`，异步与同步调用统一复用一套错误合并和文件删除实现。
1124. Runtime 删除原 44 行创建失败清理实现，仅保留对 Core owner 的活跃公共导出。
1125. Core 新增严格 `remove_existing_data_file`；文件已落盘后的 DB/绑定失败补偿必须定位到真实数据目录文件，不再把缺失文件当作成功。
1126. 素材图删除私有 `remove_profile_material_file`，复用严格补偿删除 owner。
1127. Canvas 音频删除私有 `remove_canvas_audio_file` 及 `NotFound => Ok(())` 伪安全路径。
1128. Galgame 音频删除私有 `remove_galgame_audio_file`，复用同一严格删除 owner。
1129. 素材图 write、metadata 与相对路径生成统一包入创建失败清理 owner，任一步失败都会清理已创建文件并保留二次失败。
1130. Canvas 音频 write/copy、相对路径和 ffprobe 统一包入同一 owner，删除三处 `let _ = remove_file(...)` 吞错。
1131. Galgame 音频 write、相对路径和 ffprobe 统一包入同一 owner，删除一处吞错。
1132. 创建阶段的 `NotFound` 仅在操作可能尚未创建文件时视为无需清理；正式 DB/绑定补偿不共享该语义，删除两类清理的混淆合同。
1133. 素材图与 Galgame 音频完全相同的纳秒时间戳、20 次冲突重试和 `create_new` 文件创建实现下沉到 Core。
1134. 删除素材图私有 `create_unique_timestamped_file`。
1135. 删除 Galgame 私有 `create_unique_timestamped_file`。
1136. 静态搜索确认 Orchestrator 生产代码已无吞掉的 `remove_file`、三套私有补偿删除函数或两套时间戳文件创建器；未发现反射、动态代理、AOP、XML、DI 容器或隐藏清理入口。
1137. 从图片、素材图、Canvas 音频、Galgame 音频和视频导入的前端文件选择入口继续 BFS，覆盖 `readFileAsBase64`、generated input、command manifest、Server/Tauri dispatch、Runtime、Orchestrator、文件落盘和 DB 记录。
1138. 横向比较四类上传规则和 Canvas/视频临时文件/导入图片/生成图片/立绘五套时间戳文件创建器，确认文件名、Base64 解码和唯一文件创建均存在同业务分叉。
1139. 上传文件名校验收敛到 `a4n-orchestrator::media::required_upload_file_name`，统一 trim、空值、无效名称及 `/`、`\\` 路径分隔拒绝。
1140. 删除视频私有 `required_upload_video_file_name` 及公开 re-export，Runtime 上传直接调用唯一 owner。
1141. 删除 Canvas 私有 `required_canvas_audio_file_name`。
1142. 图片上传删除私有 `required_upload_text`，并与音频/视频一致拒绝携带路径的伪文件名。
1143. Galgame 音频文件名改用同一 owner，不再允许把客户端路径文本保存为来源文件名。
1144. raw Base64 的空值、解码、空文件和大小校验收敛到 `decode_uploaded_bytes`。
1145. 共享解码 owner 在分配字节前按文件上限拒绝明显超长 Base64，删除图片/音频/视频各自只在解码后检查大小的重复路径。
1146. 图片上传删除私有 Base64 解码、空文件和大小检查实现。
1147. 视频上传删除同一套实现及零剩余用途的 imports prelude `base64::Engine` 导出。
1148. 上传音频事实收敛为 `DecodedUploadedAudio`，文件名、规范 MIME、扩展名和字节只解析一次。
1149. Canvas 删除私有 MIME/扩展名配对、Base64 解码、空文件和上传大小检查，直接使用共享音频 owner。
1150. Galgame 删除 `ImportedAudioSource` 与完整 `media_rules.rs` 旧模块，上传解析不再维护第二实现。
1151. 256MB 音频上限只保留一份，Canvas 本地路径与 Canvas/Galgame 上传共同使用。
1152. Galgame 只保留 MP3/WAV/OGG 允许集合这一项真实业务差异，通用 MIME/扩展名规则不再复制。
1153. Canvas 音频采用 Core `create_unique_timestamped_file`，删除自定义 PID/Chrono 路径生成器。
1154. 视频上传临时文件采用同一 Core owner，删除第三套时间戳与 20 次 `create_new` 循环。
1155. 导入图片采用同一 Core owner，删除第四套冲突循环。
1156. 生成图片采用同一 Core owner，删除第五套冲突循环。
1157. 立绘采用同一 Core owner，删除第六套冲突循环和私有碰撞状态分支。
1158. Canvas 音频写入已由 `create_new` 创建的文件句柄，本地来源使用 `io::copy`，删除重新按路径打开并可能覆盖同名文件的分叉。
1159. 静态搜索确认生产代码的时间戳冲突重试只剩 Core owner，旧音频上传类型、旧 Galgame 模块和三个私有文件名入口均无剩余引用。
1160. 本批显式覆盖 Server 命令路由、Tauri `invoke_host_command`、command manifest 和前端正式生产者；未发现反射、动态代理、AOP、XML、DI 容器或隐藏配置入口。
1161. 删除视频规则生产模块对仅测试使用文件名 helper 的导入，并把仍写着 Data URL 的旧测试名称改为当前 raw Base64 合同。
1162. Canvas 音频规则模块说明收窄到本地路径扩展名/MIME，删除已迁移到共享上传 owner 的旧职责描述和重复测试导入。
1163. 从导入图片、生成图片、立绘、素材图、画布抽帧、裁剪视频、成片预览和尾帧记录的落盘结果继续 BFS，覆盖创建、metadata、相对路径、DB 记录与补偿错误。
1164. 横向比较 Runtime `persist_created_media_file`、素材图私有实现、裁剪/尾帧/成片三套文件事实及多套 AppError/字符串二次错误装配。
1165. `persist_created_media_file` 下沉到 `a4n-core::files`，文件持久化基础事实不再由 Runtime 独占。
1166. Runtime 删除该函数实现，仅保留 Core owner 的公共导出，现有调用路径无需兼容别名逻辑。
1167. 素材图删除私有 write/metadata/file-size/relative-path 组合，直接调用 Core owner。
1168. metadata、普通文件校验、`u64→i64` 严格转换与数据目录相对路径收敛到 `data_file_record_facts`。
1169. `persist_created_media_file` 内部复用 `data_file_record_facts`，不再复制文件事实步骤。
1170. 裁剪视频删除私有 metadata/size/path 实现，复用共享事实并保留原返回合同。
1171. 尾帧记录删除 `metadata.len() as i64` 无检查转换，文件过大时显式失败而非截断。
1172. 成片预览删除 metadata/size/path 重复装配，读取正文前先经过同一数据目录和普通文件边界。
1173. Orchestrator 二次失败的 `AppError::InvalidState` 装配收敛到 `errors::app_error_with_secondary`。
1174. 删除 Batch Image 私有 `append_secondary_error`。
1175. 删除 Image Generation 第二份同名实现。
1176. Galgame 音频和资产绑定补偿改用唯一 AppError owner。
1177. Canvas 图片、视频、音频补偿改用同一 owner，并删除 `append_import_cleanup_error`。
1178. Portrait 回滚/文件清理错误改用同一 owner。
1179. 字符串主错误与可选二次错误拼接收敛到 `a4n-core::error::append_secondary_message`。
1180. Core 创建失败清理复用字符串 owner，删除内部手写分隔拼接。
1181. Runtime 视频错误拼接删除函数实现，仅保留 Core owner 的模块内导出名。
1182. 素材图删除私有 `append_cleanup_error`，二次失败说明由调用点显式构造。
1183. 空白二次错误不再制造尾随分号；无二次失败时原始错误保持不变。
1184. 静态搜索确认文件持久化、文件记录事实、AppError 二次失败与字符串二次错误均只剩一个实现 owner；未发现隐藏配置入口。
1185. Canvas 抽帧输出改用 Core 唯一文件创建器，删除毫秒时间戳路径分叉；已沿画布节点、Runtime 抽帧、FFmpeg `-y` 覆盖、文件事实和失败清理链静态闭包。
1186. 过期成片缓存相对路径改用 Core `data_dir_relative_file_path`，删除 Runtime 私有路径转换实现。
1187. 章节成片渲染失败只保留同步双文件清理，删除清理完成后再次为已不存在文件登记 outbox 的重复路径及其伪二次错误。
1188. 三条成片渲染调用的文件清理收敛到各自外层持久化 owner，删除共享 renderer 内部的第二次输出删除。
1189. 成片后处理输出清理收敛到 `persist_assembly_record_output`，删除 renderer 内重复删除与二次错误装配。
1190. Canvas 抽帧删除实际命令前的 `ffmpeg -version` 预检，启动失败由抽帧命令唯一报告。
1191. 视频裁剪删除同一预检，stream-copy/recode 命令的 typed error 成为唯一执行结果。
1192. 章节预览、成片推送和画布成片从共享 renderer 删除同一预检，避免每次成片额外启动一个 FFmpeg 进程。
1193. 成片后处理删除同一预检，FFprobe 音频探测与实际 postprocess 命令继续各自显式失败。
1194. Runtime 删除失去全部调用者的 `ensure_ffmpeg` helper 及 `video_files/media/imports prelude` re-export 链。
1195. `a4n-media` 删除只服务预检的 `FfmpegAvailabilityError` 与 `check_ffmpeg_available` 公共 API。
1196. FFmpeg smoke 测试改用标准库直接检查 `ffmpeg` 与 `ffprobe` 两个真实依赖，不再占用生产 API。
1197. 画布成片删除硬编码 `|_| Ok(false)` 静音分叉，改用章节成片相同的真实音频探测与 manifest owner。
1198. 生产 imports prelude 只暴露 canonical manifest builder；可注入音频探测入口收窄为测试夹具路径。
1199. `rg` 静态确认退役 FFmpeg 预检符号、画布硬编码静音 closure、成片缓存私有相对路径和 renderer 内重复删除均无剩余生产引用。
1200. 从成片后处理工作区的插入、删除、替换出发，BFS 覆盖前端 API、Host manifest/dispatch、Runtime 来源准备、Orchestrator 版本链/记录、DB、Media filter/FFmpeg 与测试入口。
1201. `AssemblyPostprocessOperation` 下沉为 `a4n-domain` 唯一事实源，API contract 改为直接 re-export。
1202. 删除 API 层第二份后处理操作 enum 与第二份 `as_str` 实现。
1203. Orchestrator plan/prepared record 改为保存 typed operation，不再保存 `&'static str`。
1204. 删除 `validate_postprocess_operation` 字符串二次校验、trim 分叉及公开 re-export。
1205. 后处理 metadata 操作字段改用 canonical enum serde 解析，删除手写字符串白名单。
1206. Runtime 直接传递 typed operation，删除字符串转 Media enum 的三分支映射与不可达错误。
1207. `a4n-media` 删除 `VideoPostprocessOperation` 第二枚举，Media request 复用 Domain enum。
1208. 基底时长删除 `.max(0.1)`，非正、NaN 与无穷值在建记录/媒体执行前显式失败。
1209. 插入/替换来源时长改为必需正有限数，删除静音场景可缺失和结果时长 `unwrap_or(0.0)` 分叉。
1210. 插入与删除/替换的互斥时间字段改为严格合同，删除操作同时拒绝来源时长。
1211. 删除/替换范围删除 `base + 0.05` 容差、`min(base)` 截断与 Media `clamp` 自动改写。
1212. 删除整段在持久化前显式失败，删除先创建伪造 `0.1` 秒记录再由 Media 失败的路径。
1213. 输出时长由纯规则一次计算，删除 `.max(0.0)`、`.max(0.1)`、`unwrap_or(0.0)` 和未知操作基底时长回退。
1214. 基底宽高与 fps 在建记录前严格校验；Media 删除 `1280x720@24` 默认值并返回 typed error。
1215. 删除操作若携带已有或上传来源会显式失败；上传来源判定不再把已出现的空字段当作未上传。
1216. 后处理 filter 删除首尾 `0.01` 秒静默丢弃阈值，严格保留所有非空片段。
1217. 音频段时长删除 `.max(0.1)`，后处理时间字面量删除非法值 `→ 0` formatter。
1218. 删除公开 `ffmpeg_duration_literal` 及非法值 `→ 0.1` 回退，统一格式化已验证秒数。
1219. 成片拼接新增 clip 时长正有限校验，非法静音片段不再伪装为 `0.1` 秒。
1220. 相关单元/smoke 测试代码改用唯一 Domain enum，并增加严格媒体属性、空结果范围和 clip 时长断言；未执行测试。
1221. `rg` 静态确认旧 enum、旧字符串 validator、旧 range helper、旧 duration formatter、空结果 error 与目标链 max/clamp/default 均无剩余引用。
1222. 从章节成片预览、分段推送和画布成片出发，BFS 覆盖 clip 查询、Runtime 上下文、manifest、Media assembly、预览 DTO 与两类记录持久化。
1223. 新增 `assembly_media_properties` 唯一纯规则，统一校验全部片段并一次计算目标宽高、fps 与总时长。
1224. 删除 `normalized_assembly_dimensions`、`normalized_assembly_fps` 和 `even_positive_dimension` 三个宽松 helper。
1225. 章节成片空列表不再伪造 `1280x720@24` manifest；共享上下文改为显式 `None`。
1226. 拼接/推送把空上下文转换为明确业务错误，缓存读取保留 `Ok(None)` 的真实缺席语义。
1227. 所有片段的非正尺寸、非正 fps、非正/非有限时长在 manifest 或记录创建前显式失败。
1228. 目标奇数尺寸只保留编码所需偶数化，尺寸加一溢出显式失败。
1229. 成片总时长删除逐片段 `.max(0.0)`，累加为非有限值时显式失败。
1230. 成片推送记录复用唯一属性 owner，删除 first/default/max 私有计算分叉。
1231. 画布成片计划复用同一 owner，删除宽高/fps `.max(1)` 与时长 `.max(0.0)`。
1232. 章节预览上下文保存 canonical 总时长，预览 DTO 删除第二次宽松求和。
1233. 旧默认值测试改为严格属性合同测试，覆盖奇数尺寸、空列表与零时长拒绝；未执行测试。
1234. `rg` 静态确认旧 normalization helper、目标链负时长归零和宽高/fps 最小值回退均无剩余引用。
1235. 从视频画布 `ExtractFrameNode` 出发，BFS 覆盖前端创建/编辑/保存、Canvas 持久化与迁移、Orchestrator 输入/计划、Host Port 注入、Runtime、Media FFmpeg 和图片记录。
1236. 在 `a4n-domain` 建立唯一 `CanvasFrameKind::{First, Tail, Time}`，统一 canonical serde、显示与严格解析。
1237. Canvas 节点输入、Host 输入、Orchestrator 计划和 Runtime action 全链改传 typed enum，删除 `frame_kind: String`、clone 与重复字符串解释。
1238. `ExtractFrameNode` 只读取必填 `frame_kind`，删除旧键 `kind` 和缺失时默认 `first` 的运行时兼容。
1239. 新增一次性 `canvas_extract_frame_contract` 迁移，将存量空配置和旧别名写回 canonical 值，并删除旧键及非时间模式的遗留 `at_secs`。
1240. 迁移对未知类型、非对象配置和 `time` 模式非法时间显式失败，并注册为最新 required schema；迁移未执行。
1241. 前端新建抽帧节点时显式保存 `{ frame_kind: "first" }`，创建阶段成为默认值唯一 owner。
1242. Inspector、节点摘要和拖拽预览删除各自的 `first` 回退与硬编码默认副本。
1243. 删除 `normalize_canvas_frame_kind` 及 `first_frame/last/last_frame/end/key/key_frame/at_secs` 等运行时别名。
1244. 抽帧源视频时长必须为有限正数，删除负时长 `.max(0.0)`。
1245. 首帧/尾帧携带 `at_secs` 时显式失败，不再静默忽略无关字段。
1246. 指定时间抽帧必须提供有限非负时间，且严格早于视频结束点。
1247. 删除抽帧时间允许超过视频时长 `0.5` 秒的容差路径。
1248. Canvas 尾帧时间与 FFmpeg `-sseof` 共用 Domain 尾部偏移常量，删除独立 `0.08` 字面量和任意 `0.12` 短视频阈值。
1249. Media 指定时间抽帧删除负时间归零，使用独立 typed error，避免缩略图/尾帧调用者增加不可达分支。
1250. 抽帧执行前严格校验源视频宽高，图片记录删除 `.max(1)` 伪造尺寸。
1251. canonical enum 直接负责文件名、prompt、metadata 和 generation spec 序列化，删除中间字符串往返。
1252. 相关测试代码改用 canonical 值并增加严格边界断言；遵照资源约束未执行测试。
1253. `rg` 静态确认旧 normalization、运行时别名、默认回退、时间容差、负时间归零和目标记录尺寸兜底均无剩余生产引用。
1254. 从章节裁剪与 Canvas `TrimVideoNode` 两个功能入口开始完成 BFS，覆盖前端、配置、命令、Server、Canvas Host、Runtime、Orchestrator、DB、文件持久化、FFmpeg、迁移和失败清理路径。
1255. Canvas 裁剪 `start_secs`、`end_secs` 改为严格有限 f32 数字解析；缺失字段、null、字符串、越界值不再被当成缺失或静默转换。
1256. 新增一次性 `canvas_trim_contract` 迁移：存量缺失 `start_secs` 显式写入 `0`；已存在的非法 start 值直接阻断迁移；不为 `end_secs` 创建默认值。
1257. 新建 `TrimVideoNode` 时由前端显式写入 `{ start_secs: 0 }`，默认值只存在于创建阶段，不存在于运行时兼容分支。
1258. 章节裁剪 UI 删除 `end=3` 默认；节点摘要和拖拽预览删除 `start=0`、`end=?` 展示回退及占位文本。
1259. `trim_video` command manifest 为起止时间声明 `minimum: 0`，与后端严格范围合同保持一致。
1260. Runtime 先加载源视频并完成项目权限校验，再把已加载记录传入 Orchestrator，删除重复查询和异步计划内重复读取。
1261. Orchestrator 要求源视频时长为有限正数、尺寸和 fps 为正数，结束时间不得超过真实时长，删除 `+0.5` 秒伪安全容差。
1262. Orchestrator 复用现有 workflow canonical 判定：项目级源视频只接受已登记 Canvas workflow，章节级源视频拒绝 Canvas workflow，未知 mode 显式失败。
1263. 裁剪输出路径不再强制要求 `chapter_id`，项目级 Canvas 视频写入项目视频目录，章节视频继续写入章节目录。
1264. 删除确定性 `video_id/start/end` 毫秒文件名和 `millis_label`，改用 Core 唯一时间戳文件创建器，避免相同范围覆盖已有记录指向的文件。
1265. Runtime 统一使用已校验计划中的起止时间和借用的源文件路径，删除无必要的路径 clone 与二次时间来源。
1266. FFmpeg 裁剪删除 stream-copy 首次执行及失败后重编码 fallback，只保留 `libx264/aac` 精确重编码这一事实标准。
1267. `FfmpegTrimError` 删除 `CopyUnavailable`、`RecodeUnavailable` 两个分叉，收敛为无效范围、命令不可用、命令失败三类真实结果。
1268. 裁剪输出文件大小和相对路径改用 Core `data_file_record_facts`，删除 Runtime 内重复的 metadata、大小转换和路径计算实现。
1269. 旧的 chapter-id helper 测试删除，替换为章节目录和项目目录两条真实输出路径合同测试源码。
1270. `canvas_trim_contract` 已加入迁移模块、执行顺序和最新 required schema marker；迁移本身未执行。
1271. 静态 `rg` 闭包确认旧 helper、毫秒标签、双 FFmpeg 错误、stream-copy、时间容差、Canvas/章节默认回退和旧路径调用均无剩余生产引用。
1272. 普通 diff 人工检查确认章节入口与 Canvas 入口只共享一套 Runtime/Orchestrator/Media 裁剪实现，未新增兼容、回退、兜底或占位逻辑。

- 第 1254—1272 项仅使用源码阅读、BFS、`rg`、普通 diff 与 `apply_patch`；累计完成 19 项 TrimVideoNode 收敛，遵照资源约束未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本；后续验证须经用户明确同意并复用仓库现有 `target/`。
1273. 完成章节视频、参考媒体、成片衔接片段和 Canvas 视频导入四条入口的 BFS，覆盖上传临时文件、本地路径、MIME/扩展名、FFprobe、记录、复制、完成状态、尾帧和失败清理链。
1274. 新增 `probe_imported_video_properties`，集中负责 FFprobe 结果到 Orchestrator `ImportedVideoProperties` 的唯一字段映射。
1275. 四条导入路径全部复用该映射，删除四份逐字段复制代码及不再需要的类型导入；各入口独有业务合同保持不变。
1276. `rg` 静态确认 `ImportedVideoProperties { ... }` 仅剩唯一 owner，未新增 fallback、兼容或占位逻辑。

1277. 从章节导入、参考媒体、成片衔接片段、Canvas 导入/成片、assembly push 和 postprocess edit 七条入口向 FFprobe、完成事务、时长回写、重复采用、cleanup outbox、尾帧清理、失败删除及 Orchestrator/DB 反向 BFS，确认完成逻辑只有一个 Runtime owner。
1278. 将 `mark_imported_video_completed` 重命名为 `complete_video_record`，删除“导入”对 assembly 与成片后处理路径造成的伪语义。
1279. 将完成重复分支的“导入视频文件”日志收敛为通用“视频文件”诊断，不改变清理、采用或错误传播逻辑。
1280. `rg` 静态确认旧符号无生产引用，7 个调用者全部指向 `complete_video_record`，未新增兼容别名、回退或占位逻辑。

1281. 从完成失败、导入复制失败、首帧尾帧失败和 assembly 输出持久化失败四条分支反向 BFS，确认 `delete_video_record_error` 只有视频记录失败补偿调用者，不读取导入 source。
1282. 将唯一删除/清理错误适配器从 `imports/source/deletion.rs` 移到 `video_records/deletion.rs`，收回到视频记录删除 owner。
1283. 将名称收敛为 `delete_video_record_cleanup_error`，消除导入模块的伪归属。
1284. `rg` 静态确认旧文件、旧模块和旧符号均无生产引用，删除事务、尾帧清理与二次错误合并保持原路径，未新增兼容、回退或占位逻辑。

1285. 对通用完成、未采用文件清理、FFprobe 和探测错误映射的全部调用执行静态 BFS，确认其 owner 是 `video_records` 而非 `imports/source`。
1286. 将 `complete_video_record` 与 `enqueue_unadopted_video_file_cleanup` 移到 `video_records/completion.rs`，删除旧 source completion 模块。
1287. 将 `probe_video_properties` 与 `video_probe_error_message` 移到 `video_records/probe.rs`，assembly 后处理和完成生命周期直接依赖唯一 owner。
1288. 导入 source probe 只保留 `ImportedVideoSource → ImportedVideoProperties` 映射，删除通用探测和错误映射职责。
1289. 删除 assembly、trim 和 import 子模块通过 `imports`/`source` 取得通用完成、清理与探测函数的转发依赖。
1290. `rg` 静态确认旧 completion 文件、source 通用 probe 实现及跨模块转发引用无生产剩余；未改变函数体语义，未新增兼容、回退或占位逻辑。

1291. 从章节导入、参考媒体、衔接片段、Canvas 导入/成片、assembly push 与后处理七条落盘路径 BFS，确认两套 output path helper 仅有章节级/项目级目录差异。
1292. 新增唯一 `video_record_output_path`，使用显式 `Option<chapter_id>` 选择 canonical 视频目录。
1293. 将通用输出路径 owner 收回 `video_records/paths.rs`，删除导入 source 的错误归属。
1294. 五条章节级路径显式传 `Some(chapter_id)`，两条 Canvas 项目级路径显式传 `None`，不再靠函数名称隐含范围。
1295. 删除 `import_video_output_path`、`import_project_video_output_path` 及其 re-export/prelude 依赖，不保留兼容别名。
1296. `rg` 静态确认旧函数无生产引用，七个调用者仅依赖一个路径实现；目录和文件名行为未改变。

1297. 从 Canvas provider 落盘、项目级裁剪和项目级视频记录输出三条路径 BFS，确认都重复 `ProjectPaths::videos_dir + create_dir_all`。
1298. 在视频媒体路径 owner 建立唯一 `ensure_project_videos_dir`。
1299. Canvas provider 持久化、项目级裁剪与 `video_record_output_path` 全部复用该目录 helper。
1300. 删除三处直接目录创建副本及 Canvas/项目两套错误文案。
1301. `rg` 静态确认 Runtime 项目视频目录创建只剩唯一实现，章节目录仍由既有 `ensure_chapter_videos_dir` 负责。

1302. 从视频更新、播放、手工轮询、历史 provider 请求预览、assembly push 状态更新和裁剪入口向 Server dispatch、Runtime 权限门禁、Orchestrator、DB 及完成补偿反向 BFS，区分六类必选查询与两类可选权威状态查询。
1303. 将 Orchestrator 已有 `required_video_record` 公开为必选视频查询唯一 owner。
1304. `update_video` 改用唯一必选查询。
1305. 播放与 scrub 播放改用唯一必选查询。
1306. 裁剪入口改用唯一必选查询，权限顺序保持不变。
1307. 外层手工轮询入口改用唯一必选查询。
1308. 历史 provider 请求预览改用唯一必选查询。
1309. assembly push 状态更新改用唯一必选查询。
1310. 删除 Runtime `required_generated_video` 第二实现及其死 import。
1311. 删除 Runtime 两套不存在错误文案，统一由 Orchestrator `AppError::NotFound` 表达。
1312. 完成与裁剪补偿保留仅有的两处可选查询，避免破坏事务提交确认和清理语义。
1313. 内部手工轮询直接复用外层已授权记录，删除同一命令第二次 DB 查询。
1314. `video_records` 删除 `records::*` 通配导出，仅显式公开三个动作。
1315. 从 Core 视频状态常量向 Runtime、Orchestrator 与 DB 生命周期执行横向 BFS，区分 generated-video 与 chapter-video-prompt 状态。
1316. Canvas 成片失败写入改用 canonical failed 常量。
1317. 裁剪失败写入改用 canonical failed 常量。
1318. Orchestrator 失败资产谱系同步条件改用 canonical failed 常量。
1319. DB provider job running 写入改用 Core 常量。
1320. DB 视频完成幂等与完成写入改用 Core completed 常量。
1321. DB 终止状态白名单改用 Core failed/interrupted/cancelled 常量。
1322. DB 启动恢复查询改用 Core pending/running 常量。
1323. DB 启动恢复中断写入改用 Core interrupted 常量。
1324. DB 等待状态校验改用 Core pending/running 常量。
1325. DB 完成前状态门禁改用 Core pending/running 常量。
1326. DB 新记录初始状态与编辑/时长门禁改用 Core running 常量。
1327. DB 删除引用时的替代视频查询改用 Core completed 常量。
1328. provider 与 Canvas 完成补偿的 cancelled/interrupted/failed 判定改用 Core canonical 常量。
1329. `rg` 静态确认旧必选 helper、旧错误文案及本批完成/DB 生命周期范围内的目标 generated-video 状态字面量无生产剩余；剩余同名字面量仅属于提示词状态或测试输入。

1330. 从 Core generated-video 状态模型向 Orchestrator 改名导出、公共 re-export、Runtime、provider、Canvas、时间线、assembly、章节连续性和测试执行 BFS。
1331. 确认六个状态别名与终态函数别名没有独立协议或校验语义，所有消费者均可直接依赖 Core owner。
1332. 删除 `job_status_rules` 六个 `GENERATED_VIDEO_*` 改名导出。
1333. 删除 `is_terminal_generated_video_status` 改名别名。
1334. 删除 `video/mod.rs` 对七个别名的二次公开转发。
1335. job status 纯规则直接使用 Core 状态常量和终态函数。
1336. Orchestrator Context 生命周期推进和进度门禁直接使用 Core owner。
1337. Runtime 完成、裁剪和 Canvas 成片失败路径直接使用 Core owner。
1338. Canvas provider 完成/取消/中断与等待状态推进直接使用 Core owner。
1339. provider execution 轮询、完成采用、迟到清理和 workflow 同步直接使用 Core owner。
1340. 视频 generation 单项/批量结果直接使用 Core owner。
1341. provider generation 定向测试删除旧别名依赖。
1342. 时间线三类 completed 判定统一使用 Core 常量。
1343. 视频选择、采用、后处理、Canvas 成片和 assembly clip completed 判定统一使用 Core 常量。
1344. 章节连续性来源视频 completed 判定改用 Core 常量。
1345. assembly push 行完成判定改用 Core 常量，metadata 状态域保持独立。
1346. provider 请求快照状态分组改用 Core 常量，不改变 interrupted 既有提示语义。
1347. 提示词状态字面量属于独立 `chapter_video_prompt` 状态域，继续保留。
1348. workflow/checkpoint 状态字面量属于独立状态域，继续保留。
1349. `rg` 静态确认七个旧别名全仓零引用，生产 generated-video 状态事实只剩 Core owner。

- 第 1273—1276 项仅使用 BFS、源码阅读、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本。

- 第 1277—1280 项仅使用 BFS、源码阅读、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本。

- 第 1281—1284 项仅使用 BFS、源码阅读、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本。

- 第 1285—1290 项仅使用 BFS、源码阅读、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本。

- 第 1291—1296 项仅使用 BFS、源码阅读、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本。

- 第 1297—1301 项仅使用 BFS、源码阅读、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本。

- 第 1302—1329 项累计完成 28 项，仅使用 BFS、源码阅读、`rg`、普通 diff 与 `apply_patch`；遵照用户资源约束，即使超过 10 项也未自动运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本；集中验证须经用户明确同意并复用仓库现有 `target/`。

- 第 1330—1349 项累计完成 20 项，仅使用 BFS、源码阅读、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本；集中验证须经用户明确同意并复用仓库现有 `target/`。

- 第 1185—1199 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；累计完成 15 项收敛，未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本；集中验证须经用户明确同意并复用仓库现有 `target/`。
- 第 1200—1221 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；累计完成 22 项收敛，未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本；集中验证须经用户明确同意并复用仓库现有 `target/`。
- 第 1222—1234 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；累计完成 13 项收敛，未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本；集中验证须经用户明确同意并复用仓库现有 `target/`。
- 第 1235—1253 项仅使用源码读取、BFS、`rg`、普通 diff 与 `apply_patch`；累计完成 19 项收敛，未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本；后续验证须经用户明确同意并复用仓库现有 `target/`。
- 第 943—971 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；遵照用户资源约束，未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 第 972—988 项继续只使用源码读取、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 第 989—1020 项继续只使用源码读取、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 第 1021—1042 项继续只使用源码读取、`rg`、普通 diff 与 `apply_patch`；遵照用户资源约束，未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 第 1043—1061 项继续只使用源码读取、`rg`、普通 diff 与 `apply_patch`；累计完成 19 项收敛后仍未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本，等待用户同意后复用仓库 `target/` 集中验证。
- 第 1062—1072 项继续只使用源码读取、`rg`、普通 diff 与 `apply_patch`；再次累计完成 11 项优化，仍未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 第 1073—1092 项继续只使用源码读取、`rg`、普通 diff 与 `apply_patch`；累计完成 20 项导入/MIME 收敛，仍未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 第 1093—1105 项继续只使用源码读取、`rg`、普通 diff 与 `apply_patch`；累计完成 13 项上传合同收敛，仍未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本。
- 第 1106—1120 项继续只使用源码读取、`rg`、普通 diff 与 `apply_patch`；累计完成 15 项路径事实收敛，仍未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本。
- 第 1121—1136 项继续只使用源码读取、`rg`、普通 diff 与 `apply_patch`；累计完成 16 项落盘/补偿清理收敛，仍未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本。
- 第 1137—1162 项继续只使用源码读取、`rg`、普通 diff 与 `apply_patch`；累计完成 26 项媒体上传/唯一文件创建收敛，仍未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本；集中验证须经用户明确同意并复用仓库现有 `target/`。
- 第 1163—1184 项继续只使用源码读取、`rg`、普通 diff 与 `apply_patch`；累计完成 22 项媒体文件事实/二次错误收敛，仍未运行 Cargo、npm、测试、构建、数据库、迁移、lint、fmt、浏览器、`git diff --check` 或任何验证脚本；集中验证须经用户明确同意并复用仓库现有 `target/`。
- 本轮 BFS 覆盖前端 generation client、Server REST/SSE、Tauri Channel、十一类 Runtime start action、Orchestrator reservation/session/terminal、MemoryTaskQueue permit、PostgreSQL active-attempt/retry lineage、Server graceful shutdown、Tauri Exit 与 Tokio Runtime 生命周期。
- 插入片段专项 BFS 额外覆盖前端 gap 构造/生成/提交/移动/删除、生成合同与 schema、视频提示词 worker、workflow fingerprint/checkpoint、Core/DB timeline 邻接约束和 provider recipe 消费。
- 图片结果专项 BFS 额外覆盖 Scene/Object/Profile/Images/Manga/Video Material/Frame Chain 八类前端功能、portrait/video 横向实现、正式 artifact 文件 inspection、generated image/portrait/video 查询与项目/章节权限边界。

## 保留的边界

- HTTP 传输层仍在反序列化后立即校验 artifact 引用并脱敏错误；能力 trait 可被注入其他实现，因此统一 payload 提取入口也继续执行同一安全清洗。两处分别保护“不可信网络输入”和“不可信能力实现”，不属于业务规则分叉。
- RAG、Embedding、图片和视频的 progress/completed payload 结构及业务合同不同，均在各自消费者拒绝不属于本能力的事件，因此未强行合并为宽泛事件抽象。
- Prompt 的“project_id 与 root_project_id 必须同时存在”是 Prompt 项目覆盖业务约束，强于通用协议约束，继续由 Prompt 入口负责。
- Build-feedback 的 cold/warm 隔离 target 是该基准测量的被测条件，继续保留在仓库 `target/build-feedback/tmp` 并由 trap 清理；它不是普通构建入口的缓存 fallback。
- Windows release smoke 继续使用 PowerShell `Start-Process` 管理 Windows 原生进程，但 release 版本、asset 与 SHA 全部来自公共 Qdrant metadata；该平台进程 API 差异不是第二份发布配置。
- CLI local 的 `database_endpoint` 还负责 URL 语法、host、默认端口和地址解析，远强于宿主启动时的数据库 scheme 合同；远端部署 Shell 则必须在停服和替换前独立拒绝非 PostgreSQL 配置。两者均处于独立信任边界，不是可删除的 Rust helper 副本。
- Remote Sidecar 的 `service_version` 仍允许与 Host 包版本不同，因为它表示独立部署构建元数据；其 protocol 与 capability 已改为精确匹配。Managed Sidecar 继续额外要求同一 workspace 构建版本，二者不是兼容逻辑分叉。
- Service Manager 的通用进程监督测试仍可使用未注册 Demo 服务，因此共享健康校验保留“无 capability 期望”参数；Runtime 生产配置全部来自八项注册服务，生产路径始终传入精确 capability。
- 章节生成 RAG 与增强 RAG 仍保留各自的 query 扩展、时间过滤、融合和结果投影，因为前者服务章节写作上下文、后者服务通用检索 API；本批只合并无业务差异的 identity/vector 校验与写入记录装配。
- 写作台第一章的增强检索会实际进入项目级待处理伏笔候选，而项目级 Host 搜索也允许 `chapter_number = None`；将 `Some(1)` 改成“第一章之前”会把当前可见候选直接改为空集合，属于业务行为变化而非可证明冗余，因此本批明确保留。
- `OrchestratorContext::store_embedded_rag_records` 仍保留为 Runtime 客户端上传的跨 crate 活跃边界；删除其只在 crate 内转发的两个旧别名不影响该生产入口。
- 客户端 prepare 与 upload 继续分别校验 embedding identity、canonical 文本 hash、向量数量/维度及上传 chunk_id；二者位于“不可信客户端生成前”和“实际持久化前”两个独立边界，本批只删除由内部 canonical 集合不变量决定的重复/不可达检查。
- Linux `use-same-proxy` 继续只在操作系统明确启用该设置且 HTTPS 未单独配置时复用 HTTP 代理；这是平台真实配置，不是通用跨协议 fallback。
- `HF_HOME_OVERRIDE_LOCK` 继续保护进程级环境变量的临时修改与恢复；fastembed 5.x 会让 `HF_HOME` 覆盖 `with_cache_dir`，该锁保护真实并发边界，不属于可删除同步冗余。
- LLM/图片/视频 baseline 的 Runtime existence gate 与 DB 写入前 existence recheck 继续同时保留：前者保证数据库已有配置时不读取、不校验已失效的 seed 环境变量，后者处理首次检查与实际写入之间其他 Host 已完成插入的常见竞争窗口；两者不是同一业务判断的重复 Facade。
- Embedding 连通性仍保留 Embedded provider 调用与 Sidecar HTTP 调用两条真实传输路径；本批只合并两者完全相同的单向量合同，不删除 Sidecar 身份维度网络边界校验。
- RAG 写入、搜索与客户端上传继续在数据完整性边界校验数量、维度和有限数；连接测试只证明单次 provider 可用性，不能替代持久化前校验。
- `EmbeddingConnectivityPreviewRequest` 继续作为不含 Base URL、API Key 和本地路径的展示投影；它不是正式 Sidecar payload 的重复别名。
- FastEmbed cache pointer 在 Windows 仍保留“符号链接权限不可用时移动 blob”的平台分支；普通 Windows 用户默认可能没有 symlink 权限，该分支是正式缓存落盘实现，不是对损坏数据的静默恢复。
- 30 秒 pending cancellation 继续保留：前端会先展示可取消状态并发出业务请求，Server 还需完成参数解析、权限/范围 preflight 后才注册 TaskHandle，“取消先于注册”是可达生产竞态而非伪安全 fallback。
- `TaskManager` 与 `MemoryTaskQueue` 继续分离：前者只拥有短生命周期 Host progress/cancel handle，后者拥有持久化队列、优先级、并发槽位、数据库终态和执行许可；强行合并会制造宽泛任务抽象。
- Tauri `GenerationSubscriptionRegistry` 继续独立：其 ID 表示窗口拥有的 Channel 订阅，不是业务 task ID；单调订阅 ID、窗口销毁取消和 worker finish 已形成完整且无字符串复用的生命周期。
- Tauri 本机 Embedding 下载继续使用本机取消令牌表：该命令没有 Server TaskManager progress 事件，前端进度来自模型缓存轮询；本批只统一身份规则、重复注册和结束清理不变量。

## 验证状态

- 本轮从普通设置页与管理员 API 配置页的 Embedding 配置、测试、预览和模型管理按钮出发完成 BFS，覆盖前端 scope/Runtime 模式、Tauri 命令、Server dispatch、Runtime 配置解析与队列、Embedded/Dynamic/Sidecar provider、Embedding Service handler、Capability 客户端、协议 payload、模型缓存目录和 RAG 下游校验边界。
- 已使用 `rg` 静态确认 `normalize_api_base_url`、`effective_fastembed_cache_dir`、`fastembed_cache_dir_from_config`、`normalized_fastembed_backend_name`、`local_provider_cache_key`、Dynamic Embedding 专用 `test_connection` override 与 `default_fastembed_cache_dir` 命名均无剩余活跃引用。
- 第 872—894 项只使用源码读取、`rg`、普通 diff 与 `apply_patch`；遵照用户资源约束，未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 本轮继续从 Embedding 模型大小、下载、断点续传、缓存完整性、Tauri 任务注册和 FastEmbed provider 初始化向下 BFS，覆盖 Runtime in-cache/Sidecar 分流、FastEmbed metadata/cache/transfer/progress、Embedding Service feature 路由与本机取消令牌生命周期。
- 已使用 `rg` 静态确认 `remove_path`、`model_cache_status`、`model_cache_is_complete`、`build_text_embedding_inner`、可变 `show_download_progress`、`put_model` 和本机 managed state 的 Clone/Arc 包装均无剩余活跃引用。
- 第 895—911 项继续只使用源码读取、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 本轮从前端 `createWorkflowTaskId`、`runProgressTask`、Queue/Embedding 取消按钮出发完成任务生命周期 BFS，覆盖 Server 14 处 Web task 注册/完成调用及其宏展开后的 21 个命令、Core TaskManager、Runtime task access/query/cancellation/projection、Orchestrator 持久队列、MemoryTaskQueue、Generation DB UUID、Tauri Embedding 下载表、Tauri generation Channel 订阅表及 CLI/command dispatch 可达入口。
- 已使用 `rg` 静态确认 `required_task_id`、Embedding `models::scope`、`PendingCancellation`、`cancel_registered_task`、Runtime `get_task_handle`、六个 AppState 任务转发方法、TaskManager `active_count`/Debug 与按 task ID 完成删除路径均无剩余活跃引用。
- 第 912—942 项只使用源码读取、`rg`、普通 diff 与 `apply_patch`；严格遵照资源约束，未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 本轮从设置页 LLM、图片、Embedding 连通性按钮与请求预览出发完成 BFS，覆盖前端 scope/快照、本地桌面分支、Host 合同、Server dispatch、Runtime config resolution/profile/queue、Capability DI、LLM Gateway probe、图片 provider 请求和 Embedding provider 测试，并横向复核全部 `LlmProvider` 实现与测试替身。
- 已使用 `rg` 静态确认 `llm_probe_succeeded`、`probe_with_config_id`、`LlmProvider::test_connection`、LLM `test_connection_call_count`、`testServerLlmConfig`、`previewServerTestLlmConfigRequest`、`testServerImageConfig`、`previewServerTestImageConfigRequest`、`ModelDiscoveryScope` 和编辑器 `serverScope` 均无剩余引用；全仓剩余 `.test_connection()` 仅属于独立的 Embedding provider 合同。
- 第 848—871 项只使用源码读取、`rg`、普通 diff 与 `apply_patch`；遵照用户资源约束，未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 本轮从设置页三类模型检测按钮出发完成端到端 BFS，覆盖前端 scope/密钥、Host 合同、Server dispatch、Runtime saved-key 与 profile、Capability DI/HTTP、三类 Sidecar 路由、Service Protocol secret policy、共享 endpoint/auth/parser/HTTP、provider registry/default URL、环境文档与夹具。
- 已使用 `rg` 静态确认 `discover_llm_models`、`discover_image_models`、`discover_video_models`、`model_discovery_payload`、`validate_model_discovery_request`、`detectServer*ConfigModels`、`withLocalRuntimeDiscoveryKey`、`model_discovery_endpoints`、`openai_model_list_urls`、`ModelDiscoveryParser::OpenAi`、`dedupe_discovered_models` 及三处 discovery payload clone 均无剩余引用。
- 第 816—847 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；遵照用户资源约束，未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 本轮从前端 LLM/图片/视频设置页与 Runtime 配置快照出发，BFS 覆盖本机设置持久化、个人 API Key 合并、服务器默认模板投影、模型发现、Host 配置 API、`AppState::build` 启动初始化、Runtime baseline 环境解析、Orchestrator settings application、DB existence/insertion、provider record input 与 `.env.example` 生产配置说明。
- 已使用 `rg` 静态确认 `BaselineLlmConfigSeed`、`BaselineImageConfigSeed`、`BaselineVideoConfigSeed`、`PublicLlmSeed`、baseline 私有 `required_seed_value`/`positive_i32`/`app_error_plain_message`/`normalize_optional_string`、三套默认模板手写投影和三套系统密钥直接读写均无剩余活跃引用。
- 第 787—815 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；遵照用户资源约束，未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 本轮从 FastEmbed 的 HF Hub/reqwest client 构造向显式环境、managed Sidecar `env_clear`、Linux/macOS/Windows 系统代理和缓存模型初始化反向 BFS；确认系统代理只需补齐缺失协议，显式环境应由原生客户端直接消费。
- 已使用 `rg` 静态确认 `merge_download_proxy`、`explicit_proxy_env`、`first_non_empty_env`、旧覆盖行为测试、平台代理 host/port `unwrap_or_default`/`unwrap_or(0)` 及三个平台转发 wrapper 均已清零。
- 第 775—786 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；遵照用户资源约束，未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 本轮继续从设置页 `handleDownloadModel` 出发，BFS 覆盖 task ID 生成、前端 Host 分流、generated schema、Server/Tauri 命令、任务注册/取消表、Runtime queue、Capability event HTTP、Sidecar 断连取消、FastEmbed metadata/文件传输/模型初始化与最终缓存状态；确认所有活跃下载路径都具备真实且必填的取消令牌。
- 已横向追踪 `download_fastembed_model`、`download_embedding_model_to_cache`、`download_embedding_model_events` 与 completed payload 的全部声明、生产调用和夹具，确认目标模型摘要可从 FastEmbed 唯一状态 owner 直接贯通到 Host，下载后全模型扫描与二次定位没有独立消费者。
- 已使用 `rg` 静态确认 Embedding 下载范围内的 `normalize_optional_embedding_download_task_id`、`cancel_token: Option<CancellationToken>`、`cancel_token.unwrap_or_default()`、可选下载 task ID、`Some(cancel_token)`、`downloaded_model_summary`、旧 metadata 收集 Facade 和下载文件 `downloaded` 镜像字段均已清零。
- 第 760—774 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；遵照用户资源约束，未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 本轮从前端设置页的 Embedding 模型管理功能出发，BFS 覆盖 Host/API schema、Server/Tauri、Runtime action/queue、Capability DI/HTTP transport、Sidecar 路由/handler/event、FastEmbed cache/download/provider、环境装配、独立进程夹具与 Cargo 依赖；确认四类纯值/传输结构镜像、路由/环境字面量和 JSON 中转均无独立业务语义。
- 已使用 `rg` 静态确认 `FastEmbedLocalModel`、`FastEmbedDownloadInfo`、`EmbeddingTextsPayload`、`EmbeddingTextsOutput`、双向 identity 转换、旧 Service contracts、CWD 缓存回退、生产路由字面量及旧 typed↔Value 中转均已清零。
- 第 740—759 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；遵照用户资源约束，未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器或任何验证脚本。
- 已使用 `rg` 静态确认退役符号和旧错误回退均无生产引用。
- 已逐文件复核 import、可见性、trait 装配与调用方向。
- 本轮从 WritingDesk 章节生成与本机向量准备入口出发，BFS 覆盖 Orchestrator application/preparation/context、章节 RAG query/filter/execution/fusion/formatter、共享增强检索校验、VectorStore trait、Memory/Qdrant dense/sparse/hybrid 实现、Host/API 合同及相关 fixture；确认旧统计包装、projection、重复分类、双分数和 source-score 回退无活跃执行路径。
- 本轮继续从 WritingDesk `startChapterGeneration` 向本机默认 Embedding 配置、prepare Host command、Runtime/Orchestrator 启动校验、章节查询 fingerprint、client vectors 与服务端 provider 路径双向 BFS；确认默认配置缺失、空查询旁路、错误字符串分类和静默切换服务端均不是需要保留的生产合同。
- 本轮从 `rag_enhanced_search`、`rag_prepare_enhanced_search` 与 `rag_enhanced_search_with_vectors` 三个 Host 入口向 Runtime preparation、Orchestrator query expansion/filter/execution/fusion、伏笔和角色数据库读取、Embedding identity、VectorStore 及相关 fixture BFS；确认增强查询结构上限固定为 8，重复 truncate、规则标签参数和 `Debug` 能力没有独立生产语义。
- 本轮从章节生成本机 prepare 与正式 start 两个入口向 query fingerprint、prepared identity、客户端向量预校验和 store 执行校验双向 BFS；确认向量数量、维度和有限数只以服务端 prepared 合同为期望事实源，客户端 identity 仅作为不可信实际输入参与匹配。
- 本轮从 NovelList/NovelDetail 的 `ragIngestAll` 出发，BFS 覆盖桌面本机 embedding 与 Server embedding 两条部署路径、generated Host 合同、command manifest、Server dispatch/task progress、Runtime prepare/upload/full ingest、expected records、identity/hash/vector 校验、Orchestrator store/delete 和 Sidecar operation trace；确认 `rebuild` 是真实全量重建能力，继续保留，镜像写入统计、重复 metadata prepare、深复制向量和第二套上传去重状态均无独立语义。
- 本轮从 `RagIngestProgress` 向 Server reporter、取消 token、全量同步和无进度的材料/场景同步反向 BFS；确认生产进度只按 canonical record 单步推进，可选转发 helper、任意 amount 和无 reporter 时的提前字符串格式化均可删除。
- 本轮再次从增强检索三个 Host 命令向 access、preparation 和两类执行路径 BFS；确认三者共享完全相同的项目 Editor gate，已由 preparation 唯一负责。
- 第 642—657 项只使用源码读取、`rg`、普通 diff 与 `apply_patch` 完成；未运行测试、构建、Cargo、npm、数据库、lint、格式化、浏览器或任何验证脚本。
- 本轮继续从客户端/服务端 ingest 的 expected records 向章节模型读取、运行模式、章节分块、语义 embedding、DP/matrix、RAG metadata、prepared store 与 stale cleanup 双向 BFS；确认 prepare rebuild 回显、借用写入深复制、名称复制、分块诊断 metadata、单字段包装和固定关闭 overlap 均无活跃消费者。
- 已从 `RagChunkConfig` 全部声明和调用点向生产 Runtime 快照、章节分块入口与测试反向搜索；确认七项参数没有环境、数据库、Host、CLI 或前端来源，唯一生产值就是原 Default，故删除假配置并保留两个跨实现共享硬约束。
- 已从句子拆分的 200/150 字上限向 DP 的 3—15 句、100—1200 字约束推导全部生产候选范围；确认最小三句最大 600 字，忽略字符限制的第二遍选择不再承担可达生产场景。
- 第 658—688 项继续只使用源码阅读、`rg`、普通 diff 与 `apply_patch`；未运行测试、构建、Cargo、npm、数据库、lint、格式化、浏览器或任何验证脚本。
- 已从章节 RAG record producer 向实时章节同步、全量 expected-record ingest、metadata 规范化、同步分类、RAG 搜索 API、WritingDesk metadata 展示、Host 合同、配置与文档反向 BFS；确认两个章节来源字段只有生产写入和镜像测试，未参与过滤、更新判定、展示或协议声明。
- 已从 `normalize_metadata` 的全部写入字段向 Orchestrator/Runtime 三条同步分类入口、VectorStore payload、增强搜索原样输出、WritingDesk 展示、Host schema 与全仓配置反向 BFS；确认 `source_generation`、`hash_algorithm`、`content_length` 均无独立消费者或协议语义。
- 已从 Blueprint/World/Relationship/Outline/Character/Protagonist/Scene/Continuity 全部 record builder 向 Runtime expected records、实时同步、统一 metadata 规范化、SearchFilter 构造、章节生成 formatter/fusion、增强搜索 API 和 WritingDesk 标题投影反向 BFS；确认删除字段均只有 producer 或镜像测试，真实消费者只需要 `data_type`、时间字段和展示标题。
- 已从章节 paragraph/summary/analysis producer 向 `analysis_rag_items`、章节生成允许类型/formatter、Qdrant 生产过滤、增强搜索输出和前端投影反向 BFS；确认 `content_kind` 与嵌套 `analysis` 仅被构造，Qdrant 对任意 analysis key 的覆盖只存在于通用过滤器测试。
- 已从章节记录的 `chapter_id` 写入向 root family metadata 装配、Core family 校验、同步/客户端上传和 source chapter 过滤字段反向 BFS；确认唯一 canonical 字段为 `source_chapter_id`，旧 `chapter_id` 只承担一次临时搬运。
- 已从 Orchestrator 所有 add/query/search/delete/metadata/drop 调用向 VectorStore collection、Sidecar payload/envelope、Qdrant physical collection、Memory collection、search filter 与章节 formatter 双向 BFS；确认生产 collection 始终是 `project_id.to_string()`，metadata `project_id` 没有独立路由语义。
- 第 689—719 项继续只使用源码阅读、`rg`、普通 diff 与 `apply_patch`；未运行测试、构建、Cargo、npm、数据库、lint、格式化、浏览器或任何验证脚本。
- 已从 Blueprint/World/Relationship/Character/Protagonist/Outline/Chapter/Scene/Continuity 全部生产 RAG record producer 出发，向同步准备、provider/embedded 写入、metadata 规范化、VectorStore collection、增强检索、章节生成检索及 Runtime Sidecar envelope 逐层 BFS；确认生产写入全部进入 root-text 项目 collection，没有 snapshot/derived RAG producer，旧 family 字段只有固定写入、过滤镜像和测试夹具。
- 已静态搜索 `root_text_family_metadata`、`root_text_metadata`、`snapshot_text_metadata`、`validate_project_family_metadata`、`normalize_metadata`、`identity_scoped_search_filter`、`build_chapter_generation_rag_search_filter` 与测试 `embedding_record` 的全部声明和调用，确认第 720—726 项删除后的旧符号、旧 import 与旧参数数量均已清零。
- 第 720—726 项仅使用源码阅读、`rg`、普通 diff 与 `apply_patch`；未运行测试、构建、Cargo、npm、数据库、迁移、lint、格式化、浏览器或任何验证脚本。
- 已从 `SidecarVectorStore` 的构造与动态 trait 注入向 Orchestrator add/query/search/delete/drop/metadata/completeness 全部生产调用、Runtime service manager、RAG capability transport、Sidecar payload handler 与 ServiceRequest context 逐层 BFS；确认八类生产 operation 的 collection 均由正整数 `project_id.to_string()` 生成，不存在全局或命名 collection 生产路径。
- 已从 RAG handler 的 request clone 向 `ServiceRequest::result_response`、trace ID 所有权、mutation future、NDJSON 终态和 payload derive 反向搜索；确认请求 DTO 的 `Clone/Debug` 与七次整批 clone 没有独立消费者。
- 已从 `.a4n-rag-mutation-in-progress` 向启动恢复、进程测试、脚本、文档、文件读取 API 与全部 mutation 调用点反向搜索；确认 marker 内容零读取者，唯一活跃合同是文件存在、`sync_all` 和成功处理后的删除。
- 已从 `A4N_RAG_SERVICE_DATA_DIR` 向 managed service 公共环境装配、source/portable 发布启动、独立进程配置与 `.env.example` 反向 BFS；确认活跃启动均显式配置，CWD `data/` 不属于生产路径。
- 已从 ingest/delete/metadata/drop 的 Sidecar producer 向 capability `JsonResponse`、Runtime `response_payload`/generation event parser、`VectorStore` 返回类型和进程样例双向 BFS；确认四组单字段/空 wrapper 均为立即包入再拆出的第二合同，现统一直接使用 `usize`、metadata map 与 `()`。
- 已静态确认非数字 collection 全局回退、payload clone、RAG contract `Clone/Debug`、旧 error mapper 闭包、marker operation 参数/内容、CWD data 默认值、七个输出 DTO、`outputs.rs` 和旧 wrapper JSON 字段均无剩余引用。
- 第 727—739 项仅使用源码阅读、`rg`、普通 diff 与 `apply_patch`；未运行测试、构建、Cargo、npm、数据库、迁移、lint、格式化、浏览器或任何验证脚本。
- 已再次使用 `rg` 覆盖所有 Sidecar 生产者、Runtime 消费者、NDJSON 原始样例、脚本和前端，确认退役事件种类及拆散字段无剩余引用。
- 已使用 `rg` 静态覆盖发布工具的 Rust、Ruby、Shell、PowerShell、Tauri 与 CI 消费者，确认 `TOOL_BINARIES`、`TOOL_PACKAGES`、`a4n-cli:a4n`、三工具固定数组、静态工具 `externalBin` 和旧参数回退无剩余活跃引用。
- 已使用 `rg` 静态覆盖临时媒体 API、Runtime Provider 输入、环境变量、部署脚本、cleanup outbox 与进程 fixture；确认请求 TTL、旧公网变量、Canvas 自动模式、data URI 兼容和本地环境解析器无剩余活跃引用。
- 已使用 `rg` 静态覆盖 Shell、PowerShell、Ruby 与 workflow 的 catalog 读取和临时目录；确认内联 service catalog 解析、重复 schema 正则、`GetTempPath`、`RUNNER_TEMP` 及浏览器截图系统 `/tmp` 路径已从本批活跃入口移除。
- 已使用 `rg` 静态覆盖 `A4N_PUBLIC_APP_URL` 的环境、部署、Runtime、Server、路由和 fixture 消费者；确认两个本地 URL normalizer、Server `Uri` 解析和 Runtime IP 判定均无剩余引用。
- 已对全部公共 Shell 库消费者、仍自行解析仓库根的检查/开发入口及其 CI 调用点执行专项 BFS；确认本批 `ROOT`/`repo_root`/`target_root`、`CARGO_TARGET_DIR` fallback 和三套私有命令探测无剩余引用。
- 已从 build-feedback workflow 向下 BFS 覆盖工具自检、测量脚本、预算脚本、baseline TSV、timing/dependency 产物和 fixture；确认方法名、默认 package 库存、回归阈值及固定计数均只由 baseline/runs 派生。
- 已从 Qdrant 安装/启动功能向外 BFS 覆盖 CLI local 生命周期、RAG live test、`.env.example`、Docker Compose、工具目录配置与机器状态文件；确认 Docker 服务、版本覆盖、配置生成器、环境默认副本及无配置 fallback 无剩余引用。
- 已从 ONNX Runtime 安装入口与 CLI local loader 双向 BFS 覆盖公共 Shell 库、build/deploy、README、`.env.example` 和全部版本字面量；确认固定版本仅存在于 `tools/onnxruntime/VERSION`。
- 已继续从 Qdrant release 下载向 CI 反向 BFS，覆盖 Sidecar matrix、Linux/Windows/macOS release smoke、全部 GitHub workflow URL、Docker image/service、asset 名、digest、解压与 readiness loop；确认 Docker Qdrant 路径已清零，版本/asset/SHA 仅存在于 VERSION 与 releases TSV。
- 已从 CLI local 的 Qdrant 启动决策向二进制发现、环境变量、PATH、源码根、portable 包和文档反向搜索；确认退役 `A4N_QDRANT_BIN`、PATH Qdrant 与 `resolve_qdrant_binary` 无剩余引用。
- 已从 CLI local 的 Server/migrator 解析向 release tool catalog、portable/Windows 打包、macOS `Contents/MacOS` 布局、release manifest 生成/校验和三平台 smoke 反向 BFS；确认正式发布均将 `a4n`、`a4n-server`、`a4n-migration` 放在同一目录，系统 PATH 不属于活跃发布合同。
- 已从 managed sidecar 解析向 `A4N_SIDECAR_BIN_DIR`、portable 启动脚本、release manifest、macOS externalBin、三平台 smoke 与历史安装清理记录反向 BFS；确认 `/lib/a4n/services` 仅剩带日期的历史测试输出，不存在生产生成器、文档入口或活跃消费者。
- 已从 Prompt sidecar 环境注入向 CLI 子进程启动、Runtime Prompt engine、managed service 环境、portable 打包、macOS Resources、Tauri external resources、中央 Registry 与第 149 项安装器清理记录反向 BFS；确认 `share/a4n/prompts` 无生产生成路径，活跃资源只来自源码/portable `prompts/` 或 macOS `Resources/prompts`。
- 已从 Runtime `sidecar_binary_path` 向 service catalog、managed service 配置、`scripts/run.sh` 源码构建、portable 启动器、Tauri externalBin 准备/打包、三平台 smoke 与 App portable home 配置反向 BFS；确认 sidecar 二进制位置只需专用环境目录或当前 Host 的 portable/同目录布局，Resources、系统 lib 与 portable-home 推断均无独立活跃场景。
- 已继续从 Tauri externalBin 输入命名向 `prepare-tauri-external-bin.sh`、动态 bundle config、最终 macOS `Contents/MacOS`、portable manifest、三平台 release smoke 与 Runtime 文件扫描反向 BFS；确认 target-triple 后缀只在构建输入目录存在，最终可执行目录与 Runtime 合同均使用 catalog 的无后缀平台名称。
- 已横向搜索 Runtime、CLI、Core 与各 Sidecar 的环境路径解析器，确认 Runtime 已依赖 Core 且 managed service 配置本身返回 `Result<String>`；因此 sidecar 目录读取可直接复用 Core 严格 owner，无需私有空值语义或吞错适配层。
- 已横向覆盖 CLI 配置路径、local 生命周期的 ONNX/sidecar/source/target 路径及 Cargo 依赖边界；确认 CLI 未依赖 Core，为单 helper 新增 crate 依赖没有收益，因此在 CLI 内将两份复制实现收敛到现有 `config.rs` owner。
- 已从 Runtime sidecar 文件定位向唯一生产调用、错误文案消费者、service catalog 与 release smoke 反向搜索；确认多目录泛型入口及临时文件测试设施没有第二个生产调用或独立合同，可由精确发布路径检查替代。
- 已从 `A4N_PORTABLE_HOME` 与 `A4N_DATA_DIR` 向 Core portable 根、AppConfig 默认目录、Tauri 配置文件/环境覆盖、Server 配置、CLI local 子进程、portable 启动脚本及 macOS/Linux/Windows smoke 反向 BFS；确认前者唯一表示包根，后者唯一表示可写数据根，四类存储目录派生已收敛到 Core。
- 已从 Prompt engine 与 managed Prompt sidecar 的共同资源入口向 CLI 子进程、Server executable、portable release manifest、macOS `.app/Contents/Resources`、Tauri resources、DATA_DIR、CWD 与中央 `_registry.yaml` 反向 BFS；确认自动资源定位只需 Runtime owner，发布布局可严格判定，CLI 探测和 EXE_DIR 候选均无独立活跃路径。
- 已横向搜索所有 Rust crate 中 managed sidecar、binary dir、Prompt dir 与 release manifest 环境键的生产者、消费者、错误文案和测试夹具；确认这些键属于 service protocol 跨进程合同，字面量现仅保留在协议 owner，Shell/PowerShell 启动器作为语言边界继续使用同值配置。
- 已从 Tauri `load_config`、配置文件路径、环境覆盖、早期日志初始化与测试模块向 Core env/path owner 反向 BFS；确认 Tauri 私有 `env_string`、`env_path`、两份 `validate_env_value` 及其测试均无独立业务规则，现由 Core 唯一解析合同覆盖。
- 已从 Server `from_env` 与 `into_app_config` 向数据库/JWT、Web 地址、前端目录、OpenAI、bootstrap admin 和存储路径全部消费者横向搜索；确认三项私有 helper 仅复制 Core 行为，没有独立校验边界，可直接删除。
- 已从 PostgreSQL URL 校验向 Server、Tauri 配置文件/环境覆盖、Runtime 启动、Migration 正式/测试入口、CLI local、远端部署脚本、CI 环境值及 Cargo 依赖反向 BFS；确认所有 Rust scheme-only 分支已收敛至 Core，剩余 CLI 强校验与 Shell 预校验均有独立活跃调用路径。
- 已从 `A4N_RELEASE_MANIFEST_PATH` 向 CLI 自动注入、portable/PowerShell 启动器、ServiceManager 构造、Sidecar 启动前哈希校验、manifest 生成/校验脚本与 release smoke 反向 BFS；确认 Service Manager 只需“未设置则不启用、已设置则严格读取”的单一路径合同，Unicode 错误分叉和 CWD 包根回退均无活跃业务场景。
- 已从 Core `portable_home_dir` 与 `AppConfig::default` 向 Tauri 本地/远程启动、Server 存储覆盖、Runtime storage summary、Prompt 资源定位、AppImage、源码启动器及三平台 portable 启动脚本反向 BFS；确认宽松路径 helper 只剩 Core 内三处调用且无独立合同，默认四目录可由一次数据根解析完整派生。
- 已从 Tauri `remote_api_only` 与 Runtime `managed_sidecars_enabled` 向源码启动脚本、portable 默认环境、service catalog、Sidecar 配置装配及现有单测横向 BFS；确认两者使用完全相同的布尔词表，Runtime 的可选字符串读取也仅复制 Core 环境边界，可统一删除而不影响 API transport 枚举解析。
- 已从 `runtime_service_configs` 的 managed 分支向监听地址环境、loopback 端口保留、service catalog、平台文件名、显式 binary dir、Host 同目录布局和进程启动反向 BFS；确认 catalog entry 已携带唯一 binary 事实，地址探测和服务名二次查表均无独立消费者，Host 路径错误也不应降级成普通缺文件。
- 已从 `apply_common_sidecar_env` 向七项路径环境、Prompt locator、RAG/Embedding 服务入口、动态库加载、Qdrant、代理/证书、`ManagedServiceConfig.env` 与 `Command::envs` 反向 BFS；确认子进程环境当前是 String 协议，因此应在唯一装配边界严格拒绝无法表达的值，而不是吞错或有损改写。
- 已从 Runtime Prompt engine 与 managed Prompt Sidecar 的共同 locator 向显式 Prompt dir、portable home、macOS Resources、portable release manifest、DATA_DIR 上级、CWD 和 `_registry.yaml` 反向 BFS；确认发布布局判定依赖可执行文件路径，路径解析失败不能降级为源码候选，只有“成功解析且未识别为发布包”才允许开发目录。
- 已从管理员存储路径摘要向 Qdrant Index `from_env`、RAG Sidecar、Embedded full profile、子进程环境透传、CLI local、`.env`/CI 与前端展示反向 BFS；确认 collection 默认只需 RAG Core 一份，URL/API key/collection 的读取错误不应由展示层或 Index 各自吞掉。
- 已从孤儿项目目录列表与清理命令向数据库项目 ID、cleanup outbox、持久化媒体引用、API DTO、前端展示、`remove_data_dir_in_data_dir` 和 Core `ProjectPaths` 反向 BFS；确认 `path` 仅是展示信息，删除权威路径必须由 canonical project ID 与数据根重建。
- 已从 OAuth2 authorize/callback 命令向 provider 参数校验、GitHub/Google 凭证环境、具体 Provider 构造、登录持久化和前端认证流程反向 BFS；确认凭证 factory 没有第二个生产消费者，双重 provider match 与私有必填环境 helper 均可删除。
- 已从 Server/Tauri 日志初始化向 `RUST_LOG`、Runtime 动态日志命令、reload handle、stdout/file layer、panic hook、Server smoke 与 `.env` 反向 BFS；确认两宿主只在默认 filter 和输出层上不同，环境读取与过滤表达式解析属于同一 Runtime 合同。
- 已继续从前端 `set_log_level` 命令向 Tauri command registry、Server dispatch、管理员权限、Runtime filter parser、tracing reload layer 与两个 Host 静态状态反向 BFS；确认 reload handle 生命周期和错误语义完全相同，外层 `RwLock<Option<_>>` 没有独立并发或禁用状态用途。
- 已从 `AppState::new`、Server/Tauri Host 初始化向 Runtime Profile 枚举、构建 features、CLI/source/portable 环境注入、minimal bundled sidecar 默认和 profile 能力校验反向 BFS；确认三方需要同一显式配置快照，Host 差异仅是未设置时的日志与 Tauri sidecar 注入副作用。
- 已从 service catalog 的 `addr_env/default_addr` 向 Runtime managed 地址分配、remote URL 派生、九个 Sidecar 入口、Bootstrap 实际 bind 与非 loopback 认证校验反向 BFS；确认地址语法解析完全相同，只有最终进程绑定边界需要额外认证约束。
- 已从八个正式 Sidecar 与 Demo 的启动入口向 catalog 地址、资源策略、Service Manager token 注入、Bootstrap Router 合并、业务路由、`/healthz`、`/metrics`、`/shutdown` 和进程测试反向 BFS；确认正式服务地址只由 catalog 提供，认证 token 只在启动期解析一次且所有非健康路由只经过同一认证实现。
- 已从九个 Sidecar 的日志初始化向 Tokio main、滚动文件、后台清理器、保留配置、文件元数据和 macOS release smoke 反向 BFS；确认日志清理任务始终运行在现有 Runtime 内，退役 runtime/时间戳/数值饱和回退无其他调用场景。
- 已从 `/healthz`、`/metrics`、`/shutdown` 向 Service Protocol、catalog serde、八个正式 Sidecar、Demo、Capability 客户端、Runtime 配置、Service Manager、Release Manifest Ruby CLI、三平台 smoke、历史版本矩阵和前端健康摘要反向 BFS；确认固定 A4N 路径只由 protocol 常量声明，`ManagedServiceConfig` 不再提供自定义健康路径。
- 已从 `ManagedSidecarProcess::stop/force_stop` 向 Supervisor command channel、健康启动、重启清理、优雅关闭 HTTP、drain timeout、强制 kill、Runtime Capability 构建和远程 TLS Client 反向 BFS；确认强制终止只保留在请求失败或进程挂死的真实恢复场景，URL、Client 与 token 解析均已收敛到单一 owner。
- 已从 Sidecar profile 的远程 URL 向 `A4N_REMOTE_<SERVICE>_*` 环境、共享 loopback token、CA/client identity、Runtime 配置、Service Manager 校验和 Capability HTTP Client 反向 BFS；确认远程配置不再经测试 Facade 或隐藏 token fallback，CA-only 配置不能冒充部署凭证。
- 已从远程 Sidecar URL 向 Runtime 环境解析、Service Manager 校验、Capability 映射、请求上下文、provider secret 策略和删除的手写 URL parser 双向 BFS；确认 URL 语法、loopback 与非本地 HTTPS 规则只剩一个 owner。
- 已从 `ManagedServiceConfig` 向全部构造器、Runtime profile 装配、Supervisor spawn、健康/关闭端点、Debug、serde 与测试夹具 BFS；确认 `args`、`health_path`、旧序列化默认和单调用默认函数均无活跃生产依赖。
- 已从管理员 `health_check` 向数据库探测、Service Manager 状态、Orchestrator 响应、Server/Tauri dispatch、前端 API 类型和 Overview 展示 BFS；确认 Rust 响应与前端合同字段一一对应，退役别名、状态镜像和未知值 fallback 无剩余引用。
- 已从 Runtime 的 `data_dir/services` 参数向 Orchestrator PID 读取、Service Manager 子进程状态、portable `start.sh/start.ps1`、Linux/Windows/macOS release smoke、CLI local 生命周期及全仓 `.pid` 写入者反向 BFS；确认 Sidecar PID 文件只有测试夹具生产，不属于任何活跃部署合同。
- 已从 Image/Video/RAG/Embedding/LLM 的事件 HTTP 入口向请求校验、任务执行、mpsc 生命周期、断连取消、NDJSON 序列化、Capabilities 流消费和 Runtime progress/completed 解包横向 BFS；确认有界通道、失败响应、message progress 与 typed completed 现各自只有一个传输 owner，服务仅保留差异化业务任务。
- 已从 Image/Video/RAG progress producer 向 Runtime 实际消费者、回调/消息通道、服务单测和进程测试反向搜索；确认 `current`/`total` 无生产读取者，删除后不需要兼容转换或占位字段。
- 已继续从 `CapabilityTaskEvent` 全部构造入口向五类 Sidecar producer、Bootstrap NDJSON、Runtime parser、Capabilities transport、测试替身和 JSON schema 反向 BFS；确认生产代码已无通用 event-kind builder，Progress/Chunk/Completed/Failed 均由语义构造器唯一装配。
- 已从 Embedding 下载事件向 fastembed progress producer、Sidecar 序列化、Runtime 解包、Host 下载任务和前端进度消费者双向 BFS；确认 `DownloadProgress` 可作为共享严格合同，Completed 的 `{models}` 包装没有独立消费者或兼容需求。
- 已从 `AppState.managed_sidecars` 向所有 `AppState` 克隆、cleanup worker、Tauri Exit、Server graceful shutdown、CLI TERM 和 Service Manager Supervisor 反向 BFS；确认共享同步结构必要，Server 绑定与关闭顺序已收敛到明确生命周期。
- 已从八类 HTTP Capability Client 向 Transport 构造器、Service Registry、资源策略、远程 TLS Client factory、健康检查与 JSON/事件请求反向 BFS；确认所有生产 Client 都有注册服务身份，匿名 Transport 与未知服务默认策略无活跃场景。
- 已从 Bootstrap NDJSON producer 向五类事件 Sidecar、Capabilities bytes stream、Runtime 消费者、原始事件 fixture 与 Content-Type 读取点横向 BFS；确认媒体类型、终态、换行和空行规则均只剩一个严格传输合同。
- 已从 `ServiceRequest`、`ServiceResponse`、`ServiceHealth`、`ServiceVersion` 向全部 Axum `Json` 路由、reqwest 反序列化、测试支持和进程入口 BFS；确认未知 Envelope 字段仅由已删除的 Prompt 兼容断言接受，没有生产消费者。
- 已从 capability version 注册表向 Runtime 服务 URL 解析、八个 Sidecar 健康构造器、Capability Transport、Service Manager、release manifest、service catalog、CI security gate 与历史版本矩阵反向 BFS；确认 `.1`/same-major 协商、Host 版本头、compatibility 信封、历史 probe 和矩阵 workflow 均无剩余引用。
- 已使用 `rg` 静态确认 `ServiceCompatibility`、`VersionRange`、`service_compatibility`、`known_service_health_for_host`、`host_protocol_version`、`SIDECAR_HOST_PROTOCOL_VERSION_HEADER`、旧未知字段接受 helper 及 historical matrix 符号全部清零。
- 已从普通 JSON Transport 向八类 Capability trait、Runtime 动态 `JsonRequest/JsonResponse` 消费者、Prompt/Import/Export typed Client、请求 header 注入、上下文校验和响应 artifact 清洗 BFS；确认 typed/dynamic 两条路径只需一个泛型 HTTP 实现，退役转换 helper 与拆包 API 无剩余引用。
- 本批仅使用 `rg`、逐文件读取、普通 diff/status 和可审查补丁完成静态复核；未运行测试、构建、lint、格式化、数据库检查或脚本验证。
- 遵照用户的资源约束，本轮未运行 Cargo、npm、数据库、迁移、构建、浏览器测试或零散测试；集中验证执行前另行通知。
- 本轮 RAG BFS 覆盖 `a4n-runtime` 客户端上传/增强搜索、`a4n-orchestrator` context、RAG write/provider/embedded/records、sync、chapter-generation RAG、enhanced search 及其测试/公开 re-export；已用 `rg` 确认退役别名、重复校验和空结果 helper 无活跃引用。
- 本轮只进行了源码读取、`rg` 符号核对和普通 diff 静态复核，未执行测试、构建、lint、格式化、数据库或浏览器验证。
- 已从前端本地桌面 embedding 的 prepare/upload 调用向 Host command manifest、Server dispatch、Runtime client ingest context、expected records、同步分类、Orchestrator write/delete、RAG metadata 与 API 严格合同执行 BFS；确认 rebuild 不消费旧 metadata，canonical 记录重复只由 `ExpectedRagRecords` owner 检查，外部上传重复/成员/hash/vector 校验仍完整保留。
- 已从章节 RAG 生成、实时同步和章节清理三条路径横向搜索全部 `chapter:{project_id}:{chapter_id}:` 生产者，确认共享前缀现只由 `chapter_rag_chunk_prefix` 构造；章节段落、摘要、分析记录仅追加各自 suffix。
- 本批继续只使用源码读取、`rg`、普通 diff 与可审查补丁；未运行 Cargo、npm、数据库、迁移、构建、浏览器、lint、格式化或任何测试命令。
- 已从 Blueprint/World/Relationship/Character/Protagonist/Outline/Chapter/Scene/Continuity 全部 RAG 记录构造器向同步、章节清理、Runtime expected-records 与客户端上传反向 BFS；确认类型前缀、exact ID 和 prefix 只由 `rag/keys.rs` 与 `RagDataType::as_str()` 共同生成，旧 `sync/keys.rs` 和手写项目级 chunk 前缀无剩余引用。
- 已从 `DeleteRagRecordsInput` 向 Orchestrator context、VectorStore trait、Memory/Qdrant、RAG Sidecar 严格 JSON、Runtime Sidecar Client、独立进程 fixture 与 live 样例双向 BFS；确认 `project_id/data_type/chapter_number` 删除过滤字段、`InvalidDeleteInput`、单调用 filter Facade及旧 JSON 字段均已清零。
- 已从 Qdrant logical→physical collection 映射向所有 add/query/search/hybrid/delete/metadata/drop/completeness 路径逐层检查；确认 physical collection 一对一隔离后，顶层 `project_id` 与 `logical_collection` payload/index/filter 没有任何原始查询、脚本或生产消费者。
- 已从 `VectorStore::hybrid_search` 向 Memory/Qdrant/Disabled/Test Store、RAG Sidecar 合同与处理器、Runtime HTTP Client、Orchestrator 搜索和 live 样例 BFS；确认无效 `vector_weight` 全链清零，未使用公开 RRF 与原始分数死状态已删除，仅保留实际执行的 Memory RRF 和 Qdrant 原生 RRF。
- 本批累计完成第 564—584 项后仍遵照资源约束，仅执行 `rg`、源码读取、普通 diff 与 `git diff --check`；未运行测试、Cargo、npm、数据库、构建、lint、格式化或浏览器命令。
- 本批从 WritingDesk/RagQuickPanel 出发，BFS 覆盖前端远程/本机 embedding 分支、generated Host 类型、command manifest、Server 字符串 dispatch、Runtime 三个增强搜索命令、Orchestrator query expansion/filter/fusion、章节生成共享 caller-vector 搜索、VectorStore embedded/sidecar 入口及相关测试引用；旧 DTO 字段、旧 execute helper、纯转发 contracts 模块、重复 `project_id/pull_k` 与前端镜像状态均已用 `rg` 静态确认无剩余活跃引用。
- 本批同时复核伏笔数据库的 `pending_foreshadowing_before_chapter`/`pending_foreshadowing_for_project`、章节生成 query/reference-context 两条消费者和写作台第一章初始化路径，确认第一章项目级候选是当前可见检索行为，未误删为 fallback。
- 遵照用户“累计多项优化后再集中验证”的资源约束，第 585—604 项仅使用源码读取、`rg`、普通 diff 与可审查补丁；未运行 Cargo、npm、数据库、构建、lint、格式化、浏览器或任何测试命令。
- 遵照用户的资源约束，第 605—622 项继续只使用源码读取、`rg`、普通 diff 与可审查补丁；未运行 Cargo、npm、数据库、迁移、构建、lint、格式化、浏览器或任何测试命令。
- 遵照用户的资源约束，第 623—635 项继续只使用源码读取、`rg`、普通 diff 与可审查补丁；未运行 Cargo、npm、数据库、迁移、构建、lint、格式化、浏览器或任何测试命令。
- 第 1350—1379 项从章节视频提示词功能入口执行 BFS，覆盖 Host 字符串合同、Orchestrator 门禁、DB prompt 同步/查询/依赖、generated-video 生命周期、项目进度、资产谱系和 migration；确认 prompt 六态由 Core 独立持有，删除 prompt 对 workflow 状态常量的跨域借用，并修复合法 `interrupted` 被 canonical 谱系跳过的问题。
- 第 1350—1379 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 第 1380—1398 项从删除章节视频提示词命令执行 BFS，保留权限、依赖规划和最终写事务三个必要快照，删除最终事务与 prompt sync 内按 ID 重查模型的冗余；底层单删/批删、DB 单条 policy 和 Orchestrator 单条别名已收敛为唯一批量删除路径。
- 第 1380—1398 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 第 1399—1449 项从章节视频规划、帧/最终提示词、参考媒体和 provider 请求预览入口执行 BFS，覆盖 Core、DB、Orchestrator、Runtime、前端 API 与视频页；提示词类型、workflow mode、合同版本和前端模式类型均已收敛到各自唯一 owner，纯转发类型/函数/常量别名已删除。
- 第 1450—1466 项从章节/参考媒体/assembly/Canvas/图片导入入口执行 BFS，统一使用 `a4n_domain::EXTERNAL_IMPORT_MODEL`，删除旧参考媒体视频类型判断和专用导入 model 分叉，并通过一次性 migration 规范化持久化旧值。
- 第 1399—1466 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 第 1467—1495 项从章节、参考媒体、assembly、后处理和 Canvas 视频功能入口执行 BFS，覆盖 Core、DB、Orchestrator、Runtime、Host 字符串合同、前端工作区与测试夹具；八类 `generated_video.video_type` 已分别收敛到 Core 和前端 API 的唯一 owner，页面级与规则级别名已删除。
- 第 1467—1495 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 第 1496—1521 项从章节混合成片与 Canvas 生成/导入/裁剪/成片入口执行 BFS，四个活跃 `generated_video.workflow_mode` 已收敛到 Core 和前端 API owner；无生产路径的 `canvas_reference_media` 已从白名单与前端类型删除，测试夹具中的类型/模式错误复用也已拆开。
- 第 1496—1521 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 第 1522—1539 项从 Canvas 文本/图片/视频/编辑节点执行 BFS，四类 `generation_spec.target_kind` 与 `canvas_node` target entity 已收敛到 DB generation catalog；写入、项目包 remap、导出/复制和测试夹具均直接引用唯一 owner。
- 第 1522—1539 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
- 第 1540—1552 项从 Canvas RenderText/LlmText 节点执行 BFS，修复前端 `canvas_text` 与后端 `canvas_prompt` 的默认分叉，并将五个真实 LLM 输出类型纳入 prompt-artifact 合法目录；前后端各自只保留一个合同 owner。
- 第 1553—1567 项从 Canvas 图片生成与导入执行 BFS，`generated_image.image_type=canvas_image` 已收敛到 Domain/前端 owner；DB 重复系统管理图片类型目录和无调用转发 re-export 已删除。
- 第 1540—1567 项仅使用源码读取、`rg`、普通 diff 与 `apply_patch`；未运行 Cargo、npm、测试、构建、数据库、迁移、lint、格式化、浏览器、`git diff --check` 或任何验证脚本。
