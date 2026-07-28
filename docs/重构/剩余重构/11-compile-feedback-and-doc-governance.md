# 工作包 11：编译反馈基线与重构文档治理

## 任务卡

| 项目 | 内容 |
|---|---|
| 主目标 | 将编译反馈收益量化并设回归门禁，同时让重构状态文档进入可审查版本历史 |
| V2 来源 | 3.23、3.27；阶段 L、阶段 M 的 timing/文档部分 |
| 当前状态 | `完成（批次 336）`：3.27 已有 commit/blame；run `30368914502` 建立 fixed-runner baseline，run `30371035840` 在同一 `25%` budget 下复验通过 |
| 可并行性 | timing、doc checks、状态一致性可并行；最终阈值和 V2 状态回写由协调者确认 |

## 子包 A：编译反馈门禁（3.23）

1. 固定 CI runner、toolchain、CPU/cache 说明，分开记录 cold workspace check 和 warm package check。
2. 为 prompt、LLM、embedding、RAG、image、video、Server、Tauri 建立至少三次测量和中位数。
3. 保存 `cargo build --timings` 产物、直接依赖清单和命令；禁止混用冷/热结果。
4. 建立局部 package warm check 回归阈值和 `check-build-feedback-budget.sh`；超阈值 CI 失败。
5. 保留 minimal host 重依赖扫描硬门禁；不得用缩短 timing 的方式把重依赖偷偷移回 Host。

## 子包 B：文档进入版本控制（3.27，已完成）

1. 精确放行当前计划、进度记录和本工作包目录（是否修改 `.gitignore` 由协调者决定）。
2. 将计划和进度记录纳入实际 Git 提交历史；仅出现在工作区或 `git status` 不算完成。
3. CI 检查计划存在、状态值只能是 `完成`/`未完成`、架构边界变化与文档同 PR 更新。
4. 历史计划不得重新成为当前状态源；删除/归档的旧文档引用不能出现在活动链接中。
5. 每个 agent 只提交自己的工作包证据；V2 3.x 数字和最终状态由协调者统一回写。

## 子包 C：状态一致性审计

当前 V2 存在“3.11/3.14 已完成，但阶段 B/E 仍写未完成”等重叠口径风险。先做证据审计：

1. 逐项对照 3.x 完成定义和阶段 A–M，不重复实施已完成的 3.11、3.14、3.18、3.22、3.28。
2. 对阶段 B/E 的状态只在关闭条件确实满足后修正文档；如果缺真实证据，列出具体缺口。
3. 维护“定义条目”和“执行阶段”的映射表，避免 3.10/阶段 A、3.13/阶段 D 等重复计数。
4. 只有所有 3.x 与阶段关闭条件均满足，才允许把 3.29 改为 `完成`。

## 不在范围

- 不为了 timing 改业务代码或增加缓存 fallback。
- 不将未提交文档写成已进入版本控制；必须有 commit/blame 证据。
- 不代替 06 实现 core public-type inventory；本包只接收其结果并检查文档/CI 门禁。
- 不在本包顺手清理所有 Markdown 历史内容。

## 关闭条件

- cold/warm timing、三次中位数、timings artifact、依赖清单和回归阈值进入固定 CI。
- minimal host 重依赖门禁继续通过，超阈值能让 CI 失败。
- 当前状态文档和工作包文档出现在 Git 提交历史，CI 能阻止状态值/计划漂移。
- 3.x 与阶段表的重叠/不一致已得到证据化裁决，3.29 仍由全局门禁决定。

## 最小验证

```bash
cargo check --workspace --timings
cargo check -p a4n-prompt-service --timings
cargo check -p a4n-llm-gateway --timings
cargo check -p a4n-rag-service --timings
cargo check -p a4n-image-service --timings
cargo check -p a4n-video-service --timings
./scripts/checks/check-build-feedback-budget.sh
./scripts/checks/check-refactor-plan-status.sh
./scripts/checks/check-refactor-docs.sh
git check-ignore docs/重构/模块化重构执行计划-v2.md docs/重构/模块化重构进度记录.md
```

最后一条命令当前应返回“未被忽略”；tracking、`HEAD:<file>` 和状态一致性由上述 checker 继续 fail closed。

