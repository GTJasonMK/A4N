# Provider 配置管理合同冗余清理记录

## 搜索覆盖范围

本专项从 LLM、图片、视频、Embedding 四类配置管理功能入口出发，按 BFS 逐层追踪：

```text
Settings 四类页面 / Admin ApiConfigSection / ConfigListPanel
→ frontend config API / localRuntimeSettings / runtimeConfigSnapshot
→ Host command manifest / generated TypeScript / CLI context injection
→ Web JWT / Tauri invoke_host_command 会话身份
→ Server dispatch / config command macros / 参数解析
→ Runtime config_records 权限、连接测试、请求预览、模型探测
→ Orchestrator settings application / 默认配置事务 / 更新校验
→ a4n-db provider records / SeaORM entities / provider migrations
→ LLM、图片、视频、Embedding provider registry / factory / 请求体预览
→ sidecar 协议、服务实现、本机执行、异步视频轮询与清理恢复
```

同时检查了以下隐式依赖：

- Host manifest 的 `x-a4n-context-fields` 与 CLI 自动注入。
- Web JWT 和 Tauri `invoke_host_command.user_id` 建立的 Runtime 请求上下文。
- `runtime_config_snapshot` 对本机配置、系统模板和无密钥预览的覆盖路径。
- Provider catalog 对设置页面表单元数据、API Key 要求和模型能力的驱动关系。
- 默认配置、删除门禁、进行中视频任务门禁和 LLM max-token 约束。
- 生成合同到 `frontend/src/api/generated/config.ts` 的代码生成链。
- 图片/视频请求体在 Runtime embedded、sidecar service 和 provider crate 间的分派关系。
- Embedding API/FastEmbed/local_hash 在配置校验、Runtime、sidecar 和本机模型管理间的分派关系。
- 视频后端在配置时长校验、请求构造、轮询、远端删除和 cleanup outbox 恢复间的能力关系。

项目没有 XML DI 容器、运行时反射注册、动态代理或 AOP 配置；生产期动态分派由显式 command dispatch、provider factory 和 sidecar 协议承担，均已纳入上述闭包。

## 优化记录

