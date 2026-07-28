# 工作包 02：A3 Video Generation 剩余完整执行链

## 任务卡

| 项目 | 内容 |
|---|---|
| 主目标 | 收完 video generation 仍留在 Runtime 的完整编排、状态和 provider 请求边界 |
| V2 来源 | 3.10、阶段 A；进度看板 `video_generation`（约 3,610 行 / 55 文件的历史体量，仅用于排序） |
| 当前状态 | 已收口；Orchestrator `video::` 357/357、Runtime `video_generation` 38/38 与工作包 09 证据已由批次 303 纳入 V2 3.10/阶段 A 关闭 |
| 可并行性 | 可与 01、03–05 并行；共享文件生命周期和事件桥交给 09 |

## 当前事实

- 已存在 `chapter_video_generation`、`reference_media_generation`、Canvas video 和 `video::prompt_application` 等 owner；禁止重新建第二套 request builder、prompt renderer、workflow 或 provider facade。
- Runtime 合法保留 runtime profile、队列、provider/sidecar transport、图片 transport、文件 IO、进度/chunk/cancel 和 DTO 投影。
- 仍需逐入口确认：generate/job submit、poll/wait、preview、interrupted/error 终态、capacity 中断、workflow progress/state、provider 请求元数据和生成记录的最终写入是否全部由 application 组合。

## 范围

1. 以 `video_generation` 的 public Host API 为入口，画出 `request → workflow/checkpoint → provider → generation record → terminal state` 链路。
2. 将剩余业务判断、状态推进、workflow/checkpoint、capacity 语义和 DB 写入收口到已有 Orchestrator video application/context。
3. 保持 `RuntimeVideoProviderHostPort` 等窄 Host port，仅让其执行 provider/队列/文件/sidecar IO，不隐藏 DB 业务写入。
4. 统一 preview 与真实 submit 的合同、identity、错误脱敏和取消语义；删除旧的 fallback/兼容分支。
5. 审计 `video_records`、`video_prompts`、Chapter/Reference/Canvas application 的调用接线，发现回流时在本包修复，不能复制 owner。

## 不在范围

- 不重做批次 205–213 已完成的 Chapter Video、Reference-media、Canvas Video、Video Records、Video Prompts 和 Chapter Generation application。
- 不建立新的文件删除、outbox、TTL、重启恢复或远程 artifact 协议；交给 09。
- 不修改 sidecar 的认证、并发、body limit、版本协商和 process matrix；交给 07/08。
- 不以清零 `AppState` 引用为目标；合法 Host 边界保留。

## 建议切片

1. `generate / job submit / creation`：确认生成记录和 workflow begin 的唯一 owner。
2. `poll / wait / complete / error / interrupted`：确认终态、capacity 中断和幂等更新的唯一 owner。
3. `preview / request assembly`：与真实 submit 共用合同和 provider request 规则。
4. `provider/file boundary audit`：只留下 Host port；把持久文件补偿交给 09。

每个切片完成后都删除无消费者 facade 和重复测试，先跑定向测试再合并。

## 主要路径（先核对）

```text
crates/a4n-runtime/src/actions/video_generation/**
crates/a4n-runtime/src/actions/video_prompts/**      # 只审计回流，不重做已收口 owner
crates/a4n-orchestrator/src/video/**
crates/a4n-orchestrator/src/context/video.rs
crates/a4n-runtime/src/state/video/**
crates/a4n-runtime/src/host_api/video* / crates/a4n-runtime/src/host_api/canvas*
```

## 关闭条件

- video generation 的每条公开 Host API 都有唯一 application owner。
- Runtime 不再直接拥有 provider 请求前的业务决策、workflow/checkpoint、生成记录写入或终态推进。
- Provider、队列、文件和进度只通过窄 Host port；不存在第二套 provider/request/状态机。
- preview 与 submit 使用同一 identity、配置和错误合同；取消、失败、重试和重复请求显式可测。
- 旧 owner、无消费者 facade、重复规则和 fallback 已删除，并有结构门禁防回流。

## 最小验证

```bash
cargo fmt --all -- --check
git diff --check
timeout 60 cargo check -p a4n-orchestrator
timeout 60 cargo check -p a4n-runtime --no-default-features
timeout 60 cargo test -p a4n-orchestrator --lib video::
timeout 60 cargo test -p a4n-runtime --no-default-features video_generation
./scripts/checks/check-task-contract-owner.sh
```

需要 PostgreSQL、真实 sidecar 或 provider 的场景必须标明环境；本地 fake provider 不能替代持久化失败和跨重启证据。

## 交接要求

提交 provider/文件副作用边界表给工作包 09；提交 event/terminal 顺序需求给工作包 09；列出仍属于合法 Host 的方法和理由。不要直接改全局进度文档的 V2 数字。

