# A4N 剩余重构工作包

基线日期：2026-07-25（批次 216 之后）

这组文档是给多个 agent 协作使用的执行分工，不是新的完成状态来源。状态、关闭条件和最终数字仍以以下两份文档为准：

- [模块化重构执行计划 V2](../模块化重构执行计划-v2.md)
- [模块化重构进度记录](../模块化重构进度记录.md)

当前 `.gitignore` 已精确放行本计划、进度记录和本目录的工作包文档；提交 `b4587817303bdf91fd2bb1d2972d5f73bfe4f15e` 已将它们纳入版本历史，后续修改仍需正常提交后才可形成新证据。

## 当前会话工作包

| 文档 | 状态 | 范围 | 开始日期 |
|---|---|---|---|
| [03-A3-character-canvas-galgame.md](03-A3-character-canvas-galgame.md) | 已收口（批次 303/305 回写 V2） | Character 79/79、Canvas 47/47、Galgame 105/105；共享 API/生命周期边界已关闭 | 2026-07-25 |
| [02-A3-video-generation.md](02-A3-video-generation.md) | 已收口（批次 303 回写 V2） | Orchestrator `video::` 357/357、Runtime `video_generation` 38/38；Video owner 与生命周期边界已关闭 | 2026-07-25 |
| [01-A3-project-transfer-import-export.md](01-A3-project-transfer-import-export.md) | 已收口（批次 303 回写 V2） | Project transfer/duplication/import/export owner、结构门禁与 09 生命周期交接已纳入 3.10/阶段 A 关闭 | 2026-07-25 |
| [04-A3-chapter-and-creative-domains.md](04-A3-chapter-and-creative-domains.md) | 已收口（批次 303/305 回写 V2） | source_sync 105/105、shooting_script 24/24；共享 API/生命周期边界已关闭 | 2026-07-25 |
| [05-A3-management-and-residual-domains.md](05-A3-management-and-residual-domains.md) | 已收口（批次 303 回写 V2） | 本地纯 rules、透传壳、生产 `.orchestrator()` 与未知一级域均为 0；AppState 已归类为合法 Host 边界 | 2026-07-25 |
| [06-host-api-contract-and-core-boundary.md](06-host-api-contract-and-core-boundary.md) | 已收口（批次 305 回写 V2） | 唯一 API 合同、DTO inventory、TS 生成、parity 与 301 项 core public-type inventory 均通过；3.12、3.24、阶段 C 已关闭 | 2026-07-26 |
| [07-sidecar-process-and-postgres.md](07-sidecar-process-and-postgres.md) | 已收口（批次 305 回写 V2） | 八项真实 sidecar process 能力矩阵和 PostgreSQL 门禁/失败恢复证据满足原始条件；3.13、阶段 D/E 已关闭，远端 run 继续作为发布审计证据而非源码关闭条件 | 2026-07-25 |
| [08-sidecar-security-resource-protocol.md](08-sidecar-security-resource-protocol.md) | 已收口（批次 306） | 唯一安全/资源/兼容 owner、八项 capability process、Export 历史/current/rollback 四组合和 Linux mTLS/watcher 满足原始关闭条件；3.19–3.21 与阶段 J/K 已关闭 | 2026-07-25 |
| [09-media-lifecycle-and-host-events.md](09-media-lifecycle-and-host-events.md) | 已收口（批次 303） | 33 个生命周期入口、created-media/scanner/cleanup outbox、PostgreSQL 双故障/强杀恢复、唯一 generation event、Server SSE/Tauri Channel、真实 Chromium/Wry 重挂载与 Destroyed 均满足原始关闭条件；远端与跨平台发布证据转交 10 | 2026-07-27 |
| [10-release-observability-supply-chain.md](10-release-observability-supply-chain.md) | 进行中（批次 346 普通三平台与编译反馈收口） | Windows/Linux/macOS 普通发布 smoke、Generation 和刷新后的单一编译反馈基线均已远端通过；待正式签名 Release | 2026-07-25 |
| [11-compile-feedback-and-doc-governance.md](11-compile-feedback-and-doc-governance.md) | 已收口（批次 336） | 3.27 已有真实 Git 证据；两次 fixed-runner artifact 已建立并复验 `25%` cold/warm 编译反馈门禁，3.23 与阶段 L 已关闭 | 2026-07-28 |

本会话当前工作包：`10-release-observability-supply-chain.md`。工作包 11 已于批次 336 收口：3.27、3.23 与阶段 L 全部关闭。总体为 `38/44`，剩余工作转入正式发布、签名与供应链证据。

## 当前快照

