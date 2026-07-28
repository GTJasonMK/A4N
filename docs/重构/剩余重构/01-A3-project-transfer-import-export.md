# 工作包 01：A3 Project Transfer / Import / Export 收口

## 任务卡

| 项目 | 内容 |
|---|---|
| 主目标 | 将 project transfer、复制、导入、导出等完整业务流程收口到 Orchestrator application |
| V2 来源 | 3.10、阶段 A；进度看板的 `project_transfer`、`project_duplication`、`import_*`、`export_files` |
| 当前状态 | 已收口；批次 303 已结合本包 owner/结构证据和工作包 09 生命周期证据关闭 V2 3.10/阶段 A，文末旧待办只保留历史过程 |
| 可并行性 | 可与 02–05 并行；共享媒体生命周期交给 09，不在本包新建清理机制 |

## 当前事实

- 文本、family、video_studio 的若干 import/export 校验和组装已经有 Orchestrator owner，不能从头重写。
- Runtime 仍可能保留 archive、manifest、媒体磁盘、ZIP/路径 IO、DTO 投影和 Host 权限边界；这些不自动等于业务 owner 回流。
- `import_analysis`、`import_reverse`、`import_review`、`import_file`、`export_files`、`project_duplication` 和 `project_records` 仍需逐入口判定，不要用目录名称推断迁移完成。

## 范围

1. 盘点每个公开 Host API 的输入校验、项目归属、快照/章节选择、事务写入、workflow/checkpoint、错误映射和文件副作用。
2. 完成 project transfer 的剩余 import/export 业务编排；Orchestrator 负责合同、权限、不变量、事务和状态，Runtime 只做 IO/DTO/Host port。
3. 完成 project duplication 的复制事务与 source snapshot/derived media 归属；复用现有 `ProjectDuplicationApplication`，不重新建立第二套复制器。
4. 收口 import analysis/reverse/review 的 postprocess、accept/modify、失败终态和可重试边界；复用已有 application/context owner。
5. 收口 export 的选择、元数据、顺序稳定性和错误语义；文件写出保留在 Host port，业务选择和 DB 读取归 owner。
6. 删除无消费者的 Runtime request/workflow/DB facade/reexport；为每个迁移片区增加唯一 owner 结构门禁。

## 不在范围

- 不修改 `cleanup outbox`、TTL、跨重启恢复和 DB/文件失败组合；由工作包 09 统一处理。
- 不创建 `a4n-api-contract` 或迁移前端 DTO；由工作包 06 处理。
- 不修改 sidecar 认证、限流、版本协商和 process matrix；由工作包 07/08 处理。
- 不为了减少 `AppState` 文本命中而搬动合法文件 IO、ZIP、归档或权限代码。

## 建议执行顺序

1. 先用当前源码重新生成 endpoint/owner inventory，标记 `[application]`、`[Host IO]`、`[DTO]`、`[未分类]`。
2. 选择一个完整纵向切片（建议先 video_studio export 或 project duplication 的剩余路径），先确认 Orchestrator owner 再迁调用点。
3. 迁移成功、失败、取消、重复请求和部分导入路径；删除旧 owner，不保留无期限 wrapper。
4. 为文件副作用输出生命周期交接表，交给 09，不在本包复制 outbox 状态机。
5. 做局部验证，再由协调者运行跨 crate/Host/前端门禁。

## 主要路径（先核对后修改）

```text
crates/a4n-runtime/src/actions/project_transfer/**
crates/a4n-runtime/src/actions/project_duplication/**
crates/a4n-runtime/src/actions/import_analysis/**
crates/a4n-runtime/src/actions/import_reverse/**
crates/a4n-runtime/src/actions/import_review/**
crates/a4n-runtime/src/actions/import_file/**
crates/a4n-runtime/src/actions/export_files/**
crates/a4n-runtime/src/actions/project_records/**
crates/a4n-runtime/src/actions/chapter_content/**
crates/a4n-orchestrator/src/{project,import_reverse,source_sync,export,context}/**
crates/a4n-db/src/{project,import,export}/**
```

目录可能随并行批次变化；以 `rg` 和当前模块树为准，不按上面路径盲目创建新文件。

## 关闭条件

- 每个纳入范围的公开 Host API 都有唯一的 Orchestrator application 方法。
- Runtime 不再拥有可独立跑完的 import/export/duplication 业务流程、workflow 状态推进或 DB 事务。
- 文件和 ZIP 读写通过已有 Host port；业务失败、补偿意图和重试交给 09 的统一生命周期。
- 成功、失败、取消、重复请求和部分失败均有显式结果；没有 fallback 或静默跳过。
- 旧 owner、无消费者 facade 和重复测试已删除，结构门禁能阻止回流。

## 最小验证

