# 工作包 07：Sidecar 独立进程矩阵与 PostgreSQL 证据

## 任务卡

| 项目 | 内容 |
|---|---|
| 主目标 | 补齐八个真实 sidecar 的错误/取消/恢复/版本/脱敏矩阵，并复核 PostgreSQL CI 证据 |
| V2 来源 | 3.13、阶段 D；3.14 与阶段 E 的状态一致性复核 |
| 当前状态 | 八项独立进程矩阵已在本机收口；PostgreSQL job 原先只跑 `a4n-db --lib`，遗漏独立 integration test，现已接入，仍待当前合并结果的远端 runner 证据 |
| 可并行性 | 可按 sidecar 分 agent；共享 process support 和 CI workflow 由一个集成者合并 |

## 不要重做的基础

- 复用 `a4n-test-support` 的空闲端口、启动、health、日志、停止和端口释放工具。
- 复用本地 fake provider；禁止调用付费 LLM/image/video 服务。
- 复用现有 CI process matrix 和 service catalog，不再创建第二份 binary/port 清单。
- 3.14 当前已标记 `完成`；阶段 E 的场景范围更广，必须逐项审计，不能仅按 3.14 状态直接关闭。

## 通用 process matrix

每个 sidecar 必须真实启动 binary，并覆盖：

```text
health / service identity / protocol / capability version
正常 typed request
非法 JSON / 非法字段 / capability mismatch / protocol mismatch
取消或 client disconnect
错误终态
错误后的下一次成功请求
bootstrap token 无凭证 / 错误凭证 / 正确凭证 / 脱敏
显式停止、graceful drain 和端口释放
```

## 能力专项

| Sidecar | 必补专项 |
|---|---|
| prompt | render、render_ab、override、version、validate、dependency graph、错误后恢复 |
| LLM | 保持现有 chat/stream/usage/finish/cancel/probe/models，补任何缺失回归 |
| embedding | model list、download、cancel、delete、embed、下载中断和下次恢复 |
| RAG | ingest/search/delete/cancel、临时索引、错误后恢复；解决冷编译预算而不是跳过 runner |
| image | preview、probe、generate events、provider failure、断连释放 |
| video | preview、submit、poll、bytes/job-id 两类完成结果、provider failure |
| import | ZIP、manifest、media payload、恶意路径拒绝、部分失败恢复 |
| export | JSON、ZIP、顺序稳定、错误脱敏、错误后恢复 |

## PostgreSQL 复核

1. 核对 CI 的 PostgreSQL 16 service、migration、`A4N_TEST_DATABASE_URL`/`DATABASE_URL` 和失败即失败语义。
2. 核对 DB/orchestrator/runtime 是否实际进入测试 runner，而不只是完成编译。
3. 已有事务、幂等、cleanup outbox lease/retry/fencing 和并发 claim 证据不重写；只补确实缺失的场景。
4. 3.14 的 CI 主体与阶段 E 的扩展失败组合分别核对；状态事实交给工作包 11，缺失测试仍回到对应 owner，不为状态文字另建测试框架。

## 并行分工建议

```text
Agent P1：prompt + LLM
Agent P2：embedding + RAG
Agent P3：image + video
Agent P4：import + export
集成者：共享 test-support、CI matrix、PG 复核、最终串行实跑
```

同一时间只允许集成者修改 `.github/workflows/*` 和共享 process helper。

## 关闭条件

- 八项 binary process test 都实际进入 runner并成功；没有冷编译超时被当作通过。
- 每项覆盖正常、非法、版本错误、取消、错误终态、错误后恢复、认证/脱敏、停止/端口释放。
- provider 型 sidecar 全部使用本地 fake provider。
- CI 独立执行八项，不允许 `|| true`、条件跳过或静默 fallback。
- PostgreSQL 状态与真实 CI 证据一致；如已完成，只做文档状态纠偏。

## 最小验证

```bash
timeout 60 cargo test -p a4n-service-protocol
timeout 60 cargo test -p a4n-capabilities
timeout 60 cargo test -p a4n-service-manager
timeout 60 cargo test -p <sidecar-crate> --test process -- --nocapture --test-threads=1
./scripts/checks/check-sidecar-package-inventory.sh
git diff --check
```