| 指标 | 当前值 |
|---|---|
| V2 完成定义 | 38 / 44，未完成 6 条 |
| 主战场 | 3.17 / 3.26 与阶段 H/M；均需正式平台、签名或 release 身份 |
| Runtime actions | 526 个生产文件 / 27,801 行；含测试共 33,893 行（`refactor-metrics.sh`，2026-07-28） |
| `AppState` 引用 | 253 个 actions 文件 / 829 次命中 |
| 实际待迁主队列 | 0 个本地纯 rules 文件 / 0 行；0 个 Orchestrator 透传壳文件 / 0 行 |
| A2 本地纯 rules | 0 文件 / 0 行 |
| Orchestrator 透传壳 | 0 文件 / 0 行 |

当前协调记录（2026-07-30）：run `30471463443` 对提交 `9506698` 的 Windows portable、Linux installed release、macOS unsigned Tauri lifecycle 和 Generation 全部通过；Windows 日志内容读取与首次 SSE 重连修复均已取得真实 runner 证据。旧 build-feedback 基线因 hosted runner 从 EPYC 7763 切换到 EPYC 9V74 而失配后，已按第一次 9V74 artifact 整体刷新八包 baseline，统一 `25%` ceiling 不变；提交 `688ac09` 的后续 run `30512222556` 中，job `90774535660` 又在 Intel Xeon Platinum 8573C / 4 CPU 上完整通过，证明无需增加 CPU 分支。正式签名/公证、受保护 environment、tag、最终 manifest/CMS 和 GitHub Release 下载证据仍缺。整体 V2 为 38/44；剩余 6 条继续由工作包 10 承接。

`AppState` 数字是边界观察量，不是待迁业务 owner 数量。权限、队列、provider、文件系统、FFmpeg、缓存、事件桥和 DTO 适配可以合法留在 Host；不要为了降低数字而增加第二套 facade 或搬动基础设施。

> 批次 216 的独立快照仍记录为 34,243 行；其后共享工作区有并行会话继续删除无消费者 Runtime 代码。合并并行改动后应重新执行 `scripts/checks/refactor-metrics.sh`，不要把移动中的数字写成单批成果。

## 已完成、不要重复实现的范围

以下 V2 条目当前已标记 `完成`，新 agent 只能做回归审计，不能重新设计或复制实现：

- 3.1–3.9：crate 拆分、Host 依赖隔离、migration、协议主体、失败语义、前端 Host API、full workspace 编译。
- 3.11：`AppState` / `OrchestratorContext` 去 service locator 化；`Deref`、完整 context getter 和公共基础设施字段已由结构门禁禁止。
- 3.12、3.24：`a4n-api-contract`、TS 生成、API parity 与 core public-type inventory 已关闭。
- 3.13：八个 sidecar 的真实 binary 通用/能力专项 process 矩阵和 CI 入口已关闭。
- 3.19–3.21：认证/egress、资源/背压和真实历史/current/rollback 协议矩阵已关闭。
- 3.14：PostgreSQL CI 的主体证据已存在；若发现阶段 E 的状态文字不一致，只修正状态/证据，不重建第二套数据库测试框架。
- 3.18：canonical `TaskKind` 唯一来源。
- 3.22：sidecar 持久数据 manifest、恢复和 graceful shutdown 的主体证据已存在；不要另造数据目录协议。
- 3.28：service catalog 和多实例基础测试已存在；只在其他包需要时复用现有 catalog。
- 阶段 A1/A2、阶段 B/C/D/E、阶段 I/J/K：Host API 白名单、状态私有化、合同、process/PostgreSQL、安全/资源/协议和任务合同已完成。

## 工作包矩阵

