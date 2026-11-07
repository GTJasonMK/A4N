# debug 冗余代码复查与清理（2026-11-07）

状态：源码清理、编译与定向测试、完整创作及真实浏览器复验全部完成。基线为 `ec8b315b1437899da3d598e5557002bc258407c2`，工作树起始干净；按用户要求直接在 debug 修改，不新建分支，不实施 Agent RAG。

## 审查方式

- 从上轮优化涉及的调用链、Rust 编译告警和工作区依赖入手，以 CodeGraph 定位后核对当前源码及全仓引用，确认运行入口、测试、重导出和配置是否仍有使用。
- TypeScript 检查覆盖 788 个源文件；对业务前端导出的函数/组件进行符号引用分析，并复查 E2E、脚本和 Rust 中的引用。API 合同、生成文件及 i18n 数据不按普通私有函数删除。
- 检查旧版本解析、回退、错误处理、版本校验和取消状态；保留有实际调用的数据升级及环境兼容路径。
- 以下证据路径相对 `data/redundancy-cleanup-20261107/`，日志、数据库、截图和构建产物不纳入 Git。

## 清理结果

| 项目 | 改动与依据 |
| --- | --- |
| 无效 Rust 接口 | 删除 12 处无调用或仅供旧测试使用的函数/宏，包括章节 Worker 的旧错误映射、未使用 prompt 宏和上传写入包装。测试改走实际上传边界，继续验证解码、临时文件所有权及失败清理。 |
| Page Session 取消状态 | 删除未被命令分发使用的 cancellable runner、取消 token 查询和整条冗余 token 状态。保留真实操作状态与回执校验，取消后迟到的成功回执仍明确失败；HTTP 等待者退出后后台命令的既有寿命继续由有效入口管理。 |
| 视频更新旧入口 | 删除 application、facade、DB read-model 和存储层的无版本更新包装，以及旧 checkpoint 包装。原断言改用当前带版本校验的入口；生成流程仍保留其实际使用的显式版本参数语义。 |
| 前端死导出 | 删除 4 个旧 Agent 草稿辅助函数、未使用的事件重要度排序函数和旧组件重导出层。所有者校验、草稿保护及在途操作管理继续使用现有 owner 入口。 |
| 无效宏参数 | 删除管理员 handler 宏从未使用的 operation 参数；请求操作、Worker lease 和身份校验继续由真实处理方法执行。 |
| 重复日期回退 | 删除漫画记录更新中两个重复的秒级日期解析分支；原主格式 `%.f` 已支持整秒及带小数秒，非法版本继续明确失败。 |
| 未使用依赖 | 删除 project-status Worker 的 `a4n-worker-db-adapter`、source-snapshot Worker 的 `a4n-api-contract`；管理员 HTTP 回归测试复用现有工作区的 migration 和 tower 测试依赖，没有引入新库。 |
| 重复界面标题 | 删除漫画分镜详情内部的重复“单页分镜”，保留外层阶段标题和操作栏；真实浏览器在三个宽度断言标题只有一个。 |

有效代码 diff：33 个文件，新增 296 行、删除 526 行，共 822 行，净减少 230 行；排除文档和 Cargo.lock。测试夹具移至共享作用域、旧测试迁移至有效接口产生的改动计入上述数字。

## 修复的逻辑问题

### 管理员待处理任务索引缺少 Worker 路由

合同、CLI/客户端和 DB 查询都已有实现，但 `AdminWorker::router` 没有注册 `admin.tasks.pending.index`，真实请求返回 404。

补齐路由及 handler，继续使用既有权限和查询 owner。回归先复现 404，再覆盖无效 Worker token、普通用户拒绝和管理员读取成功；完整创作 runner 另经真实 CLI 调用该命令，结果必须是列表，不能作为预期状态拒绝放过。

关键文件：`crates/admin/a4n-admin-worker/src/lib.rs`、`scripts/tests/run_creation_e2e.py`。证据：`red-admin/`、`admin-current-verified/`、创作报告的 `admin_pending_task_index`。

### 漫画系统参考图错误依赖当前主立绘

前端曾扫描当前角色/立绘列表来判断已保存绑定的归属。列表尚未加载或主立绘更换时，已保存系统立绘会被归入用户参考图，与后端的系统图片移除规则冲突。

现在按已保存绑定的素材类型投影系统归属，用户添加集合保持原归属，并保留历史已绑定 generated_image 的用户用途。删除两组额外输入和列表扫描，页面投影不再随当前立绘列表变化。

