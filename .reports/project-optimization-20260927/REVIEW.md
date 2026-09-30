# A4N 项目优化调查 · 2026-09-27

本轮确认三处值得优先处理的问题：一处已在真实浏览器复现的状态刷新缺陷、一处读取历史记录的性能优化、一处任务恢复入口的体验改进。建议按此顺序处理；本轮只调查和保存复现证据，未修改业务代码或用户数据。

## 1. 优先修复：慢响应会让正文事实面板无法结束轮询

**证据等级：真实 React 组件 + Chromium + 受控 API 延迟，已复现。**

当面板加载到正在运行的事实归纳任务，并依靠轮询等待完成时，如果后续请求持续比 1.5 秒轮询周期慢，完成结果会反复被丢弃。用户看到的归纳覆盖数停留在旧值，归纳、重建和预览按钮继续禁用。

源码位置：[FactFeedbackPanel.tsx](../../frontend/src/pages/novel-detail/FactFeedbackPanel.tsx)。

- 58–71 行：每次 `load()` 都增加请求序号，只有序号仍最新的结果可以写入状态。
- 84–88 行：运行期间每 1,500ms 发起下一次请求，不等待上一次完成。
- 158、180–183 行：旧的 `running` 状态继续控制按钮禁用。

以每次响应耗时 1,800ms 为例，请求 A 返回时请求 B 已经开始，因此 A 被丢弃；B 返回时 C 已开始，同样被丢弃。只要这种时序持续，面板就收不到任何可以采用的完成结果。两个机制单独看都有合理用途，组合后出现了持续淘汰响应的问题。

本次复现首次立即返回 `running`，后续全部返回 `completed`，只改变后续 API 延迟：

| 条件 | 浏览器观察 | 归纳按钮 |
| --- | --- | --- |
| 50ms 对照 | 覆盖数变为 1，待处理变为 0；只调用两次，轮询停止 | 可用 |
| 1,800ms | 至少三个完成响应已返回；覆盖数仍为 0，待处理仍为 1 | 禁用 |

这是受控慢响应下的前端缺陷证据，不表示真实数据库当前已经达到 1,800ms。由本面板直接发起的 `run()` 另有写入最终结果的路径，因此不能声称所有执行方式都会卡住；重新进入已有运行任务、依赖状态轮询时能触发本问题。

**建议的最小改动：**让同一面板的轮询请求不重叠，例如等待当前请求结束后再调度下一次。继续保留卸载、切换项目及过期读取的保护。单纯延长周期只能改变触发阈值；删除序号保护会重新引入旧结果覆盖新结果的风险。

修复验收应使用相同慢响应场景，要求完成状态最终生效、轮询停止、按钮恢复；同时验证卸载和主动刷新期间不会采用过期结果。本次脚本是“确认旧缺陷”的复现器，断言成功不代表缺陷已修好。

## 2. 性能优化：在数据库中限定有效变更历史

**证据等级：调用链与查询源码已确认；未进行生产压测。**

事实摘要每次加载都会读取项目的全部相关变更记录，随后才在 Rust 内过滤并截取 100 条。运行中的面板会周期性请求同一个摘要，因此历史增长会增加反复读取、传输到服务端和反序列化的工作量。

调用链：

1. [factFeedback.ts](../../frontend/src/api/factFeedback.ts) 的 `getProjectFactFeedback()` 调用摘要接口。
2. [fact_feedback/mod.rs](../../crates/host/a4n-orchestrator/src/fact_feedback/mod.rs) 58–63 行读取变更；116–119 行先比较 `before_json != after_json`，再 `take(100)`。
3. [fact_feedback/records.rs](../../crates/platform/a4n-db/src/fact_feedback/records.rs) 165–174 行按项目过滤、排除 `chapter_feedback`、按 ID 倒序后直接 `.all(db)`，没有数据库行数限制。

全仓搜索该函数调用，生产使用点只有上述摘要入口，另有数据库测试调用。

**建议的最小改动：**由数据库查询负责“排除无变化记录 → 倒序 → 取 100 条”，维持现有输出含义。不能只在原查询加 `LIMIT 100` 后继续在内存过滤，否则最新记录中有无变化项时，会漏掉更早的有效变更。