| 文档 | 工作包 | 覆盖的 V2 条目 / 阶段 | 可否与 A3 并行 |
|---|---|---|---|
| [01-A3-project-transfer-import-export.md](01-A3-project-transfer-import-export.md) | project transfer、复制、导入、导出 | 3.10、阶段 A | 可以；只改本包业务域 |
| [02-A3-video-generation.md](02-A3-video-generation.md) | Video Generation 剩余完整执行链 | 3.10、阶段 A | 可以；不改共享媒体清理框架 |
| [03-A3-character-canvas-galgame.md](03-A3-character-canvas-galgame.md) | Character AI、Canvas、Galgame | 3.10、阶段 A | 可以；按目录边界并行 |
| [04-A3-chapter-and-creative-domains.md](04-A3-chapter-and-creative-domains.md) | chapter/scene、character review/conflict、source/scripting/blueprint 等 | 3.10、阶段 A | 可以；先避开共用 Host 根模块 |
| [05-A3-management-and-residual-domains.md](05-A3-management-and-residual-domains.md) | P2 管理、读路径和小业务域分类收口 | 3.10、阶段 A | 可以；先盘点再迁移 |
| [06-host-api-contract-and-core-boundary.md](06-host-api-contract-and-core-boundary.md) | `a4n-api-contract`、DTO、core inventory | 3.12、3.24、阶段 C/L | 盘点可并行；公共合同迁移需协调 |
| [07-sidecar-process-and-postgres.md](07-sidecar-process-and-postgres.md) | 八项 process matrix、CI、PG 证据复核 | 3.13、阶段 D/E | 可以；共享 workflow 合并需串行 |
| [08-sidecar-security-resource-protocol.md](08-sidecar-security-resource-protocol.md) | 认证、环境隔离、限流超时、版本协商 | 3.19–3.21、阶段 J/K | 可以；复用 catalog/bootstrap |
| [09-media-lifecycle-and-host-events.md](09-media-lifecycle-and-host-events.md) | 文件/媒体生命周期与统一事件桥 | 3.15、3.16、阶段 F/G | 盘点可并行；实现依赖 A3 owner map |
| [10-release-observability-supply-chain.md](10-release-observability-supply-chain.md) | 跨平台发布、metrics、restart、SBOM、binary manifest | 3.17、3.25、3.26、阶段 H/M | 可以；发布 workflow 合并需串行 |
| [11-compile-feedback-and-doc-governance.md](11-compile-feedback-and-doc-governance.md) | 编译 timing、文档入 Git、状态一致性 | 3.23、3.27、阶段 L/M | 可以；最终状态回写由协调者完成 |

3.29 的最终关闭不分配给任何单独 agent：只能由协调者在所有工作包、V2 3.x 条目、阶段 A–M 和最终 CI 门禁同时满足后更新。阶段 B/E 与已完成 3.11/3.14 的状态一致性由工作包 11 审计，不重复实施已完成代码。

## 建议并行拓扑

```text
协调者：读取 V2/进度 → 建立基线 → 分配工作包
  ├─ 01 / 02 / 03 / 04 / 05   A3 业务域（目录互斥，可并行）
  ├─ 06                       API 合同与 core 分类（先 inventory）
  ├─ 07                       process test / PostgreSQL CI
  ├─ 08                       sidecar 安全、资源、协议
  ├─ 09                       媒体生命周期、Host 事件
  ├─ 10                       发布、可观测性、供应链
  └─ 11                       编译基线、文档和状态治理

A3 owner map 完成后：09 才能收最终跨域补偿；06 的 DTO 迁移、10 的发布清单和 11 的最终状态回写必须由协调者串行合并。
```

## 共享文件锁

默认只有协调者修改以下文件；agent 如确需修改，先在交接记录中声明，避免并行覆盖：

- `docs/重构/模块化重构执行计划-v2.md`
- `docs/重构/模块化重构进度记录.md`
- `docs/重构/模块化重构度量快照.md`
- `docs/重构/剩余重构/README.md`
- `.gitignore`、`Cargo.lock`、workspace 根 `Cargo.toml`
- `.github/workflows/*.yml`
- `crates/a4n-runtime/src/lib.rs`、`host_api.rs`、`crates/a4n-orchestrator/src/lib.rs`
- service catalog、公共 CI 检查脚本和 frontend 生成入口

各 agent 应优先修改自己的域目录、对应测试和必要的局部模块声明；不要顺手全库格式化、整理其他 agent 的脏工作树或回滚并行改动。

## 每个工作包的完成证据

完成一个包不等于关闭 V2 条目。交接时必须给出：

1. 基线快照、实际修改路径和明确的 out-of-scope 项。
2. 新 owner、删除的旧 owner、是否存在兼容 wrapper/第二套规则。
3. 目标 crate check、定向测试、必要的 PostgreSQL/process test、`cargo fmt`/`git diff --check` 结果。
4. 失败或未执行的命令，不能把超时、未进入 runner 或缺环境写成通过。
5. 对应 V2 关闭条件仍缺哪些；只有全部满足才请求协调者更新状态。

推荐交接格式：

```text
工作包：
状态：未开始 / 进行中 / 阻塞 / 待合并 / 完成
基线：
修改路径：
新 owner 与删除 owner：
验证命令及结果：
未关闭条件：
与其他工作包的冲突/依赖：
```

## 全局禁止事项

- 不以 `AppState` 引用数、文件数或批次数下降冒充 3.10 完成。
- 不在 Runtime、Orchestrator、DB 或 sidecar 再复制一份业务规则、权限规则、事件合同或失败补偿。
- 不用 fallback、静默跳过、吞错或“兼容旧路径”掩盖未迁移的真实 owner。
- 不修改已完成条目来满足新包的局部方便；缺口应回到本包的关闭条件。
- 不在没有真实环境时声称 PostgreSQL、Windows/macOS、remote mTLS 或 process runner 已通过。