关键文件：`frontend/src/pages/novel-detail/manga/mangaPagePromptModel.ts`、`MangaPagePromptList.tsx`。证据：`red-manga-reference.log` 中系统资产被误判为 null；随后回归 4 项及前端整组通过。

## 验证记录

| 验证 | 本轮结果 | 证据 |
| --- | --- | --- |
| Rust workspace / all-targets 编译 | 332 个测试目标全部编译通过，编译错误及告警为 0 | `compiled-targets-verified.jsonl`、`rust-build-verified.log` |
| 8 组定向后端测试 | 85 通过，0 失败，1 项原有发布性能基准忽略；各组 60 秒超时 | `targeted-summary.json`、各组 `database.json` |
| 管理员路由 | 无效 token 拒绝、普通用户拒绝、管理员成功；整组 2 项通过 | `admin-current-verified/` |
| Page Session / 后台命令 | 14 通过/1 原定义忽略；后台命令 2 通过；CLI 取消回执 1 通过 | `page-sessions-verified/`、`detached-commands-verified/`、`cli-page-cancel-verified/` |
| 上传、视频更新与漫画记录 | 上传 4、视频更新 20、视频一致性 28、漫画记录 14 项通过 | 各对应 `*-verified/` 目录 |
| 前端 | 153 文件、580 项测试通过；生产构建、严格 unused locals/parameters 检查通过 | `frontend-unit.log`、`frontend-build.log`、`typescript-unused-final.log` |
| 前端未引用导出复查 | 1,034 个业务函数/组件候选中无未引用项 | `typescript-export-final.json` |
| Python / E2E 语法 | 164 项通过；修改的浏览器脚本语法通过 | `python-unit.log`、`e2e-syntax.log` |
| API parity | 通过，2,387 项 generation 合同检查 | `parity.log` |
| 工作区依赖 | 无未使用依赖 | `cargo-machete-final.log` |
| 格式和空白 | 通过，提交前再核对暂存区 | `format-and-gates.log` |
| 完整创作与真实 UI | 17 组 CLI、6 个浏览器流程、9 个创作阶段全部通过，管理员任务索引前置检查成功 | `creation/report.json` |
| 界面独立复审 | 实际打开本轮 4 张截图，原 P3 重复标题核销，无新增 P1/P2/P3 | `visual-independent-review.json` |

Rust 本轮验证为全目标编译和受影响的 8 组测试；上轮全量与复验的数量属于上轮记录。本轮忽略项是明确标记的 release Page Session expiry benchmark，未计为通过。

媒体工作区真实浏览器 7 项通过，覆盖三宽度漫画标题唯一、双窗口删除重生成和参考图上传/刷新/移除。独立审图只评估截图可见区域，交互及持久化结论以真实浏览器断言为准。

首轮编译发现遗漏的旧测试重导出，已清理后通过；首次自动选择测试程序误选同名 bin，得到 0 用例时被非零断言门禁拒绝，修正为 lib/dylib/集成测试目标后完成 85 项验证。初始失败、零用例及后续通过证据均保留。

## 保留的必要路径与边界

- 历史数据库迁移、已保存漫画 v3 数据及旧项目包的有版本解析仍被真实读取/导入流程使用，保留其明确校验及升级路径；没有为清理代码而丢弃用户历史数据。
- 浏览器文件选择兼容、有效的运行配置继承、实际调用的测试类型适配与 schema 导出支持继续保留。它们有明确调用和用途。
- 不新增静默兜底、假成功、`allow(dead_code)`、测试跳过或新的业务规则来源；原权限、边界输入、版本冲突和失败清理保持显性。
- 真实流程只使用隔离 PostgreSQL、Qdrant 和本地虚拟 Provider；模型创作质量、原生 OS 文件对话框及跨平台发行包不在本轮验收范围。
- 生产构建保留既有大 chunk 提示；本轮没有以新的性能百分比作交付结论。

本轮被测产品源码指纹为 `fb5fd183afe44a4b60ebc9287170524aae4a51b051076531ec9acf918d221a3c`，前端指纹为 `eb48a410263101fae4c19c36a7b7ddb923c1f9efffa31ec1638aef255c889870`。最终验收汇总、源码快照核对和提交收尾分别记录在 `acceptance-summary.json`、`source-comparison.json` 与 `git-final.json`。