## 交接要求

提交 timing 原始 artifact、阈值选择、runner/toolchain、状态映射表和 Git 提交证据。不要自行把其他工作包的未完成项改成完成。

## 2026-07-25 本会话执行记录

状态保持：`进行中（已交接，待外部证据）`。本轮完成可在本地落地的门禁，不把尚未进入 GitHub runner 或提交历史的部分写成完成。

### 已落地

- 新增 `scripts/checks/measure-build-feedback.sh`：一次执行 workspace cold check，并在每个目标 package 的三次计时前执行显式 warmup，保存中位数、命令日志、Cargo timings、runner/toolchain 元数据和直接依赖树；少于三次测量会直接失败。
- 新增 `scripts/checks/build-feedback-baseline.tsv` 与 `check-build-feedback-budget.sh`：缺 package、重复行、非正耗时、缺少 workspace cold 样本、额外测量或超预算均显式失败；不使用静默 fallback。
- 扩展 `check-refactor-plan-status.sh`：逐项要求 V2 3.1–3.29 与阶段 A–M 各有且只有一个 `完成`/`未完成` 状态，并验证进度记录仍指向 V2。
- 收紧 `check-refactor-docs.sh`：计划、进度、工作包 README、全部工作包 Markdown/TSV 都必须存在、未被 ignore 且已由 `git ls-files` 跟踪；活动链接不得回指历史计划。
- `.gitignore` 精确放行当前计划、进度记录和 `docs/重构/剩余重构/` 工作包目录；没有放行整个 `docs/`，TSV inventory 也由 tracking 门禁单独检查。
- `.github/workflows/module-checks.yml` 新增固定 `ubuntu-24.04` / Rust `1.95.0` 的 `build-feedback` job，运行采集、预算、状态检查并上传 artifact。

### 本地 provisional package rebuild 数据

旧 warm 数据只是连续执行 no-op `cargo check`，没有重编译被测 package，已废弃。当前方法标记为
`package-rebuild-v1`：八个 package 共用一个隔离 warm target，每项先 warmup；每个计时样本前只执行
`cargo clean -p <package>`，保留依赖缓存并真实重编译该 package，且不把 clean 时间计入反馈时间。

当前机器为 13th Gen Intel Core i9-13900H、20 logical CPUs、Linux x86_64、Rust/Cargo 1.95.0。
本地使用 `--skip-workspace` 采集以下三次样本；因此没有伪造 `__workspace_cold__`，也不能直接通过
要求 cold 样本的 CI budget。

| package | 三次样本 | median |
|---|---:|---:|
| `a4n-prompt-service` | 568 / 561 / 542 ms | 561 ms |
| `a4n-llm-gateway` | 458 / 427 / 469 ms | 458 ms |
| `a4n-embedding-service` | 525 / 532 / 618 ms | 532 ms |
| `a4n-rag-service` | 619 / 600 / 617 ms | 617 ms |
| `a4n-image-service` | 464 / 493 / 468 ms | 468 ms |
| `a4n-video-service` | 547 / 527 / 485 ms | 527 ms |
| `a4n-server --no-default-features` | 5,044 / 5,134 / 5,022 ms | 5,044 ms |
| `a4n-tauri --no-default-features` | 23,637 / 22,478 / 22,615 ms | 22,615 ms |

批次 310 当时在 `target/build-feedback/runs/20260728T054052Z-747095/` 生成 metadata、hardware、
24 个 package reset log、33 份 timings 和八份直接依赖清单；该目录属于可清理的 ignored `target/`
证据，当前工作区已不再保留，不能作为现行关闭证据。上表样本只用于 provisional baseline；第一份
固定 `ubuntu-24.04` artifact 产生后必须替换。

### 未关闭条件

1. GitHub Actions run `30361691038` 的 build-feedback job 在 timing 前因 runner 缺少 `rg` 失败，504 字节 artifact 不含固定 runner 的 cold/warm 中位数、Cargo timings、hardware 或 dependency inventory。
2. 因此 3.23 与阶段 L 继续保持 `未完成`；3.27 已完成，阶段 M 的文档部分已关闭，但阶段 M 整体仍等待工作包 10 的 3.26 正式供应链/签名/Release 证据。当前汇总为 `36/44`。