## 交接记录（2026-07-24）

```text
工作包：02-A3-video-generation
状态：`进行中（已交接，待 09/协调者）`
基线：Runtime Video Generation 已有 Chapter、Reference-media、Canvas 和 provider application，但尾帧采集仍在 Runtime 构造并写入 generated_image；另有无消费者的 common/lifecycle.rs 转发 facade。
修改路径：crates/a4n-orchestrator/src/video/**；crates/a4n-runtime/src/actions/video_generation/**；crates/a4n-runtime/src/state/video.rs；crates/a4n-runtime/src/state/convergence_tests.rs。
新 owner 与删除 owner：Orchestrator 的 chapter_video_generation、reference_media_generation、canvas::video_generation、provider_execution 和 video::records 成为唯一 application owner；删除 Runtime 旧 execution/request/result/workflow owner 与 common/lifecycle.rs。没有保留兼容 wrapper 或第二套规则。
未关闭条件：实现关闭条件已满足；共享 Cargo artifact lock 下的聚合测试未进入 runner，需协调合并前补跑。真实 PostgreSQL、sidecar/provider 和跨重启文件补偿证据属于工作包 07/09。
与其他工作包的冲突/依赖：文件 cleanup intent、跨重启补偿和 Host event 顺序交工作包 09；sidecar process/security 交工作包 07/08；不直接修改 V2 全局数字。
```

### 实际收口结果

- Chapter 单条、批量和 preview 统一委托 `video::chapter_video_generation`。
- Reference-media generate 和 preview 统一委托 `video::reference_media_generation`。
- Canvas Video 委托 `canvas::video_generation`；provider 完成、失败、中断和恢复轮询统一由 `video::provider_execution` 推进。
- 尾帧文件仍由 Runtime/FFmpeg 提取，但 `generated_image` 字段、归属、provenance 和写入已迁到 `video::records::create_video_tail_frame_record_validated`。
- Runtime Video Generation 静态扫描中不存在直接 `create_generated_image_record`、`create_video_record`、`mark_generated_video_*`、`update_generated_video_*`、`state.db` 或完整 `orchestrator()` 回流。
- 删除无消费者的 `common/lifecycle.rs`；使用方直接复用 canonical `video::has_active_provider_job`。
- `state::convergence_tests::video_generation_runtime_uses_application_owner` 固化 application 入口、Runtime 禁止调用、Host IO 边界和旧 owner 不得恢复。

### 删除的旧 owner / facade

以下 Runtime 路径不得恢复：

```text
chapter/execution.rs
reference_media/execution.rs
canvas/execution.rs
job_lifecycle/workflow.rs
job_lifecycle/polling/wait.rs
provider_request/chapter_builder.rs
provider_request/preview.rs
results.rs
common/lifecycle.rs
```

### 提交给工作包 09 的副作用边界

| 副作用 | 当前 owner | 合法 Host 理由 |
|---|---|---|
| provider submit / poll | `RuntimeVideoProviderHostPort` | 外部 provider、sidecar 和持久队列 IO |
| 视频、缩略图、尾帧文件写入 | Runtime media / FFmpeg | 物理文件系统、FFmpeg 和二进制 transport |
| 尾帧 `generated_image` 记录 | Orchestrator `video::records` | 业务归属、记录字段和 provenance 必须单点生成 |
| `generated_video` 状态、generation run、provider workflow | Orchestrator `video::provider_execution` | 唯一业务状态机和终态 owner |
| cleanup intent | Runtime Host / cleanup outbox | 文件补偿基础设施；application 只请求清理，不直接删除文件 |

工作包 09 应保持以下顺序：

1. provider 返回 job id 后，application 先登记 video job/prompt 关联，再把 provider workflow 标记为可恢复的 `running`。
2. `pending/running` 轮询先更新 `generated_video` waiting 状态，再记录 workflow `running` 进度。
3. 完成时 Host 先持久化文件；application 再提交 `generated_video` 完成状态和 prompt 关联、完成 generation run，最后提交 provider workflow 终态。业务提交失败时由 Host 写 cleanup intent。
4. 失败或中断先提交 `generated_video` 终态，再提交 workflow 终态；中断保留 `provider_job_id` 且 `can_resume=true`。
5. Canvas 恢复轮询在 video 终态后同步父 workflow checkpoint；cleanup 或二次错误必须追加到主错误，不能吞掉。

### 验证证据

