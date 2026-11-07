# debug 源码修复与 WIP 收尾（2026-11-07）

状态：源码修复、既有 WIP 收尾和本轮验收完成。用户授权直接在 `debug` 修改和提交，不新建分支。Agent RAG 延期，原设计提案仅归档。

## 范围与隔离

- 起点：`e672bcdbd4d58f1d1d7e57f07a9b08c811c51252`，包含原有 193 个已跟踪改动和 10 个未跟踪文件；本轮保全并收尾这些 WIP。
- 原始备份：`work/backups/debug-closeout-20261107-150338`，包含二进制 patch、index patch、未跟踪源码和校验收据。
- 构建及验收均从 `work/debug` 执行；Node.js 为 22.21.1，Rust 产物使用 `debug/target` 及独占构建锁。
- 真实 Server、Runtime、PostgreSQL、Qdrant 和 Chromium 使用本轮隔离数据库、端口及数据目录；模型和媒体 Provider 均为本地虚拟实现，不调用付费模型。
- 以下证据路径相对 `data/closeout-20261107/`；日志、数据库、运行数据和构建产物不纳入 Git。

## 源码修复

1. **隐性属性统计窗口**：只在进入窗口外的新章节时停止，完整统计窗口最后一章；无关分类不占窗口，证据条数上限不截断统计。手工复审与自动复审共用同一修复，7 项定向回归通过。
2. **角色冲突身份**：规范姓名进入实际模型稳定上下文和请求指纹，预览复用请求构建；姓名变化不能复用旧身份的请求。4 项定向回归通过。
3. **漫画分镜原句**：按本页来源 ID 投影原文、事件、对白和旁白，执行、预览及阶段指纹共用构建；未知来源明确失败。9 项定向回归通过。

三项修复的新断言在补丁前共复现 5 个失败，`red-*.log` 与后续通过记录均保留。上下文评估与输入字段文档已同步，不将局部修复外推为全部 LLM 上下文或模型质量验收完成。

关键入口：

- `crates/host/a4n-orchestrator/src/character/implicit_review/stats.rs`
- `crates/host/a4n-orchestrator/src/character/conflict/request.rs`
- `crates/host/a4n-orchestrator/src/manga/pipeline/prompt/messages/inputs.rs`

## 漫画、迁移及既有 WIP 收尾

- 漫画生成产物采用 create-only 写入；已有产物须显式删除后才能重新生成。成图条件更新保留并发赢家，失败图片登记到既有 cleanup outbox，不覆盖原图或静默吞掉清理错误。
- 取消已完成漫画工作流的隐式续接；正常阶段推进启动新任务，显式 resume 仅用于失败、取消或中断且指纹匹配的任务。Manga Core 统一任务创建与状态读取指纹，补齐 `run_stage`。
- 检查点恢复仅补建缺失 PageContent，已有页面内容、产物指纹和版本保持不变。CLI 检查真实 Provider 请求数，覆盖重复生成拒绝、删除重生成、组合阶段两次调用和连续删除后的重新调用。
- 系统发现参考与用户添加参考分开，视觉需求识别独立于素材池是否为空；真实 Host 验证素材选择、上传、刷新持久化、移除、双窗口和 390/1024/1440px 布局。
- 修复漫画产物同时存在 NO ACTION 与 CASCADE 外键的问题，新库与升级均保留单一正确级联关系；验证重复迁移和真实项目删除。
- 收尾原测试 WIP：统一隔离数据库夹具、修复 Galgame 回填冲突迁移与历史迁移登记、保持更新预检的 Host 合同边界，修正 CLI 源码定位与测试运行入口。
- 构建脚本保留调用者的 `CARGO_TARGET_DIR`，默认并行度限制为 2；桌面入口通过现有 runner 统一构建，避免重复 Worker watcher。

## 严格验收发现并修复的问题

- **浏览器诊断假绿**：WDIO `afterTest` 抛错仅记日志；改用 Mocha root `afterEach`，并保留媒体配置继承的 hook。真实运行 Mocha 的回归证明正文通过但控制台 SEVERE 会导致失败。中文用例保留独立证据名，避免文件覆盖。
- **放弃草稿重复推进路由**：自动清理与异步放弃可能同时调用 `proceed`。共享 `useBlockerProceed` 覆盖通用导航保护、设置页和小说详情，2 项回归与真实媒体导航通过。
- **通知接口 504**：历史迁移建表名 `notification` / `recipient` 与模型不一致。新库使用规范名，兼容迁移原位重命名并保留通知、已读状态、索引和外键；混合或双份状态明确拒绝。2 项 PostgreSQL 回归覆盖新库广播、用户已读隔离、升级及重复执行；真实功能包浏览器验证通知接口返回 200。
- **视频草稿读取竞态**：列表返回 running 后，详情可能已 completed。Orchestrator 和视频记录 Worker 的只读接口返回实际快照，生命周期限制收敛到修改入口；前端按详情状态清除已完成草稿，继续拒绝错误项目/章节。前端竞态回归先失败后通过，Worker 全部 3 项、导演 CLI 和真实视频片段规划流程通过。
- **夹具与旧脚本合同**：登录态、语言和外观在应用模块启动前同步就绪；生产预览显式提供 PNG 夹具并保留解码断言。漫画脚本使用六阶段、当前有效改编样本及真实编辑流程；导演 Agent 按首个本地配置自动默认的行为断言，等待下拉选项显示，使用实际“放弃草稿”和“关闭”控件。平板 Agent 弹窗明确禁用 Escape，脚本不再用 Escape 代替关闭按钮。
- **创作验收汇总**：authoring 负责前五阶段；后续阶段校验真实前置阶段和对应业务组，缺少结果或断言继续失败。新增回归先失败后通过。

