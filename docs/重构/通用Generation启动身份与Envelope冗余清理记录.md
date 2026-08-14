# 通用 Generation 启动身份与 Envelope 冗余清理记录

## 搜索覆盖范围

本专项不从单一模块名出发，而是从用户可见的“启动一次正式生成”能力出发，按 BFS 横向覆盖全部 11 条生产命令：

1. `start_blueprint_generation`
2. `start_blueprint_refinement`
3. `start_chapter_generation`
4. `start_image_generation`
5. `start_inspiration_generation`
6. `start_optimize_analysis_generation`
7. `start_optimize_chat_generation`
8. `start_optimize_patch_generation`
9. `start_outline_generation`
10. `start_portrait_generation`
11. `start_video_generation`

BFS 拓扑如下：

```text
页面 / hook / 业务 API wrapper
→ generationClient.start 生成 start_request_id / retry_of_task_id
→ apiInvoke
→ withRuntimeConfigSnapshot(RuntimeConfigPolicy::Full)
→ Host generation envelope { input, runtime_config_snapshot? }
├─ Web
│  → GENERATION_START_PATHS
│  → generationRequestBody 校验 envelope 后展平 input
│  → Bearer token
│  → Axum /generation-tasks/* 固定路由
│  → split_generation_start_body
│  → Start*Input serde decode
├─ Tauri
│  → runtimeInvoke
│  → invoke_host_command 外层 userId
│  → dispatch_generation_start
│  → split_generation_start_command_args
│  → Start*Input serde decode
└─ CLI
   → command manifest / NO_CONTEXT
   → command_request / generation_input
   → /generation-tasks/* 固定路由

→ a4n-runtime::host_api 唯一宿主门面
→ 11 条 start_*_typed action
→ runtime_config::request_scope(user_id, snapshot)
→ Orchestrator 权限与业务输入校验
→ reserve_generation_task / initialize_reserved_task
→ generation worker（LLM / image / video）
→ checkpoint / terminal / result_ref / SSE 或 Tauri subscription
```

显式引用覆盖：

- 11 个 API contract `Start*Input`，均使用 `serde(deny_unknown_fields)`。
- `command_specs!` 动态 manifest 表及其 schema helper。
- `GENERATION_START_ROUTES`、Host TypeScript 生成器和 generation schema 导出器。
- 前端 11 条正式生成 API、`generationClient.start`、`apiInvoke`、runtime snapshot 注入和 Web body 转换。
- Axum 11 条固定路由、共享 `start_generation` typed 分发、Tauri `invoke_host_command` 和 CLI generation 路由转换。
- Runtime `host_api` 导出、11 条 typed action、Orchestrator 校验/预约、DB generation reservation 和对应 worker。

隐式依赖覆盖：

- `RuntimeConfigPolicy::Full` 生成的 runtime snapshot 注入及 task-local request scope。
- Tauri `tauri::generate_handler!` 注册、Axum Router 路由表、command manifest 宏展开和生成 TypeScript 常量。
- Web JWT、Tauri 外层认证用户、CLI session token 三种生产身份来源。
- client request id、retry task id、业务 fingerprint、任务去重、恢复和终态持久化。
- LLM、图片和视频三类 worker 分支，以及 outline/portrait/image/video operation 枚举分支。
- Runtime 内部 `generate_portrait_inline` 直接构造正式 typed input 的旁路；该路径不经过 Host envelope，仍只使用显式 `user_id` 与同一 Orchestrator 校验/预约实现。

仓库配置扫描未发现这 11 条启动命令存在 XML DI、反射字符串调用、动态代理或 AOP 切面旁路。动态入口只有 command manifest、generation route 表、Tauri handler、Runtime host facade 和 worker spawn，均已纳入闭包。

## 清理前分叉

```text
Host envelope schema
├─ image / inspiration
│  → strict runtime typed input
├─ outline / portrait
│  → strict typed input，但未声明实际可注入的 runtime snapshot
├─ blueprint generation / refinement / chapter / video
│  → typed input，允许任意额外顶层字段
└─ optimize analysis / chat / patch
   → args_opaque，完全不公开真实输入合同

身份
→ Web Bearer token / Tauri 外层 user_id / CLI session token
→ 7 条前端启动调用再次发送业务 userId

未知 envelope 字段
├─ Web generationRequestBody 静默丢弃
├─ CLI generation_input 静默丢弃
└─ Tauri dispatch_generation_start 只读取 input/snapshot 后静默忽略
```

这些分叉不会改变 Orchestrator 的真实身份来源：权限、任务 owner、项目范围和 DB reservation 始终使用宿主认证出的 `user_id`。业务 `userId` 只是无效第二身份源；宽松或 opaque schema 又使其长期不报错。

## 优化记录

