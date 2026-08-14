# Prompt 模板管理与项目级覆盖 BFS 冗余清理记录

日期：2026-08-14

## 搜索起点与覆盖范围

本专项从设置页的 Prompt 模板管理功能出发，按 BFS 逐层追踪 13 条活跃 Host 命令：

```text
prompt_list / prompt_list_categories / prompt_list_project_overrides
prompt_load_defaults / prompt_get（内部 sidecar 能力）
prompt_render / prompt_render_ab / prompt_validate_variables
prompt_update / prompt_reset
prompt_version_history / prompt_version_diff
prompt_set_project_override / prompt_remove_project_override
```

其中公开 Host 命令为 manifest 中的 13 条；`prompt_get` 是 Orchestrator 到 Prompt sidecar 的内部读取能力，不是第二条前端命令。

完整活跃调用闭包如下：

```text
Settings / PromptTab / AdvancedPanels
→ frontend/src/api/prompt 与 generated Host TypeScript
→ apiInvoke / Tauri invoke_host_command / Web command route / CLI manifest
→ a4n-server 字符串 dispatch 与严格参数解析
→ a4n-runtime host_api / prompt actions / RuntimeProfile
→ a4n-orchestrator PromptExecutionContext、权限与项目协作根作用域
→ embedded TeraPromptEngine 或 PromptClient sidecar 分派
→ a4n-prompt-service HTTP handlers / ServiceRequest envelope
→ a4n-prompt Tera、Registry、Markdown、include 与 override engine
→ a4n-prompt-git 模板修改历史
→ a4n-db PostgreSQL 项目级覆盖
→ AppState 启动 hydration / managed sidecar 环境 / CLI、Tauri 与部署布局
```

同时检查了以下显式和隐式依赖：

- `PromptRenderer`、`PromptClient` trait 实现和所有直接调用者。
- Cargo 模块导出、crate 依赖、serde/schemars DTO、生成 TypeScript 与 command manifest。
- Web JWT、Tauri Host 会话身份、项目 Editor 权限和协作根项目 envelope。
- RuntimeProfile 的 embedded/sidecar 生产分派，以及 sidecar capability 注入和 service manager 环境。
- `A4N_PROMPT_SERVICE_PROMPTS_DIR`、`A4N_PORTABLE_HOME`、当前目录、可执行文件、macOS Resources、Linux `share/a4n` 和部署脚本。
- `_registry.yaml`、78 个 Markdown 模板 frontmatter、Tera include、模板变量校验和 Git 版本历史。
- PostgreSQL settings key、项目删除级联、启动 hydration、退役模板覆盖清理和项目覆盖列表。
- 设置页存储路径展示、AppPaths、孤儿目录扫描和文件删除辅助路径。

仓库没有该功能对应的 XML DI、运行时反射注册、动态代理或 AOP 切面。动态入口只来自显式 Host manifest、字符串 dispatch、capability trait、环境配置、Prompt Registry 和数据库 key，均已纳入上述闭包。

## 唯一事实标准

- 随包 `prompts/`：只读默认模板、模板路径和中央 Registry。
- `{data_dir}/prompts.git`：用户模板修改内容与 Git 历史的唯一持久化来源。
- PostgreSQL：项目级 Prompt 覆盖的唯一持久化来源。
- `TeraPromptEngine`：模板解析、变量校验、排序、渲染和运行时覆盖的唯一实现。
- `ServiceRequest.project_id/root_project_id`：sidecar 项目作用域的唯一传输位置。
- `a4n-api-contract`：Host 输入输出与前端生成类型的唯一合同。

不存在 `data/prompts` 可写副本、Registry 合并副本、合同提示词局部副本、payload 内第二份 `project_id`、前端手写 Host DTO 或 Tauri 内第二套 Prompt 资源定位。

## 已完成优化

