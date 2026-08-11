# Embedding 连接测试与本地模型管理 BFS 冗余清理记录

日期：2026-08-11
编号范围：R-1459～R-1491

## 搜索起点与覆盖范围

以“设置页测试 Embedding 配置、列出/查询/下载/取消/删除本地 FastEmbed 模型，以及桌面端本机嵌入”为功能起点，按 BFS 顺序覆盖：

1. 前端：`EmbeddingTab`、`FastembedModelPanel`、Embedding API、本机 runtime settings、RAG 搜索/写入和章节查询调用。
2. Host 合同：Embedding API contract、command manifest、生成 TypeScript、Tauri command registry、Server dispatch/routes 和任务注册。
3. Runtime：配置连接测试、Dynamic/Local/Sidecar provider、模型管理 action、RuntimeProfile 分支、任务 ID 和进度桥接。
4. Capability：`EmbeddingClient`、HTTP 路由映射、Recording/Pending clients、健康检查和事件流。
5. Sidecar：Axum 路由、请求 DTO、模型 handler、下载事件 handler、独立进程测试和认证/secret policy。
6. FastEmbed：provider、模型列表、缓存定位、下载信息、文件元数据、分块传输、聚合进度和删除。
7. 隐式入口：Tauri 宏注册、Web 字符串 dispatch、Sidecar 路由表、Cargo feature 分支、环境变量 `A4N_EMBEDDING_CACHE_DIR`。
8. 横向搜索：Rust、TypeScript、测试和生成合同；未发现该功能相关的 XML、反射调用、动态代理、AOP 切面或其他路由配置。

## 活跃调用拓扑

```text
配置连接测试
  -> Embedding 设置页
  -> local_test_embedding_config (Tauri)
       -> LocalEmbeddingProviderRuntime
       -> canonical EmbeddingProvider::test_connection
  -> test_embedding_config (Server)
       -> Runtime 解析 snapshot 或经管理员门禁读取 DB 配置
       -> Full: concrete provider -> canonical test_connection
       -> Sidecar: /embedding/embed_texts -> 严格校验 identity/count/dimension

模型管理
  -> 设置页 typed API
  -> Tauri 本机命令或 Server command route
  -> RuntimeProfile::uses_sidecars
       -> Host FastEmbed cache owner
       or EmbeddingClient typed operation -> 独立 Sidecar 路由
  -> list / download_info / download events / delete

本机文本嵌入
  -> RAG 搜索、RAG 写入、章节查询
  -> local_embed_texts
  -> exact LocalEmbeddingConfigInput
  -> cached FastEmbed provider
  -> embed_with_identity
```

## 优化记录

