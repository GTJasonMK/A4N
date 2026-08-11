# AI 配置连接测试与模型发现 BFS 冗余清理记录

日期：2026-08-11
编号范围：R-1209～R-1235

## 搜索起点与覆盖范围

以“用户在设置页测试 LLM、Embedding、图片、视频配置并发现可用模型”为功能起点，按广度优先顺序检查：

1. 前端入口：LLM、Embedding、图片、视频设置页，测试按钮、请求预览、模型发现按钮、本机/服务器配置分支和当前用户身份注入。
2. 前端 API：本机 runtime snapshot、系统默认配置、个人 API Key、本机配置 API Key、模型发现输入及缓存键构造。
3. Host 合同：command manifest、Rust 到 TypeScript 生成结果、Tauri command/registry、Web dispatch/routes、认证 context 和 secret pointer。
4. Runtime：四类配置 connectivity action、runtime snapshot 解析、数据库配置读取、管理员门禁、任务队列和 RuntimeProfile 分支。
5. Provider 调用：LLM gateway、Embedding provider、图片/视频生成 provider、Sidecar client、请求预览与模型发现调用。
6. Capability 协议：`ImageClient`、`VideoClient`、HTTP client、Recording client、请求/响应 DTO 和 secret policy。
7. Sidecar 服务：Image/Video Axum 路由、handler、provider factory、模型发现 handler、独立进程测试和认证边界。
8. 模型发现：三类 Runtime action、`a4n-model-discovery`、provider catalog、保存密钥读取和显式密钥传输语义。
9. 生产场景：默认 Tauri/Server 启动 profile、无 embedded capabilities 的 Sidecar 路径，以及生产数据库中实际启用的远程 provider 配置。
10. 横向检查：Rust、TypeScript、生成合同、测试、脚本和路由表；未发现相关 XML、反射调用、动态代理、AOP 切面或额外配置入口。

## 活跃调用拓扑

```text
LLM / Embedding 配置测试
  -> settings UI
  -> typed frontend API + userId + optional runtime snapshot
  -> strict Host contract
  -> Tauri user-aware command or authenticated Web dispatch
  -> Runtime resolves matching local snapshot
       or requires ManageSettings before reading global DB config
  -> tracked queue
  -> real provider request
  -> empty response / wrong vector count / wrong dimension => explicit failure

Image 配置测试
  -> the same identity and config resolution path
  -> the existing image generation queue
  -> the existing real generate_image chain
  -> provider returns real image bytes => success
  -> no independent probe protocol

Video 配置
  -> no connectivity-test command or UI button
  -> generation, preview and model discovery remain independent active capabilities

LLM / Image / Video 模型发现
  -> three editors reuse one discovery input/key builder
  -> typed frontend API + userId
  -> strict Host contract
  -> one Runtime discovery implementation
       -> explicit api_key, or
       -> ManageSettings + valid saved config ID + nonblank saved key
       -> the two secret sources are mutually exclusive
  -> provider-specific Sidecar/embedded discovery adapter
  -> a4n-model-discovery
```

## 生产数据与执行路径证据

- 默认无 embedded capabilities 的 Tauri 与 Server 构建使用 `RuntimeProfile::Sidecar`，因此 Sidecar probe 不是边缘测试路径，而是默认生产路径。
- 旧 Image `/image/probe` 只创建 provider 并生成 request preview；旧 Video `/video/probe` 只创建 provider，均未访问上游服务。
- 旧 LLM 与 Embedding 测试会访问 provider，但 Runtime 返回的布尔值被 Host unit 合同丢弃，空响应或向量合同错误仍可能显示成功。
- 生产数据库只读审计：LLM 5 条，其中 DeepSeek 2 条、OpenAI 3 条；Embedding 2 条 FastEmbed；Image 11 条远程配置；Video 4 条远程配置。
- 生产 Image、Video 配置均使用远程 provider 且均有实际 API Key，假 probe 覆盖真实生产配置，不能作为无害开发辅助保留。

## 优化记录

