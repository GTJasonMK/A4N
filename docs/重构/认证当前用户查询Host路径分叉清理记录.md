# 认证当前用户查询 Host 路径分叉清理记录

日期：2026-08-12
编号范围：R-1719～R-1734

## BFS 搜索覆盖范围

从“读取当前认证用户资料”功能入口逐层广度优先展开：

1. 前端入口：Zustand AuthStore 初始化/刷新、`api/auth.ts::getCurrentUser`、`apiInvoke` Web/Tauri 路由和本地认证状态。
2. CLI 入口：`ApiClient::current_user`、认证状态命令，以及 workflow/context 等命令绑定当前用户的所有调用点。
3. Host/REST 注册：command manifest、生成 Host 清单、Tauri command registry、Server `/auth/me` 和 `/commands/{cmd}` dispatcher。
4. 应用链路：Runtime `get_current_user`、Orchestrator `current_user`、DB `current_user/active_user`。
5. 隐式依赖：Axum route table、CLI routing special request、Tauri command 宏、动态命令字符串、Bearer 认证、Serde/Schemars 和 Host TypeScript 生成。
6. 生产失败路径：禁用用户必须被当前用户查询拒绝；管理员权限检查所需的内部按 ID 原始读取仍由 DB owning layer 保持。

搜索未发现 XML、反射注册、动态代理、AOP 切面或依赖注入容器配置。应用与 DB 层原本已有唯一实现，分叉集中在 Web `/auth/me`、CLI GET 特例和 Tauri 原生命令；manifest 声明的通用 Host command 反而未注册到 Server dispatcher。

## 优化记录

- R-1719：完成前端、CLI、Host/REST、Runtime、Orchestrator、DB 和禁用用户路径的全链 BFS。
- R-1720：`get_current_user` manifest 参数从 opaque 收敛为空对象 Schema。
- R-1721：新增唯一 Server Host handler，目标用户只取认证上下文。
- R-1722：将 `get_current_user` 注册到通用 Server dispatcher，修复 manifest 与可执行路由分叉。
- R-1723：删除 Server route table 中 `/auth/me` 专用端点。
- R-1724：删除 Server `auth::me` REST handler。
- R-1725：删除 Tauri 专用 `get_current_user` 命令。
- R-1726：删除 Tauri command registry 中对应注册。
- R-1727：删除前端 `webInvoke` 的 `/auth/me` 特例。
- R-1728：前端 `runtimeInvoke` 删除 `get_current_user` 原生命令分支。
- R-1729：前端 `getCurrentUser` 删除请求体 `userId`，只调用空参数 Host command。
- R-1730：删除 CLI routing 对 `/auth/me` 的 GET 特例。
- R-1731：删除 CLI routing 随之无调用的 `get` 请求辅助函数。
- R-1732：CLI 工作流绑定当前用户改走 `/commands/get_current_user`，不再维护第二 URL 合同。
- R-1733：补充 manifest、前端 API、Tauri dispatcher 和 CLI E2E 防回归断言。
- R-1734：删除前端单身份收敛后遗留的 `requireCurrentUserId` 死导入。

## 唯一实现结论

- Host command：`get_current_user`，参数为空，身份只来自认证上下文。
- Web/Tauri/CLI 均通过 `/commands/get_current_user` 或同一 Tauri Host dispatcher。
- 应用实现：Runtime `get_current_user` → Orchestrator `current_user` → DB `current_user`。
- `/auth/me`、Tauri 原生命令、CLI GET 特例和请求体 `userId` 均已删除。

## 集中验证

- `cargo fmt --all`：通过。
- 使用仓库现有 `target` 重建生成器并执行 `--write`、`--check`：通过。
- API Contract 全量测试：28 项通过。
- 前端 `auth.ts`/`runtime.ts` 定向测试：2 个文件、10 项通过。
- `npx tsc --noEmit`：通过。
- CLI `task_and_workflow_terminal_states_use_stable_exit_codes` E2E：1 项通过，确认首个请求为 `/commands/get_current_user`。
- `cargo check -p a4n-server`：通过，同时覆盖 API Contract、Orchestrator、Runtime 和 Server。
- `git diff --check`：通过。
