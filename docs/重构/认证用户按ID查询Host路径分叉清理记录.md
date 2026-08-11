# 认证用户按 ID 查询 Host 路径分叉清理记录

日期：2026-08-12
编号范围：R-1735～R-1748

## BFS 搜索覆盖范围

从“账户设置加载当前登录用户资料”功能入口逐层广度优先展开：

1. 页面入口：`AboutTab` 只在已有认证用户时渲染 `ProfileEditor`，传入的 ID 来自同一 AuthStore 当前用户。
2. 前端调用：`ProfileEditor` → `api/auth.ts::getUser` → `apiInvoke`；横向对比已存在的 `getCurrentUser` 唯一 self 查询。
3. Host 合同：command manifest、生成 Host command 清单、Web dispatcher、Tauri 原生命令集合及 command registry。
4. 应用链路：Server/Tauri → Runtime `get_user` → Orchestrator `auth_user_by_id` → DB `user_by_id`；并与 `get_current_user` → `current_user` → DB `active_user` 对比权限语义。
5. CLI 隐式依赖：manifest context 注入测试使用该 command descriptor，但并不执行该业务命令；改由仍声明 `USER_CONTEXT` 的真实命令覆盖同一测试目的。
6. 管理员横向路径：`require_admin_permission` 与 `is_active_admin_user` 仍需要按 ID 读取原始用户后显式检查 active/role，因此保留 Orchestrator/DB 内部查询，不向普通 Host 暴露。
7. 动态入口：检查命令字符串、Tauri 宏注册、生成器输入/输出、前端动态 invoke 和文档残留；未发现 XML、反射、动态代理、AOP 或依赖注入容器配置。

结论：生产前端唯一调用者查询的始终是当前认证用户，按任意 `userId` 暴露的 Host command 是第二套 self 查询合同；内部管理员权限路径不是其消费者。

## 优化记录

- R-1735：完成页面、前端 API、Host、Server、Tauri、Runtime、Orchestrator、DB、CLI 与生成文件的全链 BFS。
- R-1736：账户资料加载改为复用 `getCurrentUser`，目标身份只来自认证上下文。
- R-1737：删除前端 `getUser(userId)` API。
- R-1738：删除前端 Tauri 原生命令集合中的按 ID 用户查询分支。
- R-1739：删除 command manifest 中重复的按 ID 用户查询合同。
- R-1740：使用仓库现有 `target/debug/generate_types` 重建生成清单并删除对应 command。
- R-1741：删除 Server 按 ID 用户查询 handler 及随之无用的参数解析导入。
- R-1742：删除 Server dispatcher 中对应命令分支。
- R-1743：删除 Runtime 同名透传函数。
- R-1744：删除 Tauri 原生按 ID 用户查询命令。
- R-1745：删除 Tauri command registry 中对应注册。
- R-1746：删除 Tauri 随之无用的 `UserSummary` 导入。
- R-1747：CLI context 注入测试改用真实保留且声明 `USER_CONTEXT` 的模型发现命令。
- R-1748：全仓残留搜索确认退役符号归零；保留管理员权限检查真实使用的 Orchestrator/DB 内部按 ID 查询。

## 唯一实现结论

- 当前登录用户资料：`get_current_user` → Runtime `get_current_user` → Orchestrator `current_user` → DB `active_user`。
- 任意用户 ID 不再作为普通认证 Host 查询能力暴露。
- 管理员权限内部读取：`auth_user_by_id` → DB `user_by_id`，仅服务 active/role 判定，不与 self 查询共享错误语义。
- 未增加别名、兼容层、回退或占位逻辑。

## 集中验证

- `cargo fmt --all`：通过。
- 使用仓库默认 `target` 重建 `generate_types`，执行 `--write` 与 `--check`：通过。
- `cargo test -p a4n-api-contract`：库测试 28 项、生成器测试 3 项全部通过。
- CLI context 注入定向测试：1 项通过。
- 前端 `auth.ts`/`runtime.ts` 定向测试：2 个文件、10 项通过。
- `npx tsc --noEmit`：通过。
- `cargo check -p a4n-server -p a4n-tauri`：通过，同时覆盖 Orchestrator 与 Runtime。
- 退役符号残留搜索与 `git diff --check`：通过。