1. 删除 LLM 配置测试、请求预览和模型探测业务参数中的 `userId`。
2. 删除图片配置测试、请求预览和模型探测业务参数中的 `userId`。
3. 删除视频模型探测业务参数中的 `userId`。
4. 删除 Embedding 配置测试和请求预览业务参数中的 `userId`。
5. 将上述命令 manifest context 统一改为 `NO_CONTEXT`，CLI 不再注入重复身份。
6. 保留 Tauri `invoke_host_command.user_id` 和 Web JWT 身份，Host 会话成为唯一调用者身份来源。
7. 将三类模型探测 manifest schema 收敛为严格 `input` 合同。
8. 将三类连接测试和预览 manifest schema 收敛为 `configId + runtime_config_snapshot` 严格白名单。
9. 将四类配置创建命令收敛为严格 typed-input，拒绝顶层额外字段。
10. 新增唯一的配置更新 schema：`configId + input`，并禁止额外字段。
11. 将 LLM max-token 更新命令收敛为严格 typed-input。
12. 将 Provider catalog 从 opaque 参数改为严格空参数。
13. Server 无参配置命令统一调用 `empty_command_args`，不再静默忽略参数。
14. Server 配置创建、更新、ID 操作、模型探测和连接测试统一拒绝未知参数。
15. 删除 Server 的 `require_global_config_admin` 角色查询和手写角色判断。
16. 将四类配置 create/update/delete/set-default 与 LLM max-token 更新的权限统一交给 Runtime `ManageSettings`。
17. 配置写操作恢复 `AppError` 原始类型，Web 能保留 403/404/409 等真实状态，不再全部降级为 400。
18. 删除图片更新在 Runtime 的第一次读库，更新只在 Orchestrator 读取一次现有记录。
19. 删除视频更新在 Runtime 的第一次读库，更新只在 Orchestrator 读取一次现有记录。
20. 删除图片和视频 mapping 中未使用的 `_existing` 参数。
21. 将 Embedding 更新合同改为完整结构字段，仅 API Key 保留“缺失=保留、null=清空、字符串=替换”。
22. 删除 Embedding Runtime 的旧值合并和重复读库。
23. 删除前端手写 `UpdateEmbeddingInput` 镜像，直接使用生成合同。
24. 删除本机 Embedding 的 `Partial` 更新分叉，改为同一完整替换合同。
25. 将 LLM 更新合同改为完整结构字段，仅 API Key 保留三态语义。
26. 删除 `a4n-llm` 的 `update/resolution.rs` 旧值解析模块，更新复用创建规范化规则。
27. 删除本机 LLM 的 `Partial` 更新分叉，并合并 Server 两套相同 ID 命令宏。
28. 将图片和视频更新合同改为完整结构字段，仅 API Key 保留三态语义。
29. 删除图片、视频 Runtime 更新前的重复读库和旧值合并参数。
30. 删除视频 mapping 的 `params.rs` 中间层，创建/更新 DTO 直接映射到 Orchestrator 合同。
31. 将四类 Provider 配置的创建与更新校验统一下沉到各自 owning layer，Runtime 不再重复校验。
32. 将四类 `has_api_key` 判断合并为 Runtime 共享函数。
33. 删除 DB Provider records/resolution 中四套重复的默认配置清理和更新结果转换。
34. 将 LLM Provider ID、展示名、默认 URL、协议类型收敛到 `a4n-llm` 唯一注册表。
35. Runtime Provider catalog 改为从 LLM 注册表生成，删除手写 LLM catalog。
36. LLM HTTP provider 的协议选择改为读取同一注册表，删除第二套 provider-to-protocol 分派。
37. 将图片 Provider ID、展示名、API Key 要求和参考图能力收敛到 `a4n-core` 唯一注册表。
38. Runtime Provider catalog、配置校验、Orchestrator provider 选择和图片 factory 统一读取图片注册表。
39. 删除图片 sidecar 请求合同中的重复 API Key 必填判断，统一由图片注册表和 factory 执行。
40. 将 Embedding backend ID、展示名、类型、Base URL/API Key 要求收敛到 `a4n-core` 唯一注册表。
41. 删除 Runtime 的 `SupportedEmbeddingBackend` 和本机 Embedding 手写 backend 解析器。
42. Embedding factory、模型身份、本机模型执行和 embedding service 统一读取 Embedding 注册表。
43. 删除 Embedding sidecar 的独立 `normalized_sidecar_backend` 解析器。
44. 合并 Embedding 连接测试与实际执行的负载构造，按注册表统一剔除本地后端无关 URL/API Key。
45. Embedding sidecar 可选 URL/API Key 改为缺失字段而非 `null`，避免制造伪 secret 字段。
46. 将视频 Provider ID、展示名、时长上下限、同步/异步、状态查询和删除能力收敛到 `a4n-video` 唯一注册表。
47. Runtime catalog、Orchestrator 配置校验、Provider factory、异步轮询和 cleanup outbox 统一读取视频注册表。
48. 删除 CLIProxy 专用的重复基础时长校验，保留其“参考图时最多 10 秒”的真实请求约束。
49. 修复 Seedance 配置允许 1-60 秒、请求阶段才拒绝的分叉，配置与请求均严格使用 4-15 秒。
50. 图片 Runtime embedded 请求预览改为复用 `a4n-image-providers` 唯一请求体分派，删除六套重复构造调用。
51. 新增 `a4n-video-providers` 唯一请求体预览入口，Runtime embedded 与 video sidecar service 共用同一分派。
52. 修复 unit sidecar 响应协议分叉：成功必须显式 `payload: null`，缺失或对象型旧负载均拒绝。
53. 同步修正 Embedding 独立进程测试的 unit 响应断言，删除对旧缺失 payload 的接受。

## 明确保留的活跃分支

- 本机运行时配置：桌面/本机 Provider 是真实生产模式，不属于旧兼容逻辑。
- Tauri 原生 Embedding：模型下载、缓存和本机连接测试是独立生产执行路径。
- `runtime_config_snapshot`：用于本机配置执行、系统模板个人密钥和无密钥请求预览。
- Provider catalog：设置页面依赖其展示元数据和能力约束，不能由 factory ID 列表替代。
- API Key 三态更新：缺失保留、`null` 清空、字符串替换，属于当前明确合同而非兼容分支。

## 验证

- `cargo check --workspace --all-targets`：通过，无 warning。
- `cargo check -p a4n-runtime --all-targets --features embedded-media-providers`：通过。
- `cargo fmt --all -- --check`、`git diff --check`：通过。
- Web API / generation contract parity：通过。
- 前端 Vitest：51 个文件、186 项通过；生产构建通过。
- `a4n-api-contract`：49 项通过。
- `a4n-core` config：17 项通过。
- `a4n-llm`：52 项通过；`a4n-llm-http`：31 项通过。
- `a4n-video`：4 项通过；`a4n-video-providers`：71 项通过。
- `a4n-image-providers`：38 项通过。
- `a4n-video-service`：16 项单元测试 + 1 项独立进程测试通过。
- Orchestrator 视频配置：19 项通过。
- `a4n-embedding-service`：13 项单元测试 + 2 项独立进程测试通过。
- Runtime 精确验证：Provider catalog 唯一 ID、图片请求体预览、Embedding 共享负载/secret policy、unit sidecar `payload: null` 均通过。
- 隔离数据库 `provider_config_defaults`：1 项通过，测试使用 `.env` 中 `A4N_TEST_DATABASE_URL`。

Runtime 全包测试二进制约 657MB，首次重新链接超过后端单测 60 秒上限；已先用 `--no-run` 完成链接，再在每个 60 秒窗口内执行上述精确断言，均通过。全工作区 all-targets 编译已覆盖其余目标。
