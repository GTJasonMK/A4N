> 原始审查记录：保留发现时的判断与证据边界。修复后的当前状态以[主台账](../../../UI_PAGE_AUDIT.md)为准；原文中的“待修/缺证”不自动代表未解决事项。

# 写作辅助与灵感流程独立图审

已完成 8 / 8 项，实看 16 / 16 张图，每 4 张落盘一次。没有已证实的 blocker / major；1 项 minor 为 W08 桌面绑定信息重复，I01-history 窄屏首条完整性有 1 处证据缺口。仅写审查记录，未改源码、测试或启动服务。

范围限定为 W07-assistant、W08-optimizer、W09-scenes、W10-profiles、I01-conversation、I01-history、I02-progress、I05-confirm；不扩展已独立复核的 W04/W05 主工作面。审查依据为 `../novel/resume-novel-full-report.json` 与下表实看截图；自动用例通过不等于视觉或交互全部通过。频率为依据写作流程的推定，不是行为统计。

| ID | 任务 / 推定频率 / 注意力顺序 | 真实缺陷或保留依据 | 实看图路径 |
|---|---|---|---|
| W07-assistant | 按本章要求生成正文；中高频；当前章与生成模式 → 创作要求 → 字数范围 → 开始生成 | 保留。1440 的正文仍是主工作面，助手是相邻辅助栏；390 改为底部面板，要求、两个字数输入和主按钮同时可见。关闭入口、当前章、选中模式清楚，未见遮挡或水平裁切。 | 1440 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-W07-assistant-1440-initial.png`）、390 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-W07-assistant-390-initial.png`） |
| W08-optimizer | 基于章节历史继续讨论局部修改；中高频；绑定章节 → 最近回复 → 执行方式 → 输入与发送 | 无阻断。390 的最近回复、输入与发送同时可见，正文明确“尚未提交或应用”，可区分讨论与写入。1440 右栏狭窄导致长回复换行，但有可见调宽把手，不能仅凭滚动成本判 major。minor：桌面顶部连续重复“第 1 章”“第 1 章·雨夜来信”“已绑定…”与 Agent 名称，挤占聊天高度；可合并绑定摘要，保留一次当前章节标识。 | 1440 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-W08-optimizer-1440-initial.png`）、390 last（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-W08-optimizer-390-last.png`） |
| W09-scenes | 阅读并展开本章场景、按需追加场景；中频；章节与数量 → 场景标题和摘要 → 正文或生成入口 | 保留。1440 明示已有 8 场、将创建场景 9，展开内容与折叠行区别明确；390 滚动后场景 2–8 和各行展开/更多入口完整可见。顶部剩余正文是正常滚动裁切，不是被固定头部永久遮挡；长场景纵向滚动不构成缺陷。 | 1440 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-W09-scenes-1440-initial.png`）、390 last（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-W09-scenes-390-last.png`） |
| W10-profiles | 写作时核对人物状态与关系；中高频；所选人物 → 档案更新范围 → 显性/隐性/社会信息 → 详情或发现 | 保留。1440 列表与详情相邻；390 改横向人物条和纵向详情，露出下一人物提供横向浏览线索，字段改为上下排列而非硬挤成两列。当前章为第 1 章、档案“已同步至第 3 章”同时明示，不能只凭不同章号判数据错误；这里呈现的可能是最新档案，章节快照语义未由截图验证。长档案需纵向滚动属正常成本。 | 1440 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-W10-profiles-1440-initial.png`）、390 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-W10-profiles-390-initial.png`） |
| I01-conversation | 阅读最近一轮建议、选方向或继续输入；每轮高频；最近问答 → 三个方向 → 输入/发送 → 生成蓝图 | 保留。两个宽度都能同时看到三个可选方向与固定输入区；选项带动作标题、解释和短标签，用户可直接辨识，不依赖手势说明。390 的长项目名收起给主要动作让位，未挤掉选项或输入；空输入发送按钮呈禁用外观。 | 1440 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-I01-conversation-1440-initial.png`）、390 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-I01-conversation-390-initial.png`） |
| I01-history | 回看较早约定并返回最新消息；中频；历史问答 → 回到底部箭头 → 输入区 | 1440 首条完整可读，回到底部入口可见；390 历史问答与底部入口可用性外观正常，但截图首条只露尾部。证据缺口：元数据虽标 `firstMessage.reached: true`，该条 y=45.42、高 72.75，而滚动可视区顶为 73.58；这只能证明部分相交，不能证明完整回到开头。可能是滚动取证过早结束，尚不能认定 UI 永久遮挡或 major。 | 1440 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-I01-history-1440-initial.png`）、390 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-I01-history-390-initial.png`） |
| I02-progress | 查看已确认设定与下一步；对话中中频；已确认的信息 → 下一步建议 → 返回对话 | 保留。桌面摘要与对话相邻，390 用底部面板完整显示六条事实及下一步，关闭入口明确。信息为短句与轻分隔线，不需先读说明墙；不把窄屏按需打开辅助摘要视为主任务隐藏。 | 1440 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-I02-progress-1440-initial.png`）、390 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-I02-progress-390-initial.png`） |
| I05-confirm | 审阅首版蓝图并调整或确认进入项目；每次建项低频且关键；故事核心 → 完整梗概 → 调整/确认 | 保留。1440 核心、梗概和进入项目的后果明确，正文是视觉主体；390 滚到结尾后最后一段及两项决策按钮完整可见，底栏没有盖住末段。长梗概需滚动符合阅读任务，固定底部决策避免读完再寻找入口。 | 1440 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-I05-confirm-1440-initial.png`）、390 last（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-I05-confirm-390-last.png`） |