RAG/Embedding 需先使用项目已有 ONNX Runtime 配置脚本；若 60 秒不足，报告冷编译耗时并在合适 runner 延长预算，不能把“代码已接入”写成 process test 通过。

## 交接要求

提交八项矩阵表，逐格标明 `通过 / 失败 / 未执行 / 不适用`，附命令和 runner。只有源码存在但未进入 runner的格子必须标为 `未执行`。

## 执行交接（2026-07-25）

工作包：07 Sidecar 独立进程矩阵与 PostgreSQL 证据

状态：`进行中（已交接，待外部证据）`

### 基线与边界

- 基线为共享工作区 2026-07-25 快照；本会话只领取 07，未修改并行会话 06、09 的代码或状态。
- 复用 `a4n-test-support::process`、service catalog、现有本地 fake provider、ONNX Runtime 配置脚本和既有 PostgreSQL job；没有建立第二套启动器、端口表、provider mock 框架或数据库门禁。
- 未修改 V2 主文档和进度文档。3.14 已写为完成而阶段 E 仍写未完成的状态漂移，继续交由工作包 11 统一纠偏。
- PostgreSQL job 本会话只做证据复核，没有本地重跑 PostgreSQL 16 容器；远端 GitHub Actions 仍需在合并后给出最终 runner 结果。

### 修改路径

- 公共进程工具：`crates/a4n-test-support/{Cargo.toml,src/lib.rs,src/process.rs}`。
- 八项 process 入口：
  - `crates/a4n-prompt-service/tests/process.rs`
  - `crates/a4n-llm-gateway/tests/process/{requests.rs,cancellation.rs,support.rs}`
  - `crates/a4n-embedding-service/tests/process.rs`
  - `crates/a4n-rag-service/tests/process.rs`
  - `crates/a4n-image-service/tests/process.rs`
  - `crates/a4n-video-service/tests/process.rs`
  - `crates/a4n-import-service/tests/process.rs`
  - `crates/a4n-export-service/tests/process.rs`
- 对应 service 的 process dev-dependency、认证/body limit、事件取消和恢复前置改动继续沿用本包既有实现。
- 生产修复：`crates/a4n-video-service/src/providers.rs` 在 `video_job_output` 单一出口脱敏 provider failed message，避免 poll 成功响应 payload 泄漏 token/base URL。
- CI：`.github/workflows/module-checks.yml` 的八项 matrix 保持唯一入口；Embedding 不再用 `--no-default-features` 跳过 FastEmbed，而是安装固定 ONNX Runtime 1.24.2 后以 900 秒预算运行完整 process test。

### Owner 结论

- 没有迁移业务 owner，也没有新增兼容 wrapper。
- 进程启动、health、认证、停止和端口释放仍由 `a4n-test-support::process` 唯一复用入口负责。
- Video provider failed payload 的脱敏规则收敛到 `video_job_output`，所有 poll 输出共用同一规则，没有在测试或调用方复制过滤逻辑。

### 八项独立进程矩阵

本机 runner：`/home/fufu/Code/A4N`，2026-07-25；所有测试真实启动 `CARGO_BIN_EXE_*` binary，并通过 typed capability client 请求。

