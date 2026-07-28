# 工作包 04：A3 Chapter / Scene / Creative 小域

## 任务卡

| 项目 | 内容 |
|---|---|
| 主目标 | 收口章节场景、角色复审/冲突、源同步、拍摄脚本和蓝图等仍有完整编排的创作域 |
| V2 来源 | 3.10、阶段 A；进度看板的 `character_implicit_review`、`character_conflict`、`chapter_scene`、`scene_profile_sync`、`source_sync`、`shooting_script_records`、`blueprint_records` |
| 当前状态 | 已收口；source_sync 105/105、shooting_script 24/24 和 06/09 共享边界均已复验，批次 303/305 已关闭 V2 3.10/阶段 A |
| 可并行性 | 可与 01–03 并行；共享 `chapter`/`workflow` 根模块改动先报协调者 |

## 范围

1. `character_implicit_review`：手动 summary/preview/check、自动同步复审、参数/上下文统计、LLM、evidence、决策落库、RAG 和结果投影。
2. `character_conflict`：冲突上下文、请求、LLM/checkpoint、严格输出/source_id 校验、决策与结果写入。
3. `chapter_scene` / `scene_profile_sync`：场景生成、章节范围、已有场景上下文、workflow、生成结果和同步落库。
4. `source_sync`：预览已归 owner 的部分只做回流审计；仍在 service/Runtime 的决策执行、派生/冲突处理需收口。
5. `shooting_script_records`：章节来源解析、发布/编辑失效已基本有 owner；只迁尚未统一的 LLM 生成编排和 Host 适配。
6. `blueprint_records`：DB/application owner 已存在；审计 generate/refine、角色/关系同步和项目状态事件是否仍有 Runtime 完整流程。
7. `chapter_content` 的 workflow preflight/postprocess 钩子：不重复已完成的 edit/import application，只处理剩余执行边界。

## 不在范围

- `chapter_generation` 已在批次 213 收口；不重做 RAG/context/prompt/streaming application。
- Character discovery/profile-sync、Canvas、Galgame 主体由 03 负责；本包只处理 implicit review/conflict 与明确列出的共享创作域。
- 不把外部 OAuth、JWT、文件 IO、provider transport 误迁为业务 application。
- 不新建 workflow、checkpoint、RAG 或事件第二来源。

## 建议执行顺序

1. 先分离“纯规则 / DB owner / LLM execution / Host IO / event”五类责任。
2. 先完成一个最小纵向链（建议 conflict 或 chapter_scene），验证失败、取消、恢复和结果投影。
3. 将同步/再生成/决策落库接到已有 context/application，不通过 `AppState` 完整上下文旁路。
4. 删除 Runtime 的 parsing/request/workflow/checkpoint/record 旧 owner和重复测试。
5. 为 shared chapter/workflow 模块修改补充结构门禁，防止其他 agent 回流。

## 关闭条件

- 纳入范围的每个完整执行链有唯一 Orchestrator application owner。
- LLM 请求、context manifest、checkpoint、严格输出校验、决策/结果写入和 workflow 终态不再由 Runtime 组合。
- 章节/角色/源同步的范围、归属、source_id 和重复请求不变量集中在一个 owner。
- 合法 Host 仅保留 provider/embedding/file/进度/DTO/外部 IO，并有明确理由。
- 结构门禁、定向测试和必要的 PostgreSQL 测试均有真实结果。

## 最小验证

```bash
cargo fmt --all -- --check
git diff --check
timeout 60 cargo check -p a4n-orchestrator
timeout 60 cargo check -p a4n-runtime --no-default-features
timeout 60 cargo test -p a4n-orchestrator --lib character::implicit_review
timeout 60 cargo test -p a4n-orchestrator --lib character::conflict
timeout 60 cargo test -p a4n-orchestrator --lib chapter_scene
timeout 60 cargo test -p a4n-runtime --no-default-features chapter_scene
```

## 交接要求

每个子域单独报告 owner 和状态，不把“DB owner 已完成”写成“完整 application 已完成”；把尚未迁的 workflow、LLM、文件和事件边界明确交给对应工作包。

## 交接记录（2026-07-24）

### 工作包状态

待合并。基线为批次 213 之后的共享工作区；工作区同时存在其他会话的并行改动，本轮只修改本包涉及的 Orchestrator/Runtime 文件、对应结构门禁和本工作包文档，没有回滚、整理或覆盖其他会话的改动。`docs/重构/剩余重构` 仍受既有 ignore 规则影响，协调者需由工作包 11 统一决定是否放行入 Git。

### 子域唯一 owner 与状态

