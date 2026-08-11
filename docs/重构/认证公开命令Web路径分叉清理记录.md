# 认证公开命令 Web 路径分叉清理记录

日期：2026-08-12
编号范围：R-1749～R-1769

## BFS 搜索覆盖范围

从“登录、注册、刷新令牌与 OAuth 登录”功能入口逐层广度优先展开：

1. 前端入口：`AuthPage`、AuthStore、账户页手动刷新和 Web 请求 401 自动刷新。
2. 前端 API：`loginUser`、`registerUser`、`refreshToken`、`oauth2AuthorizeUrl`、`oauth2Callback`，以及 Web/Tauri `apiInvoke` 分流。
3. CLI 入口：auth 子命令、`ApiClient` 登录/注册/OAuth/自动刷新、`routing::command_request` 和并发刷新测试。
4. Web 注册：Axum route table、`/auth/*` handlers、`/commands/{cmd}` 公开命令分支、请求体解析、User-Agent 会话记录和错误响应。
5. Tauri 注册：原生 command 集合、command registry 与登录前无用户上下文的调用约束。
6. 应用链路：Runtime 登录/注册/刷新/OAuth → Orchestrator 认证规则与会话 → DB 用户、邀请码、OAuth 绑定和 refresh token 轮换。
7. 合同与安全：command manifest 的 `PUBLIC_IDENTITY`、密码/OAuth code/refresh token secret pointers、严格 DTO、Bearer 自动刷新和禁用用户检查。
8. 隐式生产入口：Server 进程恢复测试、浏览器 E2E 登录配置、动态 command 字符串和生成 Host 清单。

搜索未发现 XML、反射、动态代理、AOP 或依赖注入容器配置。业务实现原本已经唯一，分叉集中在 Web 同时维护 `/auth/login|register|refresh` 与 `/commands/*` 两套路由；Tauri 原生命令因登录前无法提供 Host dispatcher 所需用户上下文，是必要的 transport 入口而非第二套业务实现。

## 优化记录

- R-1749：完成前端、CLI、Web、Tauri、Runtime、Orchestrator、DB、合同、安全与测试入口的全链 BFS。
- R-1750：Server 登录改为公开 Host command handler，继续保留 User-Agent 会话记录。
- R-1751：Server 注册改为公开 Host command handler。
- R-1752：Server refresh token 改为公开 Host command handler。
- R-1753：Server 将登录、注册、刷新与两个 OAuth 命令收敛为同一公开 command dispatch。
- R-1754：删除 `/auth/login` 专用路由。
- R-1755：删除 `/auth/register` 专用路由。
- R-1756：删除 `/auth/refresh` 专用路由。
- R-1757：删除三套 REST handler 及其 Axum extractor/JSON 死导入。
- R-1758：复用现有 User-Agent 解析函数供唯一 Host handler 使用，不复制解析规则。
- R-1759：前端 Web invoke 将五个公开认证命令合并到统一 `/commands/{command}` 分支。
- R-1760：Web 401 自动刷新迁移到 `refresh_token` command，并使用唯一 `{ input }` 请求合同。
- R-1761：CLI 是否携带认证改由 command manifest 的 `auth` 元数据决定。
- R-1762：删除 CLI 五命令硬编码 special route 分叉。
- R-1763：删除 CLI 随之无调用的 `post` 请求构造 helper。
- R-1764：删除 CLI 缺失 `input` 时伪造空对象的 fallback helper。
- R-1765：generation CLI 输入改为显式要求 object，缺失或类型错误立即失败。
- R-1766：补充 CLI 五个公开命令统一路由与无认证请求断言。
- R-1767：更新 CLI refresh 轮换与并发刷新测试到唯一 command 路径。
- R-1768：更新 Server 进程级登录调用到唯一 command 路径和严格输入包络。
- R-1769：更新浏览器 E2E 登录初始化到唯一 command 路径和严格输入包络。

## 唯一实现结论

- Web/CLI 公开认证入口：`POST /commands/{login_user|register_user|refresh_token|oauth2_authorize_url|oauth2_callback}`。
- Tauri：保留登录前可调用的原生命令，全部直接复用同一 Runtime 认证实现。
- 业务 owner：Runtime → Orchestrator → DB；路由层只解析 transport 输入与 User-Agent。
- 公开身份事实来源：command manifest `PUBLIC_IDENTITY`；CLI 不再维护第二份公开命令名单。
- `/auth/login`、`/auth/register`、`/auth/refresh` 及空输入 fallback 已删除。

## 集中验证

- `cargo fmt --all`：通过。
- 现有 `target/debug/generate_types --check`：通过。
- `cargo test -p a4n-cli`：单元测试 39 项、CLI E2E 5 项全部通过，覆盖公开命令路由与两条 refresh 轮换路径。
- `cargo check -p a4n-server`：通过，同时覆盖 API Contract、Orchestrator 与 Runtime。
- 前端 `auth.ts`/`runtime.ts` 定向测试：2 个文件、10 项通过。
- `npx tsc --noEmit`：通过。
- 旧 `/auth/login|register|refresh` 残留搜索与 `git diff --check`：通过。
- 使用独立临时 PostgreSQL 数据库执行 Server 进程恢复测试：1 项通过；真实启动后通过 `/commands/login_user` 登录并继续完成后续上传/清理流程。临时数据库已删除。