### 本会话追加（2026-07-25）

- warm 测量现在要求至少三次，并在计时前执行显式 warmup；测量文件必须包含独立的 `__workspace_cold__` 样本。
- 预算门禁拒绝缺少 cold 样本、额外 package 行、重复/非正耗时和超预算结果。
- `measure-build-feedback.sh` 现在为 workspace cold 和 package warm 使用各自的全新隔离 `CARGO_TARGET_DIR`；warm target 在八个 package 间共享但先经过显式 warmup，避免复用会话外的脏 target。
- 每次 cold、warmup 和三次 measured run 都单独归档 Cargo timings、命令日志和原始毫秒样本；baseline 只接受八个 warm package，不会把 `__workspace_cold__` 混入 warm ceiling。
- 新增 `check-build-feedback-tools.sh` contract self-test，使用 fixture cargo 只验证脚本的样本/归档/预算契约，不把 fixture 数字当作性能证据；固定 runner job 已接入该自测。
- 计划状态门禁现在同时要求 V2 `3.1–3.29` 与阶段 `A–M` 各有且只有一个合法状态。
- 文档门禁覆盖工作包 README、全部 Markdown/TSV，并拒绝活动链接回指已归档计划。
- 以 `--locked` 运行最小 timing smoke 时发现并同步了当前 workspace 的 `Cargo.lock` 依赖条目；后续编译反馈命令不会用去掉 `--locked` 的隐式更新掩盖锁文件漂移。
- 本地脚本语法检查已通过；当前门禁仍会因文档尚未进入 Git tracking 而失败，这是真实未关闭条件，不写成通过。
- `check-build-feedback-tools.sh` fixture self-test 已通过：验证隔离 target 清理、cold + warmup + 三次原始样本、每次 timings 归档、warm baseline 排除 cold，以及额外 baseline/预算门禁拒绝。
- 真实本机最小 smoke（非固定基线）已通过：`a4n-service-protocol` 使用 fresh warm target，三次为 `250/309/256 ms`，中位数 `256 ms`，并生成独立 Cargo timing HTML 与依赖树；这不替代八 package 的固定 runner artifact。

### 定义条目与执行阶段映射审计

该映射用于防止把同一关闭条件重复计数；阶段状态不机械复制 3.x 状态，因为阶段还可能包含额外的 CI、跨进程或运行环境条件。

| 执行阶段 | 对应 V2 定义条目 |
|---|---|
| A | 3.10 |
| B | 3.11 |
| C | 3.12、3.24 的 Host/API 部分 |
| D | 3.13 |
| E | 3.14 |
| F | 3.15 |
| G | 3.16 |
| H | 3.17 |
| I | 3.18 |
| J | 3.19、3.20、3.28 的运行时 catalog/实例部分 |
| K | 3.21、3.22 |
| L | 3.23、3.24 的 Core/timing 部分 |
| M | 3.25、3.26、3.27 |

批次 305 已完成该裁决：阶段 B 依据 3.11 的私有化/结构门禁关闭；阶段 C 依据 3.12/3.24 的合同、生成、parity 和 core inventory 关闭；阶段 D 依据工作包 07 的八项真实 process 矩阵关闭；阶段 E 结合 3.14、工作包 07 的 PG runner 与工作包 09 的 DB/文件双故障、outbox 恢复证据关闭。3.28 已完成但属于跨阶段支撑项，不另计一条阶段关闭。

### 远端 Git/Actions 证据核对（2026-07-25）