| 子域 | Orchestrator 唯一 owner | Runtime 仅保留 | 本轮状态 |
|---|---|---|---|
| `character_implicit_review` | `character/implicit_review/execution.rs`；自动同步入口由 `character/profile_sync/execution.rs` 调用 | 权限、`RuntimePromptRenderer`、请求预览 DTO、进度/任务适配 | 完整 summary/preview/check、LLM、evidence、严格输出、决策落库和同步复审已在 application；本轮审计通过，未复制 owner |
| `character_conflict` | `character/conflict/execution.rs` 及其 context/output/workflow rules | 权限、Prompt renderer、请求预览和进度回调 | context manifest、checkpoint 重校验、source id/输出规则、决策写入和终态由 Orchestrator；本轮审计通过 |
| `chapter_scene` | `chapter_scene/execution.rs`、`context/project_scene_records.rs` | 章节访问校验、Prompt renderer、进度与 DTO 投影 | 生成/预览、章节范围、场景记录更新和排序均为 Orchestrator owner；本轮审计通过 |
| `scene_profile_sync` | `scene_profile/application/{preview,execution}.rs`，章节分析负责调用编排 | `RuntimeSceneProfileSyncHostPort`（RAG ingest/trace）、Prompt renderer、权限和进度 | workflow、解析、同步落库和恢复规则由 Orchestrator；RAG 外部 IO 保留 Host port |
| `source_sync` | `source_sync/service.rs`（apply workflow/决策/终态）与 `context/source_sync.rs`（读模型/派生项目 detach） | task queue 包装、权限、DTO、预览/运行结果投影 | Runtime 不再拥有 decision preparation、冲突策略解析、DB transaction 或 workflow begin/finalize |
| `shooting_script_records` | `context/shooting_script.rs`（来源解析、发布、当前版本校验、编辑）与 `shooting_script/generation.rs`（LLM 生成） | 权限、Prompt renderer、进度/取消、DTO | 新增 `edit_current_chapter_shooting_script_validated`；编辑必须匹配当前 source record，低层公开编辑 wrapper 已删除 |
| `blueprint_records` | `context/project_blueprint.rs`（版本读写、diff/restore、事件）与 `blueprint/{generation,refine}/execution.rs` | 权限、Prompt renderer、请求预览、DTO | diff/restore 已收回 Orchestrator；Runtime 不再组合 JSON diff、读取两个版本、校验 restore 或发布事件 |
| `chapter_content` | `chapter_content/application.rs` | `RuntimeChapterAnalysisHostPort`、权限、章节查询/DTO；展示投影可读取 workflow 状态 | preflight、导入/编辑、派生清理、postprocess、状态刷新均由 application；Runtime 不含 DB/workflow 组合 |

### 本轮删除与收口

- 删除 `crates/a4n-runtime/src/actions/blueprint_records/diffing.rs` 及其 `json_diff` 第二实现；`versions.rs` 改为调用 `diff_blueprint_versions_for_user_validated` / `restore_blueprint_version_for_user_validated`。
- 蓝图版本权限、非空版本校验、版本读取、递归 diff、restore 事件发布全部集中到 `crates/a4n-orchestrator/src/context/project_blueprint.rs`。
- 删除 Runtime shooting-script edit 中的当前来源读取、`record.id` 比较和低层 edit 旁路；由 `edit_current_chapter_shooting_script_validated` 一次完成来源存在性、record id 不变量和 DB edit/invalidate。
- 删除 Orchestrator Context 对低层 `edit_chapter_shooting_script` 的公开 wrapper；没有新增兼容层或第二套规则。
- 更新 `crates/a4n-runtime/src/state/convergence_tests.rs` 的 blueprint、shooting-script、source-sync、chapter-content 门禁，增加旧 owner 文件、低层旁路、DB/workflow transaction 和 application owner 断言。

### 合法 Host 边界与副作用

- 合法保留：权限检查、`RuntimePromptRenderer`、task queue 与 `TaskFamilyContext`、进度/取消回调、RAG/embedding/file/provider 外部 IO、Host port、DTO 和只读展示 projection。
- 蓝图 diff 是只读操作；restore 先由 DB owner 写入，再由 Orchestrator 发布 `ProjectStatusChanged`（有状态变化时）和 `BlueprintUpdated`，Runtime 不直接碰 event bus。
- 拍摄剧本编辑由 DB owner 执行脚本 schema 校验、乐观版本更新和派生项目失效，Runtime 只做权限和 DTO 投影。
- Source Sync 的 workflow begin、决策准备/严格校验、transaction、apply 和 finalize 均在 Orchestrator；Runtime 只包装任务调度并投影结果。
- Chapter Content 的 preflight、派生清理、正文导入/编辑、后处理、章节处理和项目状态刷新均在 Orchestrator；Host port 只执行外部派生媒体清理/分析同步。

