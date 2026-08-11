# 默认 AI 配置功能 BFS 冗余清理记录

日期：2026-08-11
编号范围：R-1181～R-1208

## 搜索起点与覆盖范围

以“管理员查看并切换 LLM、Embedding、图片、视频默认配置”为功能起点，按广度优先顺序检查：

1. 前端入口：设置页四类配置列表、默认项切换、系统/本机来源切换、本机配置删除和当前登录用户身份注入。
2. Host 合同：`a4n-api-contract` command manifest、Rust 到 TypeScript 生成结果、Tauri command registry/macro、Web dispatch/routes 和 CLI context 注入。
3. Runtime：四类配置 action、管理员权限门禁、Host 返回值及 config summary 投影。
4. Orchestrator：四类模型有效性校验、默认项切换 facade、默认项删除门禁及 settings-all 导入编排。
5. DB：四类 provider records、默认项解析、全量配置导入、SeaORM entity 和事务边界。
6. 数据库约束：迁移注册、PostgreSQL partial unique index、历史重复默认项审计和隔离数据库迁移测试。
7. 隐式依赖：Serde/Schemars 参数合同、生成 Host 表、认证用户上下文、前端本机运行时持久化和 settings-all 默认项切换路径。
8. 横向检查：全仓 Rust、TypeScript、测试、脚本和配置引用；未发现相关 XML、反射调用、动态代理、AOP 切面或额外路由表。

## 活跃调用拓扑

```text
server default switch
  -> settings UI / typed frontend API
  -> callerUserId + configId strict Host contract
  -> Tauri caller-aware command or authenticated Web dispatch
  -> Runtime ManageSettings permission gate
  -> Orchestrator provider-specific model validation
  -> one DB set_default_provider_config transaction
       -> clear previous default except target
       -> set target default
       -> rollback both writes when target does not exist
  -> PostgreSQL one-default partial unique index

settings-all import
  -> strict file validation: at most one declared default per provider kind
  -> import transaction
  -> the same clear_provider_config_defaults primitive
  -> provider row insert/update
  -> the same PostgreSQL unique constraint

local runtime default switch
  -> local config existence check
  -> one in-memory default mutation
  -> persisted local runtime settings

switch to server default
  -> read and verify current server default exists
  -> persist local default_source = system

delete local config
  -> reject deletion of the active local default
  -> remove only the requested non-default row
  -> no implicit first-item promotion or source fallback
```

## 数据与约束证据

- 生产数据库通过只读查询审计：LLM 5 条、Embedding 2 条、Image 11 条、Video 4 条。
- 四张表均恰好存在 1 个默认项，没有需要修复或兼容的重复默认历史数据。
- 清理前四张表均只有主键索引，没有默认项唯一约束或触发器；默认唯一性仅依赖四套应用代码约定。
- 生产库未执行测试写入；迁移和原子切换只在仓库 `.env` 指向的隔离 `a4n_test` 数据库验证。

## 优化记录