- `origin` 指向公开仓库 `GTJasonMK/A4N`；`git ls-remote origin refs/heads/master` 返回 `9deb6ca453524456003e6c42264f2bc583cb158b`，该提交是本地 `HEAD=30adf2c15f28b63a8fb67c24163c14ff4762c58f` 的祖先。当前重构代码、workflow 和文档均不在远端 master。
- GitHub 公共 REST API 的 `/actions/workflows` 返回 `total_count=0`。因此当前不存在可以证明 `build-feedback`、PostgreSQL、sidecar、供应链或跨平台矩阵通过的远端 Actions run/artifact；不能把本地 workflow 定义写成远端证据。
- `scripts/checks/check-refactor-plan-status.sh` 的结构/status 检查通过，随后按设计因 `docs/重构/模块化重构执行计划-v2.md` 尚未被 Git 跟踪而失败；`scripts/checks/check-refactor-docs.sh` 同样在该 tracking 边界失败。
- `git status --porcelain=v1` 当前为 2,373 项并行修改/未跟踪文件，且三个会话仍在共享同一工作树。本轮不 stage、commit 或 push 其他会话成果，避免把移动中的跨包状态做成不一致提交。

结论保持不变：工作包 11 的本地门禁主体已落地，但 3.23/3.27、阶段 L/M 仍缺固定 runner artifact 与真实 commit/blame。应在各工作包交接、工作树收敛后由协调者做精确提交并推送，再以该提交的 Actions artifact 关闭，不新增 fake timing、临时 tracking fallback 或本地冒充远端的证据。

## 批次 305 · 合同、process 与阶段状态一致性裁决（2026-07-28）

状态：`完成`

- 当前复验：`a4n-api-contract` 编译、DTO inventory、API contract boundary、core public-type/module/dependency boundary、frontend API parity 全部通过；inventory 为 301 个 core public declarations，前端/Web/桌面缺失映射均为 0。
- 复核工作包 07 的八个真实 binary process 能力矩阵及唯一 CI matrix，并结合工作包 09 已关闭的 PostgreSQL DB/文件双故障、强杀恢复和 cleanup outbox 证据，不再把“尚无远端 run”混作 3.13/阶段 D/E 的源码缺口。
- 关闭 V2 3.12、3.13、3.24 与阶段 B/C/D/E；3.23、3.27、阶段 L/M 继续保持 `未完成`，总体由 `23/44` 更新为 `30/44`。

## 批次 310 · package rebuild timing 与 committed-history 门禁（2026-07-28）

状态：`未完成`

- `measure-build-feedback.sh` 改为 `package-rebuild-v1`：每个 measured sample 前在隔离 warm target 中执行 `cargo clean -p <package>`，依赖保持 warm，被测 package 真实重编译；旧连续 no-op 数据不再进入 baseline。
- baseline 与 measurement 都必须声明相同 method；budget 拒绝缺失、重复或不匹配的方法标记。fixture self-test 覆盖 24 次 reset、33 份 timing 目录、cold/warm 契约和方法不匹配负例。
- timing artifact 增加 hardware、Git/runner/Actions/Cargo incremental 元数据，直接依赖改为 `cargo tree --depth 1`。固定 runner 先用现有 `setup-onnxruntime.sh` / `require_onnxruntime` 准备 embedding 路径，避免干净 runner 在 `ort-sys` 处失败。
- 本地完成八 package 三次真实 rebuild 样本，但明确使用 `--skip-workspace`；budget 按设计拒绝该 warm-only measurement，不能用伪 cold 值补齐。
- `check-refactor-docs.sh` 在 `git ls-files` 之外增加 `git cat-file -e HEAD:<file>`，仅 staged、尚未进入提交历史的文档不能通过 3.27 门禁。
- Bash 语法、fixture self-test、workflow YAML、Server/Tauri minimal-host 重依赖扫描和精确 diff check 通过；状态检查先通过结构/status/current-source，再因 V2 未 tracked 按设计失败。

未关闭条件不变：当前代码和文档进入真实提交并推送；固定 `ubuntu-24.04` / Rust 1.95.0 生成 workspace cold、三次 package rebuild、hardware/timing/dependency artifact；用该 runner 的 median 替换 provisional baseline 并通过 budget。3.23、3.27、阶段 L/M 与总体保持 `未完成` / `35/44`。

## 批次 320 · build-feedback Action pin 与 job 合同门禁（2026-07-28）

状态：`未完成`