### 验证证据

- `cargo check -p a4n-orchestrator -p a4n-runtime --no-default-features`：通过（55.23s）。
- `cargo check -p a4n-server -p a4n-tauri --no-default-features`：通过（约 65s）。
- 目标文件 `rustfmt --edition 2024 --check`：通过；`git diff --check`：通过。
- `cargo test -p a4n-orchestrator --lib --no-default-features version_diff_tests -- --nocapture`：1 passed。
- `cargo test -p a4n-orchestrator --lib --no-default-features current_record_tests -- --nocapture`：1 passed。
- `cargo test -p a4n-orchestrator --lib character::implicit_review -- --nocapture`：9 passed；`character::conflict`：9 passed；`chapter_scene`：13 passed。
- `cargo test -p a4n-orchestrator --lib --no-default-features blueprint -- --nocapture`：56 passed。
- `cargo test -p a4n-orchestrator --lib --no-default-features source_sync -- --nocapture`：104 passed；1 个 PostgreSQL 用例因缺少 `A4N_TEST_DATABASE_URL` 在初始化处失败。
- `cargo test -p a4n-orchestrator --lib --no-default-features shooting_script -- --nocapture`：22 passed；2 个 PostgreSQL 用例因缺少 `A4N_TEST_DATABASE_URL` 在初始化处失败。
- `cargo test -p a4n-runtime --no-default-features state::convergence_tests -- --nocapture`：67 passed。旧 `GenerationRuntimeFacade` 已无消费者并已删除，结构门禁改为禁止该 facade 与 `AppState::generation` 回流，没有恢复兼容壳。
- `scripts/checks/check-task-kind-boundary.sh`、`scripts/checks/check-refactor-docs.sh`、`git diff --check`：通过。
- PostgreSQL 断言：本次已配置隔离 PostgreSQL 18.4，`source_sync` 定向命令先因 60 秒冷编译超时，随后 300 秒命令在 runner 前因 `crates/a4n-orchestrator/src/context/generation.rs` 缺少 `GenerationError`、`TaskPermit`、`GenerationTaskPersistenceError` 等 09-owned 符号而编译失败；没有把失败或未进入 runner 写成通过。`shooting_script` 因同一 crate 编译阻塞未重复执行。

### 未关闭条件与依赖

- 本记录只表示工作包 04 代码已收口、等待协调者合并，不等价于 V2 3.10 或阶段 A 全局完成。
- 依赖工作包 06 处理公共 core/API 边界；工作包 07 提供真实 PostgreSQL/process 证据；工作包 09 在所有 A3 owner map 合并后处理跨域媒体生命周期与补偿；工作包 11 负责编译反馈、文档入 Git 和最终状态一致性。
- 未迁移或不应迁移的内容：chapter generation 主链、Character discovery/profile-sync 主体、Canvas/Galgame 主体、OAuth/JWT、文件/provider transport；这些由既有工作包或 Host 边界负责。

### 并行复核交接（2026-07-25）

- 04 范围内的 owner/结构门禁没有新增代码缺口；`source_sync`、`shooting_script` 的 PostgreSQL 证据仍待 09 generation owner 编译恢复后重跑。
- 09 交接前不修改 `context/generation.rs`、generation imports 或共享 `chapter/workflow` 根模块；协调者应先收敛 09 的编译状态，再按本文件命令集中复验。

### PostgreSQL 定向复验（2026-07-25）

前述“09 generation owner 编译失败”已经被当前工作树事实取代。本轮没有修改 04 或 09 的源码，只在本地 PostgreSQL 18.4 上重跑既有定向 runner：

```text
A4N_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:55432/postgres \
DATABASE_URL=postgres://postgres@127.0.0.1:55432/postgres \
cargo test -p a4n-orchestrator --lib --no-default-features source_sync -- --nocapture --test-threads=1
结果：105/105 passed（此前 104 passed + 1 个缺数据库失败）

A4N_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:55432/postgres \
DATABASE_URL=postgres://postgres@127.0.0.1:55432/postgres \
cargo test -p a4n-orchestrator --lib --no-default-features shooting_script -- --nocapture --test-threads=1
结果：24/24 passed（此前 22 passed + 2 个缺数据库失败）
```

两个命令均实际进入 runner，PostgreSQL 场景不再是 04 的未关闭条件；04 的 owner/结构与定向数据库证据现已收口。工作包仍等待 09 的跨域生命周期补偿、06 的公共 Core/API 边界以及协调者最终合并，不能单独把 V2 3.10 或阶段 A 改为完成。