- R-1459：完成专项 BFS，覆盖前端、Host、Runtime、Capability、Sidecar、FastEmbed、任务事件与生产 profile。
- R-1460：连接测试文本统一使用 `EMBEDDING_CONNECTIVITY_TEST_TEXT`，删除 Sidecar 任意文本参数和重复常量。
- R-1461：Direct 配置连接测试删除手写 `embed + 数量/维度` 分叉，唯一调用 `EmbeddingProvider::test_connection`。
- R-1462：删除 `SidecarEmbeddingProvider::test_connection` 宽松覆盖，恢复 trait 默认的数量与维度严格校验。
- R-1463：连接测试成功合同统一为 unit；失败只能通过显式错误表达，不保留布尔成功协议。
- R-1464：Capability 模型操作由运行时字符串改为穷举 `EmbeddingModelOperation`，删除未知操作不可达错误分支。
- R-1465：模型请求 payload 删除 `operation` 字段，路由成为操作类型的唯一事实来源。
- R-1466：Sidecar handler 删除与 Axum 路由重复的 `validate_operation` 二次分派。
- R-1467：模型列表使用 unit payload；下载信息、删除和下载事件使用模型名必填 DTO，不再共享可选模型占位结构。
- R-1468：模型删除统一返回 unit，删除 Runtime `DeleteEmbeddingModelOutput.deleted` 伪确认 DTO 和布尔分支。
- R-1469：本地模型列表删除 Host 缓存绝对路径字段，FastEmbed 内部状态不再跨 Capability 泄漏机器路径。
- R-1470：本地模型列表与下载信息 summary 启用未知字段拒绝，旧 `cache_path`、`files` 不能被静默忽略。
- R-1471：清理 Runtime 与独立进程测试中的旧 `cache_path/files` 返回形状，测试改用内部可确定缓存位置。
- R-1472：下载事件消费端删除旧 `percent` 字段；进度唯一由 `downloaded_bytes/total_bytes` 表达。
- R-1473：FastEmbed 下载进度删除 `min(total)` 静默钳制，累计字节超过声明总量时立即失败。
- R-1474：下载信息与实际下载统一复用 checked byte totals，删除一套手写求和并补齐溢出检查。
- R-1475：实际下载拒绝总大小为零，并将“已完整缓存”条件收紧为严格相等。
- R-1476：本机 `batch_size` 按必填正整数处理，删除 `unwrap_or(32).max(1)` 默认与修正兜底。
- R-1477：`LocalEmbeddingConfigInput` 改为严格 DTO；前端只发送五个所需字段，不再把完整配置和无关密钥字段交给 Tauri。
- R-1478：本机嵌入统一使用 provider 的 `embed_with_identity`，删除配置拼装身份和空输入提前成功分叉。
- R-1479：Host 与 Dynamic FastEmbed 缓存路径统一复用一个 resolver；`Some(空白)` 明确失败，仅 `None` 使用配置默认目录。
- R-1480：模型管理直接使用 `RuntimeProfile::uses_sidecars`，删除只包装布尔判断的 `EmbeddingModelExecution` 枚举与适配函数。
- R-1481：Host 与 Tauri 下载完成后统一复用 `downloaded_model_summary`，删除第二套模型定位代码。
- R-1482：删除只承载两个常量的下载任务 scope struct/function，直接导出任务总量和初始消息常量。
- R-1483：Web 与 Tauri 下载任务 ID 统一由 Runtime 校验；空白、超长和非法字符均失败，不再把空白 ID 降级为无任务 ID。
- R-1484：Host 的 `DownloadEmbeddingModelInput` 在任务边界显式拆分，Runtime 下载 owner 只接收实际需要的 `EmbeddingModelInput`。
- R-1485：Capability mocks、Runtime tests 和 Sidecar 独立进程测试同步到唯一协议，不保留旧请求/响应兼容样例。
- R-1486：删除连接测试收敛后唯一调用已归零的 `positive_i32_as_usize`，并将本机 FastEmbed 解析函数限定到实际 feature。
- R-1487：按 `local-embedding` feature 限定 FastEmbed 专用导入与 re-export，默认 Sidecar 构建不再携带无效本机实现符号。
- R-1488：独立进程测试由公开 `model_code` 推导测试缓存目录，不恢复已删除的 `cache_path` wire 字段。
- R-1489：unit Sidecar 响应改为只校验成功状态且拒绝非空 payload，解决 JSON `null` 被 `Option` 折叠问题且不恢复删除布尔 DTO。
- R-1490：补充 unit 响应合同检查，明确拒绝旧 `{deleted: true}` payload。
- R-1491：使用仓库现有 `/home/fufu/Code/A4N/target` 重生成并校验 Host/TypeScript 合同，没有创建第二套 Cargo target。

## 当前事实标准

Embedding 连接测试必须真实调用 provider 并严格验证单向量及配置维度；模型操作类型只由 Capability 枚举和 Sidecar 路由表达，payload 不再重复声明操作；删除返回 unit；模型列表不暴露缓存路径；下载进度只传字节并禁止钳制；本机配置只接受严格 DTO，`batch_size`、显式缓存路径和任务 ID 均不做静默修正。

## 验证结果

- `cargo fmt --all`、目标前端 Prettier 和 `git diff --check`：通过。
- `a4n-api-contract`：22 项通过；生成合同 `--check`：通过。
- `a4n-capabilities`：25 项通过。
- `a4n-embedding`：6 项通过。
- `a4n-embedding-fastembed`：10 项通过。
- `a4n-test-support`：35 项通过。
- `a4n-embedding-service`：13 项单元测试通过；独立进程模型列表、缓存清理、删除和恢复 smoke：1 项通过。
- `cargo check -p a4n-runtime --lib`：通过；启用 `local-embedding` feature 后同样通过。
- Runtime 过滤测试在 60 秒内仍停留于编译 `a4n-orchestrator`，未产生本批代码错误；实际 unit Sidecar 删除链已由独立进程 smoke 覆盖。
- Server/Tauri 联合检查已进入 Server，受既有 UsageGuide、Outline 和 Video Prompt 缺失导出/参数漂移的 11 个错误阻断；未出现 Embedding 专项错误。
- 最终 TypeScript 检查受既有 Outline 生成合同漂移的 9 个错误阻断；未出现 Embedding API、严格本机 DTO 或模型管理类型错误。
- 退役符号搜索：旧模型操作字符串 payload、`EmbeddingModelExecution`、删除布尔 DTO、`cache_path/files/percent` wire 字段、`batch_size` 默认兜底和旧连接测试 helper 的生产引用均为 0。