- `build-feedback` job 复用正式 release workflow 已核验的 commit，把 checkout、Rust 1.95.0 toolchain 和 artifact upload 从可移动 tag 固定到 40 位 SHA；没有新增 Action 或 CI job。
- 现有 `check-build-feedback-tools.sh` 直接截取唯一 `build-feedback` job，锁定 `ubuntu-24.04`、incremental、三项 Action pin、文档检查、ONNX 前置、三次测量、预算和 always-upload artifact，并拒绝该 job 重新出现 `@vN` / `@stable`。
- `origin/master` 仍为 `9deb6ca453524456003e6c42264f2bc583cb158b`，其提交树不含当前 build-feedback workflow/checker；匿名 GitHub REST API 本次返回 403 限流，因此不把 API 状态写成新证据。

本批只关闭 CI 配置可移动和无自检的本地缺口。当前代码/文档仍未进入远端提交，固定 runner 尚无 cold/warm artifact；3.23、3.27、阶段 L/M、3.29 与总体继续保持 `未完成` / `35/44`。

## 批次 327 · 文档 base/diff fail-closed · 2026-07-28

状态：`未完成`

- 根因是 `check-refactor-docs.sh` 只在 `BASE_REF` 可解析时才检查架构改动，并通过 process substitution 读取 `git diff`；base 缺失或 diff 无 merge base 时都会静默得到空变更集，从而绕过“架构边界变化必须同步计划或进度记录”的合同。
- 现有 checker 现在先把 `BASE_REF` 显式解析成 commit，解析失败立即报错；`git diff BASE...HEAD` 的退出状态也被显式捕获，无法比较时 fail closed，不再把基础提交缺失当作没有架构变更。
- `check-build-feedback-tools.sh` 继续复用同一自检，新增锁定 `fetch-depth: 0`、`A4N_REFACTOR_BASE` 和两个 fail-closed 错误合同。官方 `dtolnay/rust-toolchain` 当前固定 commit 的 `action.yml` 已确认内部选择 Rust 1.95.0，无需再叠加第二个 toolchain 配置来源。
- Bash 语法、build-feedback fixture self-test、workflow YAML、`actionlint v1.7.7` 通过；独立临时 Git 仓库负向夹具证明缺失 base 和无 merge base 两种情况均明确失败。

当前代码和文档仍未进入远端提交，固定 `ubuntu-24.04` 尚无 cold/warm artifact；本批不关闭 3.23、3.27、阶段 L/M 或 3.29，总体保持 `未完成` / `35/44`。

## 批次 328 · 44 项状态和当前汇总一致性 · 2026-07-28

状态：`未完成`

- 原状态 checker 只要求 3.1–3.29 与阶段 A–M 各有一个合法状态，没有约束 A1、最终整体状态、44 项总数、3.29 前置关系，也没有把计划、进度记录和工作包 README 的当前 `35/44` 汇总与真实状态计算绑定。
- `check-refactor-plan-status.sh` 现在从 V2 唯一计算 29 个定义、13 个阶段、A1 和最终状态共 44 项；3.29 必须服从 3.1–3.28 与阶段 A–M，最终状态必须服从其余 43 项，阶段 A 完成时 A1 也必须完成。
- 同一计算结果必须同时出现在 V2 最新观察、进度记录顶部看板和工作包 README 当前快照；单改状态不改汇总、单改汇总不改状态都会显式失败，没有第二份手工计数 owner。
- 当前真实文档结构检查通过后仍只在 V2 尚未进入 Git 历史处失败。独立已提交 Git fixture 的正向检查通过；把 README 改成 `34/44` 或把 3.29 提前改为完成的两个负向夹具均被拒绝。

本批只关闭状态漂移的本地门禁缺口；固定 runner artifact 与 commit/blame 仍缺，3.23、3.27、阶段 L/M、3.29 和总体继续保持 `未完成` / `35/44`。

## 批次 329 · 编译反馈依赖与 TSV 证据完整性 · 2026-07-28

状态：`未完成`

- `cargo metadata --no-deps --locked` 证明八个目标 package 中 `a4n-tauri` 有直接 build-dependency `tauri-build`；旧 `cargo tree -e normal --depth 1` 会把它排除，因此原“直接依赖清单”并不完整。
- 唯一测量脚本改为 `cargo tree -e normal,build --depth 1`，仍按实际 Ubuntu 编译目标和 Server/Tauri minimal features 生成每包一份清单，不引入全目标或第二依赖 inventory。
- baseline TSV 现在必须恰好三列，measurement TSV 必须恰好两列；旧解析器接受额外字段的宽松行为已删除。现有 fixture self-test 同时要求八份依赖文件，并增加额外字段负例。
- Bash 与 build-feedback fixture self-test 通过；真实 `a4n-tauri` tree 已显示直接 `[build-dependencies] tauri-build v2.6.0`。