| Sidecar | health / identity / version | 正常与能力专项 | 非法 / 版本 / 认证 | 取消或断连 | 错误终态与下一次恢复 | 停止 / 端口 | 结果 |
|---|---|---|---|---|---|---|---|
| Prompt | 通过 | 通过：list、render、render_ab、validate、update、version/history/diff、override、dependency graph、Git backup/restore | 通过：malformed、oversized、unknown field、capability/protocol mismatch、missing/wrong/correct token、响应脱敏 | 通过：partial request body disconnect | 通过：missing prompt、非法/损坏 manifest 后恢复 | 通过：graceful stop、force stop、端口释放 | 通过 |
| LLM | 通过 | 通过：chat、stream、usage、finish reason、probe、models | 通过：malformed、oversized、unknown field、capability/protocol mismatch、missing/wrong/correct token、provider secret 脱敏 | 通过：关闭 event receiver 取消 stream 并释放 provider body | 通过：本地 provider 500 产生唯一 Failed terminal，下一次 chat 成功 | 通过 | 通过 |
| Embedding | 通过 | 通过：local hash embed、FastEmbed model list、download progress、delete、完整/不完整 cache 识别 | 通过：malformed、oversized、unknown field、capability/protocol mismatch、missing/wrong/correct token | 通过：partial body disconnect；关闭 receiver 取消 FastEmbed download | 通过：unsupported backend typed error 后 local hash 成功；下载中断、force stop、manifest 错误后恢复 | 通过 | 通过（无特性与完整 FastEmbed 两个 runner） |
| RAG | 通过 | 通过：ingest、search、query、delete、drop/rebuild、completeness、临时索引重启 | 通过：malformed、oversized、unknown field、capability/protocol mismatch、missing/wrong/correct token | 通过：关闭 ingest event receiver，客户端得到 Cancelled，双索引原子完成 | 通过：错误维度 query 返回 ProviderError，下一次合法 query 成功；mutation marker/manifest 后显式恢复 | 通过：含 force stop | 通过 |
| Image | 通过 | 通过：preview、probe、generate、generate events | 通过：malformed、oversized、unknown field、capability/protocol mismatch、missing/wrong/correct token | 通过：关闭 receiver 取消 hanging provider，并确认 response body 释放 | 通过：不可达本地 provider 产生 Failed terminal，错误脱敏，下一次 fake provider 成功 | 通过 | 通过 |
| Video | 通过 | 通过：preview、probe、submit、generate events、bytes、job-id、poll pending/failed/completed | 通过：malformed、oversized、unknown field、capability/protocol mismatch、missing/wrong/correct token | 通过：关闭 receiver 取消 hanging provider，并确认 response body 释放 | 通过：provider Failed terminal 后成功；poll failed payload token/base URL 脱敏 | 通过 | 通过 |
| Import | 通过 | 通过：normalize JSON、真实 ZIP、manifest、media payload | 通过：malformed、oversized、unknown field、capability/protocol mismatch、missing/wrong/correct token | 通过：ZIP partial request body disconnect | 通过：invalid base64、恶意 `../` manifest path 拒绝，随后正常 ZIP 成功；错误脱敏 | 通过 | 通过 |
| Export | 通过 | 通过：JSON、真实 ZIP、ZIP magic、两次输出稳定 | 通过：malformed、oversized、unknown field、capability/protocol mismatch、missing/wrong/correct token | 通过：ZIP partial request body disconnect | 通过：invalid base64 typed error 后正常 ZIP 成功；错误脱敏 | 通过 | 通过 |

说明：Embedding 的 download 项由真实 `/embedding/model/download/events` 进入下载流程并产生 progress，随后验证 receiver cancellation 和下次恢复；测试未调用付费 provider。Image、Video、LLM 全部使用本地 fake provider。

### 实际验证命令

以下命令均实际执行并通过，没有把冷编译超时记为通过：

```bash
timeout 300 cargo test --quiet -p a4n-prompt-service --test process -- --nocapture --test-threads=1
timeout 300 cargo test --quiet -p a4n-llm-gateway --test process -- --nocapture --test-threads=1
timeout 300 cargo test --quiet -p a4n-image-service --test process -- --nocapture --test-threads=1
timeout 300 cargo test --quiet -p a4n-video-service --test process -- --nocapture --test-threads=1
timeout 300 cargo test --quiet -p a4n-import-service --test process -- --nocapture --test-threads=1
timeout 300 cargo test --quiet -p a4n-export-service --test process -- --nocapture --test-threads=1
timeout 300 cargo test --quiet -p a4n-embedding-service --no-default-features --test process -- --nocapture --test-threads=1
source scripts/lib/common.sh
configure_onnxruntime
timeout 600 cargo test --quiet -p a4n-embedding-service --test process -- --nocapture --test-threads=1
timeout 600 cargo test --quiet -p a4n-rag-service --test process -- --nocapture --test-threads=1
timeout 60 cargo test --quiet -p a4n-service-protocol
timeout 60 cargo test --quiet -p a4n-capabilities
timeout 60 cargo test --quiet -p a4n-service-manager
timeout 60 ./scripts/checks/check-sidecar-package-inventory.sh
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/module-checks.yml")'
bash -n scripts/dev/setup-onnxruntime.sh scripts/lib/common.sh
git diff --check
```

结果：八项 process test 全部通过；`a4n-service-protocol` 42 tests、`a4n-capabilities` 20 tests、`a4n-service-manager` 16 tests 全部通过；sidecar inventory 为 8 项；workflow YAML、shell syntax 和 diff whitespace 均通过。