- R-1181：以四类 AI 默认配置切换为入口完成专项 BFS，覆盖前端、Host、Runtime、Orchestrator、DB、settings-all、迁移和生产数据。
- R-1182：生产库只读审计确认四表各恰好一个默认项，且不存在唯一索引或触发器保护，无旧数据兼容需求。
- R-1183：图片、视频默认切换命令由 `activate_*` 统一命名为 `set_default_*`，不保留别名或适配器。
- R-1184：四类默认切换命令统一返回 unit/null，删除图片、视频无消费者的配置 summary 返回值。
- R-1185：四类命令统一复用严格 `args_config_id`，要求正整数 `configId` 并拒绝未知参数。
- R-1186：删除四类命令会覆盖管理员 caller context 的 `context(NO_CONTEXT)`，恢复认证身份进入 Host 合同。
- R-1187：command manifest 将误用的 `result_accept_project_invite` 统一更名为真实语义 `result_unit`，删除结果 schema 的错误业务别名。
- R-1188：Runtime 四类默认切换统一执行真实 `ManageSettings` 管理员权限校验，不再只依赖 manifest 元数据声明。
- R-1189：Tauri 新增并复用唯一 caller-aware ID command 宏，将调用者与目标配置 ID 分开传递。
- R-1190：前端四类服务器默认切换显式传入当前用户 ID，删除 Host 侧缺失调用身份的路径。
- R-1191：Web 四类默认切换复用认证用户 ID 调用 Runtime 门禁，不接受请求体伪造 caller 身份。
- R-1192：删除 LLM、Embedding、Image、Video 四套结构相同的 DB 默认切换函数，收敛为 `ProviderConfigKind` 与唯一 `set_default_provider_config`。
- R-1193：默认切换统一在单事务内清除旧默认并设置目标；目标不存在时事务回滚，不再留下零默认项。
- R-1194：settings-all 删除四套默认清理宏，复用同一 `clear_provider_config_defaults` DB 原语。
- R-1195：settings-all 严格拒绝同一 provider 在导入文件中声明多个默认项，删除按行顺序“最后一项获胜”的分叉语义。
- R-1196：新增不可逆迁移 `provider_default_config_integrity_contract`，为四表建立默认项 partial unique index。
- R-1197：迁移发现任一表已有重复默认项时直接失败，不自动挑选、覆盖或修复生产数据。
- R-1198：新增隔离 PostgreSQL 测试，验证重复默认迁移失败及四张表第二个默认项均被数据库拒绝。
- R-1199：新增隔离 PostgreSQL 原子切换测试，验证不存在目标不会清除现有默认项，成功切换后仍恰好一个默认项。
- R-1200：本机 `setSingleDefault` 改为只执行状态变更并返回 void，删除无人消费的目标对象返回值。
- R-1201：本机图片、视频默认 setter 与 LLM、Embedding 统一返回 void，删除 API 层的结果分叉。
- R-1202：本机配置删除不再把第一项隐式晋升为默认项，避免数组顺序决定业务状态。
- R-1203：本机配置删除统一拒绝删除仍生效的本机默认项，要求先显式切换默认来源或默认配置。
- R-1204：切换到服务器默认来源前先确认服务器确有默认配置，再写本机状态，失败时不留下虚假 system 来源。
- R-1205：使用仓库现有 `target/` 重新生成 Rust 到 TypeScript 合同和 Host 表，未在 `/tmp` 创建第二套构建产物。
- R-1206：退役的 activate 命令、四套 DB setter、旧返回 summary 和错误结果 schema 生产引用清零。
- R-1207：按累计超过 10 项集中完成合同、DB、迁移、settings-all、前端、生成器、Host parity 和生产构建验证。
- R-1208：默认配置当前只保留一套事实标准：Host 传递认证身份，Runtime 负责权限，Orchestrator 负责模型门禁，DB 事务与 PostgreSQL 约束负责唯一性。

## 数据库迁移

新增不可逆迁移 `provider_default_config_integrity_contract`：

- 迁移前确认四张 provider 配置表均存在。
- 任一表存在多个 `is_default = TRUE` 时明确失败，不执行静默修复。
- 四张表分别建立 partial unique index，只允许至多一行 `is_default = TRUE`。
- 默认项删除仍由 Orchestrator 的非默认删除门禁阻止，因此活跃写路径维持恰好一个默认项。

## 验证结果

- Rust 到 TypeScript 生成器：通过，使用仓库 `/home/fufu/Code/A4N/target/`。
- `cargo check -p a4n-api-contract -p a4n-db -p a4n-migration`：通过。
- API contract：22 个测试通过。
- DB 原子默认切换：1 个测试通过。
- settings-all：2 个测试通过。
- 默认配置迁移：1 个测试通过。
- Host parity：通过。
- 前端 Vitest：44 个测试文件、162 个测试通过。
- 前端生产构建：通过。
- 本机运行时设置目标测试：2 个测试通过。
- `cargo check -p a4n-runtime -p a4n-server -p a4n-tauri` 被并行改动中的 Orchestrator 编译错误阻断；日志中没有默认配置命令、caller macro、`ProviderConfigKind` 或默认切换路径诊断。
- 退役符号全仓搜索只剩两条“旧命令必须不存在”的合同断言，生产引用为 0。

## 当前事实标准

同一 provider 默认配置能力只保留唯一执行链：严格 Host 合同携带认证 caller，Runtime 执行 `ManageSettings` 门禁，Orchestrator 执行 provider 模型校验，DB 在事务内切换默认项，PostgreSQL 唯一索引兜住并发和旁路写入；本机配置只接受显式切换，不再自动晋升或回退。
