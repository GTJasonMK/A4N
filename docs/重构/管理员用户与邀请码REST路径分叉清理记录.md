# 管理员用户与邀请码 REST 路径分叉清理记录

日期：2026-08-12
编号范围：R-1782～R-1796

## BFS 搜索覆盖范围

从“管理员查看/创建用户、查看/创建邀请码、修改角色、停用用户”功能入口逐层广度优先展开：

1. 页面入口：设置页管理员用户区的列表加载、创建、角色修改、停用和邀请码操作。
2. 前端 API：六个 typed API 全部调用同名 Host commands，没有访问 `/admin/*` URL。
3. CLI：command catalog、认证 context 注入与通用 `/commands/{command}` 请求；没有 Admin REST 特例。
4. Tauri：统一 Host dispatcher 注入当前用户身份；没有六个专用原生命令。
5. Web：command manifest 的 `ADMIN_IDENTITY`、Server command dispatcher/handlers，以及并存但无消费者的六条 `/admin/*` Axum routes/handlers。
6. 应用链路：Runtime `admin_records` → Orchestrator 管理员权限与用户规则 → DB 用户、角色、停用、邀请码记录。
7. 隐式入口：路由字符串、脚本、测试、文档、动态 command 名与生成 Host 清单。

全仓只找到 `/admin/*` 的六条 route table 注册，没有前端、CLI、脚本、测试或文档消费者。搜索未发现 XML、反射、动态代理、AOP 或依赖注入容器配置。两套路由最终调用同一 Runtime，专用 REST 模块只是重复 transport facade。

## 优化记录

- R-1782：完成页面、前端 API、CLI、Tauri、Web、Runtime、Orchestrator、DB 和隐式入口的全链 BFS。
- R-1783：删除 `GET /admin/users` 路由。
- R-1784：删除 `POST /admin/users` 路由。
- R-1785：删除 `GET /admin/invite-codes` 路由。
- R-1786：删除 `POST /admin/invite-codes` 路由。
- R-1787：删除 `POST /admin/users/{user_id}/role` 路由。
- R-1788：删除 `POST /admin/users/{user_id}/disable` 路由。
- R-1789：删除重复的 REST 用户列表 handler。
- R-1790：删除重复的 REST 用户创建 handler。
- R-1791：删除重复的 REST 邀请码列表 handler。
- R-1792：删除重复的 REST 邀请码创建 handler。
- R-1793：删除重复的 REST 角色修改 handler。
- R-1794：删除重复的 REST 用户停用 handler。
- R-1795：删除只承载上述六个 facade 的 `routes/admin.rs` 模块文件。
- R-1796：删除 Server route module 对退役 Admin REST 模块的声明。

## 唯一实现结论

- Web/CLI/Tauri 唯一入口：六个同名 Host commands。
- 身份与权限：认证 transport 提供 caller，Runtime/Orchestrator 执行真实管理员门禁。
- 业务实现与 DB 规则未复制、未迁移，删除仅发生在无消费者 transport facade。
- 未保留 `/admin/*` 别名、重定向、兼容或回退。

## 集中验证

- `cargo fmt --all`：通过。
- `cargo check -p a4n-server`：通过，同时覆盖 API Contract、Orchestrator 与 Runtime。
- 六个管理员 Host command 的生产引用保持完整，专用 `/admin/users*` 与 `/admin/invite-codes*` 路由字面量残留为 0。
- `git diff --check`：通过。