## Nielsen 简表

“通过”限实看截图中可见的布局与文案，不代替实际交互验证。

| Heuristic | Finding（位置与结论） | Severity | Fix / 保留 |
|---|---|---|---|
| 1 系统状态可见 | 可见态通过：W07 当前章与模式、W08 已绑定和未应用、I02 已确认信息均有明示。 | — | 保留；运行中反馈不在本次证据内。 |
| 2 对应真实任务 | 通过：场景编号、人物字段、下一步方向、故事核心等与写作任务一致。 | — | 保留现有命名。 |
| 3 控制与退出 | 可见态通过：面板均见关闭入口，历史见回到底部，I05 可调整或确认。 | — | 不据截图声称 Escape、撤销或关闭逻辑已通过。 |
| 4 一致性与惯例 | 通过：暖色底面、线性图标、标题/分隔线、主按钮和窄屏面板延续同一语言。 | — | 保持暖色 iOS 与现有 Lucide；无 emoji、无新依赖。 |
| 5 错误预防 | 局部可见：I01 空输入发送呈禁用；W07 字数边界、W08 写入前审阅有入口。 | 未完整覆盖 | 未试非法字数、重复提交与真实写入，不宣称逻辑门禁通过。 |
| 6 识别优于记忆 | 通过：三个灵感方向、场景展开箭头、字段标签与输入位置直接可见。 | — | 保留；角色条露出下一项，是横向浏览线索。 |
| 7 操作效率 | 可见态基本成立：输入和决策按钮近任务结果，历史有回底部入口；I01-history 390 首条未完整呈现。 | 证据缺口，非已证实缺陷 | 不采信“部分相交=首条读全”；在整体结论中保留此限制。 |
| 8 简洁与注意力 | W08 1440 右栏顶部重复章节和绑定摘要，多占聊天高度。其他页面主次明确。 | minor | 若处理此轻微项，合并重复绑定摘要，保留一次当前章；无需重做聊天或布局。 |
| 9 错误诊断与恢复 | 未展示失败、重试或恢复态；报告 `errors: []` 仅表示该次流程未记录异常。 | 未覆盖 | 不据此判错误恢复通过或另行推定缺陷。 |
| 10 上下文帮助 | 通过：字数、场景生成、下一步和确认去向为简短局部说明，无顶部教程墙。 | — | 保留。 |

## 动态与取证边界

- 所看宽度为 1440 和 390，高度均为 823。对应元数据 `overflow: 0`、`errors: []`；图片中未见非预期水平溢出，但本审查没有重新执行 DOM 测量，也没有看 900/1024 或其他断点。
- W08 的 `runCoverage: idle only`：确认的是六条历史消息、最近回复和输入可见；发送、运行中、写入审阅、应用结果、取消和失败路径未验证。
- I05 的 `replayOnly: true`，终态结果为 `novel_blueprint`、id `1`、revision `3`；确认页是任务完成事件回放后的展示证据，不证明真实生成、确认持久化或进入项目调用已完成。
- I01-history 390 首条部分相交是唯一关键取证缺口。W09 的下部列表、I05 的梗概末段已实看；正常滚动中的顶部裁切、长正文和长档案不记为阻断。
- 键盘焦点、Escape、触屏横向人物切换、真实输入法/软键盘、焦点恢复、历史位置恢复、重复提交幂等、tooltip 延时、读屏、精确对比度、缩放及减少动画未通过静态图证明。这里统一限定结论，不按每个未见非关键状态另开任务。

优先结论：无已证实的阻断或 major 可交给主流程；仅保留 W08 的轻微精简建议及 I01-history 390 的证据限制。审查到此结束，不扩展页面或引入实现任务。