- R-1209：以四类 AI 配置测试与模型发现为入口完成专项 BFS，覆盖前端、Host、Runtime、Orchestrator、Capability、Sidecar、provider catalog 和生产数据。
- R-1210：生产库只读审计确认 Image 11 条、Video 4 条均为远程 provider，假 probe 确实覆盖活跃生产场景。
- R-1211：确认默认 Tauri/Server 使用 Sidecar profile，删除“Full profile 真实、Sidecar probe 足够”的错误分叉假设。
- R-1212：LLM 连接测试统一返回 unit；Sidecar `false` 和 Direct 空白响应均改为显式错误，不再由 Host 丢弃失败布尔值。
- R-1213：Embedding Sidecar connectivity 删除布尔匹配结果，改为逐项验证身份维度、向量数量和向量维度并返回显式错误。
- R-1214：Embedding Direct connectivity 同样严格要求恰好一个向量且维度匹配，删除宽松成功语义。
- R-1215：Image 连接测试删除 Full/Sidecar 两套 probe 分支，唯一复用现有真实 `generate_image` 队列与 provider 链。
- R-1216：删除 Video 设置页连接测试按钮，避免向用户暴露不能证明连接性的操作。
- R-1217：删除前端 `testVideoConfig`、`testServerVideoConfig`、runtime snapshot 构造及只为假测试存在的系统默认视频 helper。
- R-1218：删除 `test_video_config` command manifest、生成 Host 命令、Tauri command/registry、Web dispatch/route 和 Runtime connectivity action，不保留别名。
- R-1219：删除 Video `probe_video_config`、`probe_video_via_sidecar`、`VideoClient::probe`、HTTP `/video/probe`、Sidecar handler 和 `ProviderProbeOutput`。
- R-1220：删除 Video Recording client probe 状态、相关单元测试和独立进程 probe 断言，测试不再维持退役协议。
- R-1221：删除 Image `probe_image_config`、`ImageClient::probe`、HTTP `/image/probe`、Sidecar handler、`ImageProviderProbeOutput` 和 `ProviderProbeOutput`。
- R-1222：删除 Image Recording client probe 状态、Runtime probe 测试和独立进程 probe 断言，只保留 preview 与真实 generation 测试。
- R-1223：LLM、Embedding、Image 测试与预览统一由 Runtime 解析配置；本机 snapshot 允许当前用户使用，回落读取全局 DB 配置必须 `ManageSettings`。
- R-1224：Web 删除三套“是否存在 runtime snapshot”条件权限分叉，只传认证用户 ID，由 Runtime 成为权限唯一 owner。
- R-1225：Tauri 新增并复用 user-aware ID command 宏；前端、manifest 和生成 Host 统一携带 `userId`。
- R-1226：三类测试与预览参数合同改为严格 schema，要求 `userId`、`configId` 并拒绝未知字段。
- R-1227：将通用请求预览结果函数从误导性的 `result_preview_galgame_pipeline_request` 更名为 `result_model_request_preview`。
- R-1228：删除 LLM、Image、Video 三个结构重复的 Runtime discovery 文件，收敛为一个 `detect_config_models` 实现和三个薄入口。
- R-1229：模型发现显式 `api_key` 与 `use_saved_api_key=true` 改为互斥，拒绝双重密钥来源。
- R-1230：保存密钥读取移动到 Runtime 管理员门禁之后；配置 ID 无效或保存密钥缺失时明确失败，不再降级为无密钥请求。
- R-1231：`ModelDiscoveryInput.api_key_encrypted` 更名为真实传输语义 `api_key`，不保留旧字段兼容。
- R-1232：三个配置编辑器删除重复的 discovery input 与 cache key 组装，复用唯一 `buildModelDiscoveryField`。
- R-1233：使用仓库现有 `/home/fufu/Code/A4N/target/` 重生成 TypeScript 类型与 Host 合同，没有建立第二套源码或 Cargo target。
- R-1234：退役的 Video test command、Image/Video probe route、probe DTO、probe client、旧 preview 结果名生产引用清零。
- R-1235：累计超过 10 项后集中完成合同、Capability、Sidecar、前端测试、生产构建、Host parity、格式和残留检查；Runtime 全库测试被既有 Orchestrator 编译错误阻断并显式记录。

## 验证结果

- Rust 到 TypeScript/Host 生成器：通过，使用仓库 `/home/fufu/Code/A4N/target/`。
- API contract：22 个 library 测试和 3 个生成器测试通过。
- `a4n-capabilities`：26 个测试通过。
- `a4n-test-support`：35 个测试通过。
- Image Sidecar：11 个单元测试和 1 个独立进程测试通过。
- Video Sidecar：16 个单元测试和 1 个独立进程测试通过。
- 前端 Vitest：44 个测试文件、162 个测试通过。
- 前端生产构建：通过。
- Web API parity、generation contract 和 fixtures：通过。
- 目标 Rust 文件 `rustfmt`、目标前端文件 Prettier：通过；生成文件最终保持生成器原始格式。
- `git diff --check`：通过。
- 退役符号搜索：Video test command、Image/Video probe route/DTO/client、旧 preview 结果名生产引用均为 0。
- `cargo test -p a4n-runtime --lib` 在编译必需依赖 `a4n-orchestrator` 时失败，首批错误包括 `reject_deprecated_output_fields`、`DependencyGraph`、`shooting_script` 等既有并行改动缺失，以及 Galgame/Prompt/Project Status 类型不一致；编译尚未进入本批 Runtime crate。
- 因同一强制依赖阻断，未重复执行必然得到同组错误的 Tauri/Server Host check，也未添加兼容重导出或 fallback 绕过。

## 当前事实标准

LLM 与 Embedding 连接测试必须真实访问 provider 并满足响应合同；Image 连接测试只复用真实生成链；Video 不提供无法证明连接性的假测试。模型发现只保留一个 Runtime 实现，显式密钥与管理员读取的保存密钥互斥；Host 只传认证身份和严格参数，Runtime 独占权限与密钥来源决策，Sidecar 只负责真实 provider 能力。