### PostgreSQL 证据复核

| 证据 | 状态 | 复核结果 |
|---|---|---|
| PostgreSQL 16 service | 通过 | `postgres:16`，带 `pg_isready` health check 和固定测试数据库 |
| 数据库环境变量 | 通过 | 同时设置 `A4N_TEST_DATABASE_URL` 与 `DATABASE_URL` |
| migration 先行 | 通过 | `timeout 300 cargo run -p a4n-migration -- "$DATABASE_URL"` 在三个 test runner 前执行 |
| DB runner | 通过 | `timeout 900 cargo test -p a4n-db --lib --tests -- --nocapture`，同时执行 lib tests 与 `tests/generation_persistence.rs` |
| Orchestrator runner | 通过 | `timeout 900 cargo test -p a4n-orchestrator --lib --no-default-features -- --nocapture` |
| Runtime runner | 通过 | `timeout 900 cargo test -p a4n-runtime --lib --no-default-features -- --nocapture` |
| cleanup outbox | 通过（源码/CI 证据） | 包含 dedupe 幂等、过期 lease fencing/reclaim、retry、并发 claim 单 owner、非法 payload 隔离 |
| 失败即失败 | 通过 | job 没有 `|| true`、条件跳过或错误转成功 fallback |
| 本机会话 PostgreSQL 实跑 | 通过 | PostgreSQL 18.4；`generation_persistence` 两项独立 integration tests 通过，测试 helper 使用进程级独立 schema |

### 未关闭条件与依赖

- 本地代码和 process runner 已收口；状态保持 `待合并`，直到远端 `sidecar-process` 八项 matrix（含完整 FastEmbed）与 `postgres-integration` job 在当前合并结果上实际通过。
- 阶段 E 还包含 DB/文件失败组合、worker 重启恢复等比 3.14 更宽的场景；这些由工作包 09 和对应 owner 收口，本包不越权修改最终状态来源。
- 与工作包 08 的边界：本包验证现有 bootstrap token、版本和脱敏行为，不扩展限流、mTLS 或资源策略。
- 与工作包 09 的边界：本包只验证 Image/Video provider body 释放和 sidecar 事件终态，不修改跨事务媒体 cleanup/outbox owner。

### 补充交接：独立 PostgreSQL 测试进入 CI（2026-07-25）

- 根因：`postgres-integration` 使用 `cargo test -p a4n-db --lib`，Cargo 不会执行 `crates/a4n-db/tests/generation_persistence.rs`，因此 reservation/checkpoint/lifecycle gate/TTL 的真实 PostgreSQL 测试只有本地证据，没有进入 CI runner。
- 修复：`.github/workflows/module-checks.yml` 改为 `cargo test -p a4n-db --lib --tests`；未修改 09 的 generation 实现或测试源码。
- 本机验证：`timeout 60 env A4N_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:55432/postgres cargo test -p a4n-db --test generation_persistence -- --nocapture --test-threads=1`，2/2 通过；workflow YAML 和 `git diff --check` 通过。
- 仍缺：远端 `postgres-integration` 与八项 `sidecar-process` 在当前合并结果上的成功记录。

### 远端 runner 事实复核（2026-07-25）

- 本文件前述八项 sidecar process runner 均已在本机真实启动 binary 并通过；`generation_persistence` 两项测试也已在本地 PostgreSQL 18.4 通过。这里不再重复执行整套矩阵。
- `origin/master=9deb6ca453524456003e6c42264f2bc583cb158b` 是本地 `HEAD=30adf2c15f28b63a8fb67c24163c14ff4762c58f` 的祖先，当前 `.github/workflows/module-checks.yml` 尚未进入远端。
- GitHub 公共 Actions API 返回仓库 workflow 数为 0，因此当前没有 `sidecar-process`、`postgres-integration` 或 FastEmbed setup 的远端 run/artifact；“待外部证据”只指远端当前合并结果，不再表示八项本地 process 尚未执行。

结论：07 的本地实现与本地 runner 证据已经收口；远端 CI 证据必须在三会话合并、提交并推送后产生。阶段 E 中更宽的 DB/文件双故障和 worker 重启仍由 09 收口，本包不复制测试框架或提前修改 V2 状态。