固定 runner 仍未生成真实 timing/dependency artifact，本批不关闭 3.23、阶段 L/M、3.29 或总体，继续保持 `未完成` / `35/44`。

## 批次 330 · API/domain/manifest 文档同步边界 · 2026-07-28

状态：`未完成`

- 原文档治理路径覆盖 core/runtime/orchestrator/protocol/capability/manager，却遗漏 V2 明确指定的 `a4n-api-contract`、`a4n-domain` 和各 crate 自有 `Cargo.toml`；这些边界单独改变时可以不更新状态文档。
- 现有 `check-refactor-docs.sh` 的唯一 architecture case 已补入上述三类路径，没有创建新清单或把所有普通业务源码都纳入噪声门禁。
- 临时已提交 Git fixture 分别只修改 API contract、domain 和一个 crate manifest，三种情况都在未同步 V2/进度记录时得到同一明确拒绝。

真实 commit/blame 仍缺，本批不关闭 3.27、阶段 M、3.29 或总体，继续保持 `未完成` / `35/44`。

## 批次 331 · 3.29 阻塞列表与当前事实去陈旧化 · 2026-07-28

状态：`未完成`

- 状态 checker 新增把 3.29 正文列出的阻塞项与 3.1–3.28 的真实未完成状态逐项比较；数字正确但漏项、多项或顺序漂移都会失败。临时已提交 Git fixture 删除 3.23 后得到明确拒绝。
- V2 2.4–2.6 原先仍声称业务主要留在 Runtime、完整 Orchestrator getter/公开 queue/engine 尚存、没有 Windows/macOS runner、资源/协议/数据/metrics/SBOM/graceful shutdown 未实现且 docs 全部被 ignore；这些早期快照与当前完成状态和源码相反，现已删除并收敛为当前 owner、CI、发布和真实外部缺口。
- 工作包原先把 ignored `target/build-feedback/runs/...` 写成当前可用 artifact，但该目录已不存在；现明确标记为批次 310 的临时 provisional 证据，不能代替固定 runner artifact。

本批修复状态文档自身的反向结论，不伪造远端证据；3.23、3.27、阶段 L/M、3.29 和总体继续保持 `未完成` / `35/44`。

## 批次 332 · A3 工作包协调状态回写 · 2026-07-28

状态：`未完成`

- README 和工作包 01–05 的任务卡仍写“进行中、待 09/06/协调者”，但 V2 已在批次 303 关闭 3.10/阶段 A，06/09 也分别在批次 305/303 收口；该协调状态会诱导新会话重复迁移已关闭 owner。
- 五份工作包只更新任务卡当前状态，README 五行改为已收口并保留原定向测试/owner 证据；历史失败、交接和验证正文不重写。
- 工作包 05 的当前观察同步为 253/829，明确未知一级域、本地纯 rules、透传壳和生产 `.orchestrator()` 均为 0；AppState 文本量不再作为待办。

当前活跃工作包仍只有 10/11 的外部发布、固定 runner 和 Git 证据；总体保持 `未完成` / `35/44`。

## 批次 333 · Git 文档证据与首轮 CI 前置修复 · 2026-07-28

状态：`未完成`