```bash
cargo fmt --all -- --check
git diff --check
timeout 60 cargo check -p a4n-db
timeout 60 cargo check -p a4n-orchestrator
timeout 60 cargo check -p a4n-runtime --no-default-features
timeout 60 cargo check -p a4n-server --no-default-features
timeout 60 cargo check -p a4n-tauri --no-default-features
./scripts/checks/check-task-contract-owner.sh
```

涉及数据库事务时必须在 `A4N_TEST_DATABASE_URL` 可用的环境执行定向集成测试；没有数据库时只报告编译结果，不能写成 PostgreSQL 通过。

## 交接要求

在 `README.md` 的交接格式基础上，附上：每个入口的 owner 表、保留的 Host IO 列表、移交给 09 的文件副作用清单，以及明确未迁的入口。不要自行修改 V2 进度数字。

## 交接记录（2026-07-24）

工作包：`01-A3-project-transfer-import-export`  
状态：`进行中（已交接，待 09/协调者）`

### 基线

- 本包只处理 A3 的 project transfer / duplication / import / export 入口；不改 V2 全局完成数字，也不接管工作包 09 的生命周期状态机。
- 当前工作区存在其他批次的并行改动；本交接记录只描述本包相关的 owner 收口，不对无关脏改动做整理或回滚。
- 关闭判定以真实 owner 和结构门禁为准，不以 Runtime 行数、`AppState` 命中数下降为准。

### 修改路径与 owner 收口

| 入口组 | 唯一 owner | Runtime 合法保留 |
|---|---|---|
| project duplication / branch | `a4n-orchestrator/src/context/project_duplication.rs` 的 validated application | 媒体复制、Host 权限、cleanup outbox、DTO；不保留复制事务或项目不变量 |
| family package export/import | `a4n-orchestrator/src/context/project_family_transfer.rs`；纯 package 规则在 `a4n-orchestrator/src/project/family_package/**` | ZIP/sidecar 读写、媒体磁盘写入、路径映射、失败 cleanup、DTO |
| video studio transfer | `a4n-orchestrator/src/context/video_studio_transfer.rs` | ZIP/媒体物理 IO、Host port、cleanup、DTO |
| text project full import | `a4n-orchestrator/src/project/text_project_import.rs`（落库复用 project mutation owner） | 文件名、Base64、UTF-8、JSON 边界解析和 Chapter Host port |
| import analysis | `a4n-orchestrator/src/import_analysis/application.rs`；progress/cancel/ingest 复用同一 orchestrator 模块 owner | PromptRenderer、Chapter/Character Host port、trace、权限、进度/DTO 适配 |
| import reverse | `a4n-orchestrator/src/import_reverse/application.rs` | 模板变量解析、预览 DTO、权限和 progress callback 适配 |
| import review accept/modify/batch/dedup | `a4n-orchestrator/src/context/import_review.rs` + `import_review/rules.rs` | Chapter cleanup Host port、权限调用边界、DTO；不保留 DB 写入、后处理、批量循环或 SimHash 阈值 |
| export files | `a4n-orchestrator/src/export/application.rs` | task scope/权限、进度回调和 DTO；不保留章节读取、排序、EPUB/PDF/JSON 生成 |
| uploaded import file | `a4n-import-core` parser；Runtime `actions/import_file` 是上传边界 | Base64/data URL、文件类型选择、parser 调用、结果 DTO；无 DB/workflow/project state |

本包实际触碰的主要文件片区：

- `crates/a4n-orchestrator/src/context/import_review.rs`
- `crates/a4n-orchestrator/src/import_review/{mod.rs,rules.rs}`
- `crates/a4n-runtime/src/state/import_review.rs`
- `crates/a4n-runtime/src/actions/import_review/{acceptance.rs,dedup.rs}`
- `crates/a4n-runtime/src/actions/project_transfer/family_package/**`
- `crates/a4n-runtime/src/actions/project_transfer/family_package/archive/payload.rs`
- `crates/a4n-runtime/src/state/convergence_tests.rs`
- `crates/a4n-orchestrator/src/project/family_package/**`（纯校验/组装迁移后的 canonical owner）

### 删除的 owner / facade

- 删除 Runtime family package 的纯规则/旧 application 子树：`export/**`、`import_rows/**`、`validation/**`、`rules.rs`、`archive/lifecycle.rs` 及其无消费者测试/契约文件；规则和 package 组装统一归 `a4n-orchestrator::project::family_package`。
- 删除 Import Review Runtime 的低层 `accept_import_review_validated`、`modify_import_review_validated`、`import_dedup_chapters` 旁路以及 Runtime 后处理/批量循环；窄 facade 只保留四个高层 validated application 入口。
- 未新增兼容 wrapper、第二套 SimHash/阈值、第二套 cleanup 状态机或第二套导出格式实现。

### 文件副作用（交给工作包 09）