1. 删除启动时将随包模板复制到 `data/prompts` 的 portable prompts 逻辑。
2. 删除“用户目录 Registry 缺失时与随包 Registry 合并”的兼容路径。
3. 删除合同提示词从随包目录同步到局部可写目录的分叉实现。
4. 删除 Runtime 旧 `prompt_engine/contract.rs`、`portable.rs`、`registry.rs` 三个模块。
5. 删除 Orchestrator 旧 `prompt_override.rs` re-export 中间层，直接使用 DB owning module。
6. 删除只服务旧 portable prompts 的无调用文件回滚辅助函数及测试。
7. 删除 `AppPaths.prompts_dir`，不再把只读资源伪装成应用数据目录。
8. 删除 `StoragePathsSummary.prompts_dir`、设置页假 `data/prompts` 展示和对应生成类型字段。
9. 删除 Runtime 不再使用的 `serde_yaml` 依赖和 lockfile 记录。
10. 新增 Runtime 唯一随包 Prompt 资源定位器，embedded engine 与 managed sidecar 共用同一结果。
11. 显式 `A4N_PROMPT_SERVICE_PROMPTS_DIR` 或 `A4N_PORTABLE_HOME` 缺少 `_registry.yaml` 时直接失败，不回退到其他候选目录。
12. 保留 CLI 启动子进程前对安装布局的必要探测；删除同一进程内 macOS Tauri 对 `Resources/prompts` 的重复定位和校验。
13. 前端 Prompt API 删除重复 `userId`；身份只由 Web JWT 或 Tauri Host 会话提供。
14. `prompt_update` 的 author 改由后端认证用户生成，删除客户端可伪造 author 的合同字段。
15. `RenderInput.project_id` 必须显式传 `null` 或正整数，禁止省略作用域。
16. `RenderInput.template_name`、`variables`、`project_id` 的 JSON Schema 全部设为必填，修复 schema 扩展覆盖原 required 列表的问题。
17. Prompt sidecar render payload 删除重复 `project_id`，只读取 `ServiceRequest.project_id`。
18. Prompt sidecar set override payload 删除重复 `project_id`，只读取 envelope 作用域。
19. 删除 `PromptRemoveProjectOverridePayload`，remove 复用严格的 `PromptNamePayload`。
20. hydration item 删除逐项重复 `project_id`，一次请求只允许一个 envelope 项目作用域。
21. set/remove 的 embedded wrapper 改为同步函数，删除没有异步工作的 `async/.await`。
22. Prompt sidecar 缺少项目作用域时显式报错，不使用全局覆盖或默认项目。
23. `PromptEntry` 的 description、path、version 等活跃字段改为必填，删除 `null` 合同分叉。
24. `PromptCategory` 的 label、description 改为必填，前端不再处理不存在的空分类元数据。
25. 版本历史 sidecar 输出改为专用 summary，删除列表响应中无人消费的完整 `content_snapshot`。
26. A/B 渲染字段从误导性的 `version_a/version_b` 改为真实语义 `current/original`。
27. Markdown frontmatter 只允许本地 `name`，其余元数据全部归中央 Registry 所有。
28. 清理 78 个模板 frontmatter 中重复的 title、description、tags、category、version、used_by、dependencies 和 path。
29. `_registry.yaml` 删除无人读取的顶层 `version`、`last_updated`。
30. Registry 顶层、Prompt entry 和 category 全部启用 `deny_unknown_fields`，缺失或未知字段直接失败。
31. Git 历史中的非法时间戳和非 UTF-8 blob 改为显式错误，删除伪造时间和跳过记录的静默路径。
32. update/reset 删除第二次模板编译，校验结果只计算一次。
33. `list_all()` 排序下沉到 Engine，删除 Prompt service 的第二次排序。
34. 项目覆盖 DB content 收紧为 `String`，删除运行时重复 JSON 类型判断。
35. DB 查询只扫描 Prompt override key，删除读取全部 settings 后再过滤的分叉。
36. 启动 hydration 支持按项目整体替换，避免旧覆盖在重复 hydration 后残留。
37. sidecar 项目 hydration、set、render、remove 全部使用同一 envelope 作用域和协作根项目。
38. Host 生成类型重新生成，删除旧 `prompts_dir`、nullable 元数据、旧 A/B 字段和旧可选 RenderInput。
39. command manifest 增加完整 required 集合断言，防止生成合同再次把模板名或变量标成可选。

## 明确保留的活跃边界

- embedded 与 sidecar 是两个明确部署模式；sidecar 模式缺 capability 时直接失败，不回退 embedded。
- CLI 的安装布局探测发生在启动 Server 子进程之前，Runtime crate 不在 CLI 依赖图中，不能用删除它来换取表面上的单实现。
- 项目覆盖写入失败时恢复运行时状态属于跨 DB/内存边界的一致性补偿，不是业务默认值或旧格式兼容。
- `PromptRenderer` trait 被多个生成工作流注入，是真实依赖倒置边界，不是单实现占位接口。
- Registry、Markdown 本地 name 和 Git 历史承担不同职责，不合并为第二份大配置文件。

## 集中验证

本批超过 10 项优化后集中验证。Cargo 命令复用仓库现有 `/home/fufu/Code/A4N/target`；源码未在 `/tmp` 修改。独立进程测试和生成合同脚本创建的 `/tmp` 目录仅承载可删除测试数据。

实际结果：

- 11 个相关 crate 集中 `cargo check`：通过，包括 Prompt、Git、API contract、capabilities、DB、Orchestrator、Runtime、Prompt service、Server、CLI、Tauri。
- `a4n-api-contract`：65/65 通过。
- `a4n-prompt`：77/77 通过。
- `a4n-prompt-git`：12/12 通过。
- `a4n-capabilities`：25/25 通过。
- 隔离数据库 `prompt_override` 专项：19/19 通过，串行使用仓库 `.env` 的 `A4N_TEST_DATABASE_URL`。
- Orchestrator Prompt envelope/response 专项：2/2 通过。
- Prompt service 独立进程测试：1/1 通过，覆盖 HTTP 严格合同、项目覆盖 set/render/remove、Git 备份恢复和损坏 manifest 拒绝启动。
- Web API parity、generation schema/TypeScript/fixture 校验：通过。
- 前端 Vitest：57 个文件、195/195 通过。
- 前端 `tsc && vite build`：通过。
- `cargo fmt --all -- --check` 与 `git diff --check`：通过。

受 60 秒后端测试上限约束：

- `a4n-db --lib` 共 565 项，完成链接并开始执行后超时；已执行项无失败，随后运行本专项 19 项精确测试并全部通过。
- Runtime 目标测试二进制连续两个 60 秒窗口都停在链接阶段，未执行；相关 crate 集中编译已通过，资源目录行为由 Prompt service 独立进程测试和显式目录单测代码覆盖。