| 命令 | 结果 |
|---|---|
| `cargo check -p a4n-orchestrator --no-default-features` | 通过，退出码 0；仅有 Optimize 既有 dead-code warning |
| `cargo check -p a4n-runtime --no-default-features --keep-going` | 通过，退出码 0；仅有其他领域既有 dead-code warning |
| `rustfmt --edition 2024 --check <本工作包触碰的 5 个 Rust 文件>` | 通过，退出码 0 |
| `cargo fmt --all -- --check` | 未通过；共享工作区中 `admin`、`character`、`galgame`、`optimize` 等其他会话文件尚未格式化，本工作包触碰文件已单独通过 |
| `git diff --check` | 通过，退出码 0 |
| `bash scripts/checks/check-task-kind-boundary.sh` | 通过：`canonical TaskKind boundary is valid` |
| `bash scripts/checks/refactor-metrics.sh` | 通过：本地纯 rules `0/0`、Orchestrator 透传壳 `0/0`、actions `.orchestrator()` `0/0`；Runtime actions `539` 文件 / `28,774` 行 |
| 两组 Runtime Video Generation owner 静态扫描 | 通过，无输出 |
| `cargo test -p a4n-runtime --no-default-features state::convergence_tests::video_generation_runtime_uses_application_owner -- --exact --nocapture` | 两次均在 60 秒超时；一次仍在依赖编译，一次先等待共享 artifact lock 后编译 Orchestrator，均未进入测试 runner，不能记为通过 |
| `cargo test -p a4n-orchestrator --lib --no-default-features video::`、`cargo test -p a4n-runtime --no-default-features video_generation` | 未执行；共享工作区已有另一会话占用 Cargo artifact lock，协调合并前集中补跑 |
| `scripts/checks/check-task-contract-owner.sh` | 仓库中不存在；实际可用且已通过的脚本是 `check-task-kind-boundary.sh` |

不需要 PostgreSQL 或真实 provider 才能判断本包的 owner 收口；这些环境只用于工作包 07/09 的持久化、进程和跨重启证据。

### 并行复核交接（2026-07-25）

- 当前 Server/Tauri 对 `video_generation_actions::has_active_provider_job` 的调用通过 Runtime 的单一 `pub use a4n_orchestrator::video::has_active_provider_job` canonical re-export；没有第二份 job-status 规则。此前进度记录中“调用已迁走、尚未恢复编译”的描述已过时，不再作为当前 blocker。
- 本次未修改 Video owner、provider Host port 或 09 生命周期代码；当前聚合 runner 仍会受 09 `context/generation.rs` 编译状态影响，待其交接后集中重跑 `video::` 与 `video_generation` 定向测试。
- 真实 PostgreSQL、sidecar/provider 和跨重启文件补偿继续由 07/09 提供证据，本包不将本地静态扫描替代这些证据。

### 聚合 runner 复验（2026-07-25，当前事实）

- 使用本地 PostgreSQL 18.4（`127.0.0.1:55432`）串行执行：
  - `cargo test -p a4n-orchestrator --lib --no-default-features 'video::' -- --test-threads=1`：`352 passed / 5 failed`（357 个测试）。
  - `cargo test -p a4n-runtime --no-default-features 'video_generation' -- --test-threads=1`：`38 passed / 0 failed`。
- 5 个失败均已定位，未修改 02 owner 或 09 活跃路径：
  - `video_prompt_user_includes_action_load_and_temporal_capacity_rules`：测试夹具传入空 `segments`，而当前 builder 明确拒绝不存在的 `segment_number=1`。
  - `missing_chapter_prompt_queries_return_not_found_without_touching_orphans`、`delete_missing_video_returns_not_found_without_touching_prompt_reference`、`delete_missing_video_references_clears_stale_prompt_lock`：夹具直接插入不存在外键的 `chapter_video_prompt`，在进入目标断言前被 PostgreSQL FK 拒绝。
  - `delete_prompt_rejects_segment_plan_when_frame_prompts_exist`：当前返回的 stage 标签为“视频片段规划”，用例仍断言旧的“首帧串联片段规划”文案；删除守卫本身没有吞错。
- 这些是既有测试夹具/文案与当前合同或数据库约束的偏差，不通过放宽 owner 逻辑或添加 fallback 迁就；待协调者决定是否统一修订历史测试后再纳入最终门禁。

### 历史测试夹具收口（2026-07-25，取代上述 5 个失败）

- 只修正四个测试文件，没有修改 Video owner、DB 生产规则、Runtime Host 边界或 09 生命周期路径：
  - final video prompt 夹具补入与 `current_segment` 对应的 `segment_number=1`。
  - 三个 PostgreSQL 夹具先创建真实 project/chapter，再验证缺失 chapter/video 引用语义，避免通过非法外键制造 orphan。
  - 删除守卫断言改为当前 canonical stage 标签“视频片段规划”。
- 定向 `rustfmt --check` 与 `git diff --check` 通过。
- 本地 PostgreSQL 18.4 上重跑 Orchestrator `video::`：`357/357 passed`；Runtime `video_generation` 既有结果仍为 `38/38 passed`。