## 验证结果与证据口径

初始 `full/matrix.json` 的 22 组结果为 **18 通过、4 失败**，原文件保持不变。四个失败组由后续完整分组或针对根因的复验核销，最终汇总入口为 `acceptance-summary.json`；不能把它描述为初次全量一次全绿。

| 验证 | 结果 | 证据 |
| --- | --- | --- |
| Rust workspace / all-targets 编译 | 通过 | `rust-verified-build.log` |
| Rust 原全量 + 后续复验，按 target 去重 | 4,813 通过，62 按原定义忽略 | `full/rust-workspace/database/command.log`、下述三个复验目录 |
| Orchestrator 最终整组 | 1,601 通过、0 失败、2 忽略，450.09 秒 | `orchestrator-verified/` |
| 通知迁移增量 | 2 通过 | `notification-migration-current/` |
| 视频记录 Worker 最终整组 | 3 通过 | `video-draft-current/` |
| 前端单测 / 生产构建 | 153 文件、579 项通过；构建通过 | `frontend-video-final-unit.log`、`frontend-video-final-build.log` |
| Python | 164 项通过 | `python-video-final.log` |
| 浏览器 helper | 9 项通过 | `browser-diagnostics-final-helpers.log` |
| 生产包 UI 功能矩阵 | 9/9 组、167 项通过 | `ui-functional-strict-final.log` |
| UI presentation 标准 runner | 生产预览 31 项；源码 controls 18 项、visual 26 项通过 | `ui-presentation-verified.log` |
| 完整创作验收 | 17 组 CLI、6 个浏览器流程、9 个创作阶段全部通过 | `creation-completed/report.json` |
| API parity | 通过，2,387 项 generation 检查 | `parity-verified.log` |
| Rust 格式 / diff 空白 | 通过；提交前另检查文档与暂存区 | `format-verified-final.log` |
| E2E JS/MJS 语法 | 103 个文件通过；最后关闭按钮改动由完整浏览器复验覆盖 | `e2e-syntax-verified.log` |

Rust 计数方式：初次 332 个 target 为 4,808 通过、1 失败、62 忽略；其中旧 Orchestrator target 为 1,599 通过、1 失败、2 忽略。用最终 Orchestrator 整组的 1,601 通过替换该 target，得到 4,810；再计入新增通知迁移 2 项和 Worker 新增 1 项，得到 4,813。Worker 的其他两项已有全量结果，不重复相加。这是全量加整组/增量复验的合并覆盖数。

生产包 UI 功能矩阵在最后的视频只读修复前完成；该增量由新增前端回归、最终 Worker/Orchestrator 测试和完整真实创作流程验证，没有把较早的 UI 矩阵写成最终源码的重新全跑。

编译耗尽 60 秒、未进入断言的早期定向命令不计通过；后续使用当前已编译二进制在隔离 PostgreSQL 中实际完成 Worker 3 项测试。强行用生产预览运行依赖 `/src` 的 visual 夹具曾失败，保留记录；标准 runner 的生产预览与源码夹具结果分别报告。

## 视觉与验收边界

- 标准视觉流程产生 214 条记录，200 条含状态色对比度；错误色最小 4.79，保留 4.5 阈值。205 条有页面 overflow 测量，全部为 0，无意外错误。证据为 `visual-verified-summary.json` 及 `.dev-runtime/ui-design-closeout-verified-20261107/visual/report.json`。
- 独立审阅实际打开最终 8 张宽窄、明暗及真实 Host 截图，没有可确认 P1/P2；保留 1 项非阻塞 P3：390px 漫画阶段 5 重复“单页分镜”标题，未扩展为界面重构。
- 参考图截图中系统区标题已滚出画面；390px `settings-editor` 截图实际为列表态，不能据此宣称窄屏编辑表单审图通过。静态审图不替代交互、持久化或图片解码证据。
- 虚拟 Provider 验证请求、状态流转、断点、媒体传输和保存行为，不证明付费模型的创作质量。操作系统原生文件对话框、跨平台发行包和全站每个页面的减少动态偏好仍不在本轮验收范围。
- 构建保留既有大 chunk 与未使用代码警告，不将警告写成编译失败或宣称已清零。

## 版本、归档与 Git 收尾

- 最终被测产品源码指纹：`a82b8f6630017804f74cc7bb4f563c09bb3ed0bb7b457f94668ade2376c16c5f`；前端指纹：`48949c585d81837a8495b49905d7ffca58af0e5d49d3f5190da348b6e007c68d`。
- `source-comparison-completed.json` 验证当前源码与创作快照一致、268 个测试文件一致、四个被测程序与构建收据的 SHA-256 一致。
- Agent RAG 原提案保持原文，Git blob 为 `2198af3908d26b12d3d8c3d0cc6699d2c2ba12ce`；`rag-archive.json` 对照原 WIP 备份验证。本轮没有实施 Agent RAG。
- 本批相对起点的有效代码 diff 为 227 个文件，新增 6,525 行、删除 1,524 行，共 8,049 行；排除文档、锁文件、生成 API 与二进制，运行提示词计入源码。暂存区复核记录为 `effective-diff-completed.json`。
- Git 收尾纳入贡献指南 `7ad4471c8` 和全部授权修复；只读测试审计分支与原 debug 同点，没有独有提交。任务归档、两个干净 agent 工作树和已纳入分支的回收结果，见 `work/IN_PROGRESS.md` 与 `git-final.json`；历史登记保留，孤儿任务归档为停止而非宣称功能完成。