- family/video studio package 的 ZIP 和媒体文件读写。
- duplication 复制出的媒体文件及失败时 cleanup outbox intent。
- text import / import review 章节派生数据后处理涉及的媒体清理。
- 现有 `cleanup_outbox` intent/claim/retry/recovery 机制继续作为唯一生命周期 owner；本包不扩展其跨重启语义。

### 结构门禁与验证

新增/强化 `crates/a4n-runtime/src/state/convergence_tests.rs`：

- `import_review_runtime_uses_application_owner`
- `project_transfer_import_export_residuals_use_application_owners`
- 既有 duplication、family transfer、video studio transfer owner gates 继续覆盖对应路径。

已执行：

```text
timeout 120 cargo check -p a4n-orchestrator -p a4n-runtime --no-default-features
结果：通过（仅既有 dead_code warning）

rustfmt --edition 2024 --check <本包触碰的 Rust 文件>
结果：通过

timeout 180 cargo test -p a4n-runtime --no-default-features \
  state::convergence_tests::project_transfer_import_export_residuals_use_application_owners \
  -- --exact --nocapture
结果：1 passed

timeout 60 cargo test -p a4n-runtime --no-default-features \
  state::convergence_tests::import_review_runtime_uses_application_owner \
  -- --exact --nocapture
结果：1 passed

timeout 60 cargo test -p a4n-runtime --no-default-features \
  transfer_runtime_uses_application_owner -- --nocapture
结果：family + video studio 2 passed

timeout 60 cargo test -p a4n-runtime --no-default-features \
  state::convergence_tests::project_duplication_runtime_uses_application_owner \
  -- --exact --nocapture
结果：1 passed

timeout 90 cargo test -p a4n-orchestrator --lib --no-default-features \
  import_review::rules::tests -- --nocapture
结果：4 passed

timeout 90 cargo test -p a4n-orchestrator --lib --no-default-features \
  project::family_package::tests -- --nocapture
结果：231 passed

git diff --check
结果：通过

bash scripts/checks/check-task-kind-boundary.sh
结果：通过

bash scripts/checks/refactor-metrics.sh
结果：通过；本地纯 rules / orch 透传壳均为 0 / 0
```

未执行或未通过：

- `A4N_TEST_DATABASE_URL` / `DATABASE_URL` 当前未配置，未声称 PostgreSQL 事务集成测试通过。
- 第一次 60 秒定向测试因另一会话占用 Cargo 构建锁并触发 test-profile 编译而退出 124；锁释放后以 180 秒重跑，1 passed。
- `bash scripts/checks/check-core-boundary.sh` 失败：并行工作区新增了 `a4n-core::dependency_conflict` 公共模块，但共享 allowlist 尚未同步。本包未修改该共享 core 合同文件，交给工作包 06/协调者处理。
- 计划中的 `./scripts/checks/check-task-contract-owner.sh` 在仓库不存在；实际可用的相关门禁是 `check-task-kind-boundary.sh`、`check-core-boundary.sh`、`check-sidecar-*` 和 `refactor-metrics.sh`。

### 未关闭条件 / 依赖

- V2 3.10 和阶段 A 是跨多个 A3 工作包的全局条件，不能由本包单独改为 `完成`。
- 工作包 09 仍需统一确认媒体副作用、cleanup outbox、跨事务失败和恢复语义；本包只提供 owner/副作用清单。
- 协调者合并其他 A3 工作包后，应集中重跑 runtime/orchestrator/server/tauri 编译、结构门禁及可用的 PostgreSQL/process 测试。

### 并行复核交接（2026-07-25）

- 本次未修改 project transfer、duplication、import/export owner 或 09 生命周期代码；沿用既有 owner 表与结构门禁结果。
- 当前本地 PostgreSQL 已可用，但 Orchestrator 聚合测试在进入 runner 前受 09 `context/generation.rs` 缺少 `GenerationError`、`TaskPermit`、`GenerationTaskPersistenceError` 等符号的编译错误阻塞。
- 09 交接后由协调者集中重跑本包的 runtime convergence、Orchestrator rules 和 PostgreSQL transaction tests；未把旧的“未配置数据库”或当前编译失败写成通过。

### 聚合 runner 复验（2026-07-25，取代上述编译阻塞结论）

- 09 generation owner 当前已恢复编译；本轮只验证既有 owner，没有修改 project transfer、duplication、import/export 或 09 活跃路径。
- Runtime convergence：
  - `project_transfer_import_export_residuals_use_application_owners`：1/1 通过。
  - `transfer_runtime_uses_application_owner`：2/2 通过（family transfer、video studio transfer）。
  - `project_duplication_runtime_uses_application_owner`：1/1 通过。
- Orchestrator 规则：
  - `import_review::rules::tests`：4/4 通过。
  - `project::family_package::tests`：231/231 通过。
- 本轮未宣称 PostgreSQL transaction、跨重启文件补偿或远端 artifact 证据；这些仍由 07/09/协调者负责。