先用超过 100 条、夹杂无变化项的数据检查结果与原语义一致，再比较大历史量下的查询耗时及返回行数。是否需要索引应依据查询计划判断；目前没有证据支持增加缓存层或更换存储。此项也不能被当作第一项的已证实生产诱因。

## 3. 体验优化：从任务队列直接返回需要恢复的业务页面

**证据等级：组件、任务字段与项目路由源码已确认；未进行完整交互验收。**

[WorkflowRunsPanel.tsx](../../frontend/src/pages/settings/queue/WorkflowRunsPanel.tsx) 21–26、124–155 行区分了可以在队列恢复的任务和必须回业务页恢复的任务。后一类显示“请到对应业务页面恢复”，但行内菜单只有查看详情，没有前往目标项目或章节的入口。展开后的详情同样没有业务页导航。

已有 `WorkflowRunSummary` 包含 `project_id`、`root_project_id`、`chapter_id` 和 scope 字段；[projectRouting.ts](../../frontend/src/utils/projectRouting.ts) 98–147 行已经负责不同项目类型的入口选择。

**建议的最小改动：**对目标可定位的任务增加“前往项目继续”，复用已有项目路由；能够准确定位章节时再进入对应章节。目标缺失、权限不足或项目已删除时显示明确原因。实际恢复仍交给业务页已有逻辑，避免扩大队列恢复白名单而绕过业务前置条件。

收益是缩短“知道任务可以恢复 → 找到正确入口”的路径。这属于可发现性改进，不能表述成项目缺少任务恢复能力。

## 已排除的疑点

- **模型费用只统计最近 100 条：不成立。**[model_usage.rs](../../crates/platform/a4n-db/src/model_usage.rs) 的 `list_model_usage_records` 限制明细数量，`aggregate_model_usage_records` 独立按时间范围聚合；不能据明细上限推断费用总计截断。
- **事实面板切项目时必然残留旧 busy 状态：证据不足，不计为缺陷。**[OverviewTab.tsx](../../frontend/src/pages/novel-detail/OverviewTab.tsx) 和 [AnalysisSubTab.tsx](../../frontend/src/pages/writing-desk/AnalysisSubTab.tsx) 的两个实际调用都以项目 ID 作为 React `key`，切换项目会重新挂载。

## 复现与验证边界

源码基线为 `debug d07261552c783b263dae0821ae1fc884b0ba0231`。复现运行于本会话的 `agent/project-film-20260927` 工作树。本轮又逐一比较了当前 debug 中事实面板、事实摘要、变更查询和队列面板的 SHA-256，四个文件与调查版本一致；未覆盖其他会话的未提交修改。

事实面板 SHA-256：`841b1cec2f0b1eaba9b22a08322cc8561e2f9d8e1e3e50504b9a6d00f4b12293`。

复现文件：

- [polling-repro.mjs](polling-repro.mjs)：打包未改动的生产组件，只替换 API、翻译和装饰组件等外部依赖。
- [polling-repro.py](polling-repro.py)：启动临时本地静态服务器，在 Chromium 中执行快慢两组对照并保存结果。写入 API 均为遇到调用就抛错的桩，没有连接业务服务或数据库。
- 原始运行证据：工作树内 `.dev-runtime/project-optimization-20260927/result.json`、`latency-50.png`、`latency-1800.png`，不纳入 Git。

在本会话工作树根目录重现：

```bash
A4N_FRONTEND_DEPENDENCIES=/home/fufu/Code/A4N/work/debug/frontend \
  node .reports/project-optimization-20260927/polling-repro.mjs
.dev-runtime/film-env/bin/python .reports/project-optimization-20260927/polling-repro.py
```

环境：Node 24.18.0、React 18.3.1、esbuild 0.25.12、Chromium 150.0.7871.186；Python 环境提供 Playwright。仅只读引用已安装的前端依赖，构建产物及浏览器输出均写在本会话工作树；未安装新的项目依赖。

已执行浏览器对照复现、JavaScript/Python 语法检查和本次新增文件的 diff 检查。未运行 Rust、真实数据库压测或完整业务 E2E，因为本轮没有业务实现改动。本报告是围绕事实回流、任务恢复与成本展示的定向调查，不是全仓已经无其他问题的结论；第一项缺陷仍待修复。