1. `start_blueprint_generation` 改用既有 `strict_runtime_typed_input_schema`。
2. `start_blueprint_refinement` 改用既有 `strict_runtime_typed_input_schema`。
3. `start_chapter_generation` 改用既有 `strict_runtime_typed_input_schema`。
4. `start_optimize_analysis_generation` 退役 `args_opaque`，公开真实 strict runtime typed input。
5. `start_optimize_chat_generation` 退役 `args_opaque`，公开真实 strict runtime typed input。
6. `start_optimize_patch_generation` 退役 `args_opaque`，公开真实 strict runtime typed input。
7. `start_outline_generation` 从 strict input 收敛为实际生产所需的 strict runtime input。
8. `start_portrait_generation` 从 strict input 收敛为实际生产所需的 strict runtime input。
9. `start_video_generation` 从宽松 typed input 收敛为 strict runtime input。
10. 保留已经正确的 image、inspiration 两条 manifest，不新建第二套 generation schema helper。
11. 新增一项 manifest 回归，一次验证 11 条命令只允许 `input` 和 `runtime_config_snapshot`、必须 typed、必须 Full runtime policy、不得声明业务 `userId`。
12. 蓝图正式生成删除重复 `userId`。
13. 蓝图 refinement 删除重复 `userId`。
14. 章节正文生成删除重复 `userId`。
15. 视频正式生成删除重复 `userId`。
16. Optimize analysis 正式生成删除重复 `userId`。
17. Optimize chat 正式生成删除重复 `userId`。
18. Optimize patch 正式生成删除重复 `userId`。
19. Tauri 共享 `dispatch_generation_start` 新增唯一 command envelope 拆分点，拒绝 `input`/`runtime_config_snapshot` 之外的字段。
20. 删除 Tauri dispatcher 对额外顶层字段的静默兼容行为；未知字段统一返回 `generation_request_invalid`。
21. CLI `generation_input` 在展平 HTTP body 前拒绝未知 envelope 字段，不再静默丢弃。
22. CLI canonical route 回归遍历全部 11 条命令，验证退役身份字段均被拒绝。
23. Web `generationRequestBody` 在展平 input 前只允许 `input` 与 `runtime_config_snapshot`，拒绝所有未知 envelope 字段。
24. Web 不再静默丢弃额外字段，也不会让与合法 input 同名的 envelope 字段覆盖正式业务输入。
25. 新增 Server command envelope 回归，验证 Tauri 业务 `userId` 被拒绝。
26. 新增 Server Web body 回归，验证直接 HTTP 调用中的未知字段由正式 `Start*Input` 合同拒绝。
27. 新增前端 Web generation envelope 回归，验证未知字段在 body 展平前显式失败。

## 清理后唯一事实标准

```text
认证身份
├─ Web：Bearer token 解析出的 user_id
├─ Tauri：invoke_host_command 外层 userId → Rust user_id
└─ CLI：session token 对应的服务端认证用户

Host generation envelope
→ { input: Start*Input, runtime_config_snapshot? }
→ additionalProperties = false
→ context = NO_CONTEXT
→ runtime policy = Full

Web 固定路由 body
→ { ...Start*Input, runtime_config_snapshot? }
→ 仅移除 runtime_config_snapshot
→ 其余字段全部交给 Start*Input deny_unknown_fields

业务执行
→ Runtime host_api typed action
→ Orchestrator 使用认证 user_id 校验权限并预约任务
→ DB requested_by_user_id 只来自认证身份
```

未新增默认用户、旧字段别名、兼容转换、未知字段忽略、异常回退、备用 schema、专用 adapter 或占位实现。

## 集中验证

- `cargo test -p a4n-api-contract`：60 项库合同测试、5 项 TypeScript 生成器测试全部通过；新增 11 命令统一 envelope 回归通过。
- `cargo test -p a4n-server --lib routes::generation::tests`：5 项通过，覆盖合法 runtime snapshot、Tauri 未知 envelope 字段拒绝和 Web 转发字段 typed 拒绝。
- `cargo test -p a4n-cli routing::tests`：6 项通过，canonical generation route 遍历全部 11 条命令并拒绝退役身份字段。
- `cargo check -p a4n-runtime -p a4n-server -p a4n-tauri -p a4n-cli`：通过，覆盖 Runtime、Web、桌面和 CLI 宿主。
- `cargo test -p a4n-server --test cleanup_recovery server_process_streams_optimize_chat_generation_failure_terminal -- --test-threads=1`：按 `docs/TEST_DATABASE.md` 加载仓库 `.env`，1 项通过。
- `cargo test -p a4n-tauri --features embedded-media-providers tauri_channel_remounts_active_image_generation_after_window_cleanup -- --test-threads=1`：按隔离测试库串行执行，1 项通过，覆盖真实 Tauri dispatcher、runtime snapshot 和 generation worker。
- `npm run test:run`：57 个测试文件、194 项前端测试通过。
- `npx tsc --noEmit`：通过。
- `npm run check:web-api-parity`：通过；Host TypeScript、generation schema、TypeScript 输出和 fixtures 均无漂移。
- 目标 Rust 文件 `rustfmt --check`：通过。
- `git diff --check`：通过。
- 退役符号扫描：7 条前端启动调用均不再发送业务 `userId`；Optimize 三条启动 manifest 无 `args_opaque` 残留；11 条 manifest 全部指向 strict runtime typed schema。