- `HEAD`、`origin/master` 与 Actions run `30361691038` 均对应提交 `b4587817303bdf91fd2bb1d2972d5f73bfe4f15e`。本计划、进度记录、工作包 README 和本文件均已被 Git 跟踪、存在于 `HEAD:<file>`，并有真实 `git log` / `git blame`；子包 B 和 V2 3.27 因此完成。
- Actions run `30361691038` 最终为 11 个成功、25 个失败；其中 build-feedback job 在状态检查阶段因 `rg: command not found` 退出，always-upload artifact 只有 504 字节 provisional baseline，没有 workspace cold、八包三次 rebuild、Cargo timings、runner/hardware metadata 或 dependency inventory，不能关闭 3.23。
- 同一远端 run 证明 Host minimal 被 `build.sh` 的无条件 ONNX 检查错误拦截，full backend 则确实需要但未准备 ONNX。本批让 `build.sh` 依据实际 Cargo normal/build 依赖图决定是否要求 FastEmbed/ONNX，full workspace 复用现有 setup；所有调用 `rg` checker/smoke 的受影响 Linux/macOS job 显式安装 `ripgrep`，既有 build-feedback 合同锁定该依赖。状态 checker 还关闭 Git 非 ASCII 路径转义，避免中文计划/进度文件在 diff 中无法匹配。
- Bash/YAML、build-feedback fixture、状态/文档 checker 与 diff 检查通过；无效 ONNX 根下 minimal Server 已进入 Cargo check，但受共享 target 锁和 60 秒预算影响超时，full Server 则仍按预期 fail closed。
- 最小本地门禁通过后仍需提交并推送本批补丁，再等待下一次固定 runner 成功 artifact。当前仅 3.23/阶段 L 属于本工作包未关闭范围；总体为 `36/44`、还差 8 条。

## 批次 334 · generation inventory 纳入 Git · 2026-07-28

状态：`未完成`

- 远端提交 `4fc1b0c9d9e30d734e441b152a91fef4139de249` 的 Host minimal 已通过 Server/Tauri 编译和重依赖扫描，最终只因 `docs/重构/全量增量生成迁移清单.md` 在干净 checkout 中不存在而失败。
- 根因是该既有权威清单仍命中 `docs/重构/*` ignore；本批只新增单文件 allowlist 并纳入 Git，不放行其他本地重构文档，也不复制 inventory。
- 当前共享工作树及独立 clean-HEAD worktree 均通过 `inventory-generation-paths.sh --check`：`actual=47 / declared=47 / missing=0 / stale=0`。
- 3.27 继续保持完成；本包唯一未关闭条件仍是 3.23/阶段 L 的 fixed-runner 成功 timing artifact。

## 批次 335 · fixed-runner baseline 建立 · 2026-07-28

状态：`未完成`

- GitHub Actions run `30368914502` 的 `Cold and warm build feedback budget` job（`90307300857`）成功，artifact `build-feedback-Linux-X64`（`8692474800`）包含 workspace cold、八包各三次 rebuild、Cargo timings、hardware、toolchain 和直接依赖清单。
- 固定环境为 Ubuntu 24.04、AMD EPYC 7763、4 logical CPUs、Rust/Cargo 1.95.0、`CARGO_INCREMENTAL=1`；workspace cold 为 `228231 ms`。
- `package-rebuild-v1` 中位数依次为 prompt `739`、LLM `597`、embedding `654`、RAG `777`、image `615`、video `619`、Server `4792`、Tauri `27802 ms`。本批直接替换 provisional baseline，并统一收紧为 `25%` regression ceiling，不新增第二套性能配置。
- 3.23/阶段 L 暂不提前关闭：还需该 baseline 进入 Git 后由同一 fixed runner 再通过一次 budget。

## 批次 336 · fixed-runner budget 复验与收口 · 2026-07-28

状态：`完成`

- baseline 提交 `c16d35351201567451418e3c1af63a9bee48a9f8` 触发 GitHub Actions run `30371035840`；`Cold and warm build feedback budget` job `90314642186` 成功。
- artifact `build-feedback-Linux-X64`（`8693269442`，1,877,890 bytes）记录 workspace cold `172303 ms`；八包中位数为 `571 / 458 / 542 / 637 / 482 / 489 / 4265 / 22122 ms`，全部通过首份 fixed-runner baseline 的 `25%` ceiling。
- artifact 继续包含每次 Cargo timings、原始样本、runner/hardware/toolchain metadata 和 `cargo tree --depth 1`；minimal Server/Tauri 及重依赖扫描已在前一 run 成功，不新增平行门禁。
- 3.23 与阶段 L 正式关闭；工作包 11 完成。总体由 `36/44` 更新为 `38/44`，未完成 6 条。
