# 项目基础操作 REST 路径分叉清理记录

日期：2026-08-12
编号范围：R-1797～R-1816

## BFS 搜索覆盖范围

从“项目列表、创建、派生、自由视频项目、读取和源快照章节列表”功能入口逐层广度优先展开：

1. 前端入口：项目列表/详情、新建项目、派生项目、自由视频项目和源快照章节选择。
2. 前端 API：所有活跃入口调用 `list_projects`、`create_project`、`create_derived_project`、`create_freeform_video_project`、`get_project`、`list_source_snapshot_chapters` Host commands。
3. CLI：project 子命令与通用 command routing；没有 `/projects*` 专用请求分支。
4. Tauri：统一 Host dispatcher；没有项目专用原生命令。
5. Web：六个 command handlers 与并存的六条 `/projects*` Axum REST routes/handlers。
6. 应用链路：Runtime project records → Orchestrator 项目权限/家族投影 → DB 项目、派生关系与源快照记录。
7. 隐式依赖：`routes/projects.rs` 中唯一被 Host command 借用的 family summary helper、URL 校验测试示例、路由字符串、脚本和文档。

全仓未发现前端、CLI、测试或脚本调用六条 REST API。唯一跨模块引用只是 command 模块借用了 REST 文件中的 summary helper，已可直接调用现有 Runtime action。搜索未发现 XML、反射、动态代理、AOP 或依赖注入容器配置。

## 优化记录

- R-1797：完成页面、前端 API、CLI、Tauri、Web、Runtime、Orchestrator、DB 与隐式引用的全链 BFS。
- R-1798：删除 `GET /projects` 路由。
- R-1799：删除 `POST /projects` 路由。
- R-1800：删除 `POST /projects/derived` 路由。
- R-1801：删除 `POST /projects/freeform-video` 路由。
- R-1802：删除 `GET /projects/{project_id}/source-snapshot-chapters` 路由。
- R-1803：删除 `GET /projects/{project_id}` 路由。
- R-1804：删除重复的 REST 项目列表 handler。
- R-1805：删除重复的 REST 项目创建 handler。
- R-1806：删除重复的 REST 派生项目创建 handler。
- R-1807：删除重复的 REST 自由视频项目创建 handler。
- R-1808：删除重复的 REST 源快照章节列表 handler。
- R-1809：删除重复的 REST 项目读取 handler。
- R-1810：删除只服务 REST query 的 `ListProjectsQuery` 与分页默认/校验 helper。
- R-1811：删除只验证退役 REST DTO/query 的模块内测试。
- R-1812：Host command 直接复用现有 Runtime family summary action，不再反向依赖 REST 模块。
- R-1813：删除只剩 REST facade 的 `routes/projects.rs` 文件。
- R-1814：删除 Server route module 对退役 Projects REST 模块的声明。
- R-1815：CLI URL 安全测试示例改用仍存在的 `/meta`，不再暗示 `/projects` REST 能力。
- R-1816：CLI fragment 拒绝测试同步改用 `/meta`。

## 唯一实现结论

- Web/CLI/Tauri 项目基础操作唯一入口为同名 Host commands。
- 项目权限、分页、类型过滤、家族投影和 DB 读取仍由原有 Runtime/Orchestrator/DB owner 承担。
- Server route 层不再维护 snake_case query、REST body 与 Host args 两套 transport 合同。
- 未保留 `/projects*` 重定向、别名、兼容或回退。

## 集中验证

- `cargo fmt --all`：通过。
- CLI URL 安全定向测试：1 项通过。
- `cargo check -p a4n-server`：通过，同时覆盖 API Contract、Orchestrator 与 Runtime。
- `/projects*`、`/api/projects*` 和 `routes::projects` 退役引用搜索为 0。
- `git diff --check`：通过。
