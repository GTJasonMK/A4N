# 工作包 03：A3 Character AI / Canvas / Galgame 收口

## 任务卡

| 项目 | 内容 |
|---|---|
| 主目标 | 将 Character discovery/profile-sync、Canvas workflow 和 Galgame pipeline 的完整执行链收口到 Orchestrator |
| V2 来源 | 3.10、阶段 A；进度看板 `character_ai`、`canvas_workflow`、`galgame_records` |
| 当前状态 | 已收口；Character 79/79、Canvas 47/47、Galgame 105/105 及 06/09 共享边界已由批次 303/305 纳入 V2 3.10/阶段 A 关闭 |
| 可并行性 | 三个子域可并行；共享模块根、Cargo 文件和事件/媒体框架需协调者合并 |

## 当前事实

- Character discovery、object discovery、implicit review 的部分 owner、Portrait application 和 Galgame asset generation 已经存在；不要从 Runtime 复制新 parser、prompt renderer、generation record 或 media binding 逻辑。
- Canvas Text/Image/Video 节点 application 和媒体导入已有多批收口；图 runner、节点循环、跨节点 workflow、provider/文件 Host 边界仍是重点。
- Galgame DB/application 和资产生成主要 owner 已存在，但 pipeline/export/媒体绑定的完整编排仍可能由 Runtime 持有。

## 本轮收口记录（2026-07-24）

- Character：`CHARACTER_PROFILE_SYNC_STEPS` 迁到 Orchestrator 作为唯一常量；Runtime 只保留完整同步 application 委托、Prompt/进度/取消 Host 和 DTO 投影。
- Canvas：图 runner、resume、节点 checkpoint/终态继续由 Orchestrator 持有；图片节点删除重复项目权限检查，Host 仅提供 provider/文件/队列边界。
- Galgame：脚本、资产规划/生成、pipeline、媒体绑定/音频导入和 WebGAL export 均经用户验证的 Orchestrator application；Runtime 删除无消费者的低层 facade 和悬空 re-export，WebGAL 文件读取保留 Host Port。
- 结构度量：Runtime actions 生产 `538` 文件 / `28,735` 行，含测试 `34,786` 行；`AppState` 引用 `264` 文件 / `880` 次；actions 生产代码 `.orchestrator()` `0`。

## 范围

### Character AI

1. 收口 discovery/profile-sync 的 staged LLM、checkpoint、evidence、RAG/结果投影和取消/恢复组合。
2. 复核 runtime 只保留 `RuntimePromptRenderer`、progress/cancel、provider/embedding/file Host 和 DTO。
3. 与工作包 04 的 implicit review/conflict 明确边界：本包负责 discovery/profile-sync/asset 生成接线，04 负责 review/conflict 执行链。

### Canvas

1. 收口 graph runner、run/resume、节点状态和 shared/owned workflow 的业务 owner。
2. Text/Image/Video 节点共用节点生命周期、checkpoint 和 source contract；删除重复节点状态机。
3. provider、队列、文件、FFmpeg、媒体资产操作通过已有 Host port；不在本包修改统一生命周期实现。

### Galgame

1. 收口 script/pipeline/export/asset binding 的业务决策、状态、失败补偿和结果投影。
2. 复用 `GalgameApplication` 与 `asset_generation_application`，不要新增 parallel application。
3. 保留真实媒体/文件 Host；将跨事务补偿需求交给 09。

## 不在范围

- `chapter_generation`、`video_prompts`、`video_records`、`image_generation`、`profile_material` 已收口，只做回流审计。
- 不重做纯 rules、TaskKind、AppState 私有化或 service catalog。
- 不修改公共 API DTO 归属；工作包 06 统一处理。

## 主要路径（先核对）

```text
crates/a4n-runtime/src/actions/character_ai/**
crates/a4n-runtime/src/actions/canvas/**
crates/a4n-runtime/src/actions/galgame_records/**
crates/a4n-orchestrator/src/{character,canvas,galgame}/**
crates/a4n-orchestrator/src/context/{character,canvas,galgame}.rs
crates/a4n-runtime/src/host_api/{character_ai,canvas,galgame}*.rs
```

## 建议执行顺序

1. 先为每个子域建立「输入合同—context—workflow—capability—persistence—output」清单。
2. 选择一条可独立验证的纵向链，先建立/确认 application owner，再迁移 Host 调用。
3. 用真实 checkpoint/取消/失败测试证明恢复路径，而不是只测试 happy path。
4. 删除旧 Runtime owner、薄 wrapper 和重复测试；增加禁止回流的结构扫描。
5. 把文件副作用、事件顺序和 artifact 恢复需求交接给 09。

## 关闭条件

- [x] Character、Canvas、Galgame 纳入范围的每条完整业务链均有唯一 Orchestrator owner（源码 owner 审计通过）。
- [x] Runtime 不再持有可独立完成的多阶段 LLM、图 runner、pipeline 或资产绑定业务流程（旧 owner/透传壳静态扫描通过）。
- [x] workflow/checkpoint/capability 调用、DB 写入和失败语义集中在 owner；Host 只保留合法 IO/DTO/权限边界（目标 crate check 通过）。
- [x] 跨节点/跨资产引用、取消、恢复、重复请求和 provider 失败已有显式测试；Character/Canvas/Galgame 本地 PostgreSQL 定向 runner 已进入断言并通过。
- [x] 未引入第二套 prompt、RAG、media、event 或 cleanup 实现。

本包代码状态为“待合并”：本机 PostgreSQL 集成验证和 Server/Tauri 聚合 check 尚未形成通过证据。后者当前被工作包 02 的旧 `video_generation_actions::has_active_provider_job` 调用阻塞，不在本包跨域修复。

### 并行复核交接（2026-07-25）

- 本次未修改 Character、Canvas 或 Galgame owner 文件；静态 owner/结构结论沿用本记录。
- 本机 PostgreSQL 已可用，但 03 的定向 runner 尚未能启动：`a4n-orchestrator` 当前在 09 会话编辑的 `context/generation.rs` 因缺少 `GenerationError`、`TaskPermit`、`GenerationTaskPersistenceError` 等符号编译失败。
- 09 交接并恢复 workspace 编译后，再集中运行 Character/Canvas/Galgame 的定向 PostgreSQL 场景；在此之前不把“数据库已启动”写成测试通过。

### PostgreSQL 定向复验（2026-07-25，取代上述编译阻塞结论）

- 使用本地 PostgreSQL 18.4（`127.0.0.1:55432`）和 `--no-default-features --test-threads=1` 串行执行 Orchestrator 定向 runner。
- Character：`character::` 79/79 通过。
- Canvas：`canvas::` 47/47 通过。
- Galgame：`galgame::` 105/105 通过。
- 09 generation owner 的旧编译阻塞已解除；本轮没有修改 Character、Canvas、Galgame 或 09 的业务源码。

## 最小验证

```bash
cargo fmt --all -- --check
git diff --check
timeout 60 cargo check -p a4n-orchestrator
timeout 60 cargo check -p a4n-runtime --no-default-features
timeout 60 cargo test -p a4n-orchestrator --lib character::
timeout 60 cargo test -p a4n-orchestrator --lib canvas::
timeout 60 cargo test -p a4n-orchestrator --lib galgame::
./scripts/checks/check-task-kind-boundary.sh
```

## 交接要求

分别报告 Character、Canvas、Galgame 三个子域的 owner、未迁入口和 Host 保留理由；不要用一个“Canvas 已完成”或“Character 已完成”的笼统结论覆盖仍未迁的子流程。
