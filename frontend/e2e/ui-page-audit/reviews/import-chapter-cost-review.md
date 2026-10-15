> 原始审查记录：保留发现时的判断与证据边界。修复后的当前状态以[主台账](../../../UI_PAGE_AUDIT.md)为准；原文中的“待修/缺证”不自动代表未解决事项。

# 导入、章节与成本：独立图审

范围严格限定为 N10-review-detail、N10-consolidate-blueprint、N10-consolidate-outline、N10-consolidate-part、N11-chapters、N11-import、N12-cost。审查对象是已有实际截图与 `../novel/resume-novel-full-report.json` 轨迹；未改源码、测试或启动服务。频率是基于写作任务的推定，不是使用统计。保留现有暖色 iOS、Lucide 图标语言，不建议新依赖。

7 项的 1440 / 390 关键态已查看，共 16 张图（其中候选详情补看两尺寸末态）。发现 **1 项 major：关闭项目上下文后蓝图说明仍宣称使用项目资料；1 项 minor：候选编辑提交缺少禁用原因**。未见已证实的操作阻断。major 已即时发给主流程。上游 112 项自动检查通过不代替图审，也不把正常长内容滚动视作缺陷。

| 项目 | 任务 / 推定频率 / 注意力 | 图像判断与保留理由 | 实际截图（相对本文件） |
| --- | --- | --- | --- |
| N10-review-detail | 核对来源、摘要与结构后接受候选 / 每次批量导入中频 / 内容核验高注意力 | 桌面队列与详情分栏；移动端详情完整占宽。文件名、等待状态、章节号和候选说明能建立审核上下文；末态的结构编辑区与底部按钮均可到达，保留“摘要→高级编辑”的层级。**minor F1：两尺寸末态的“保存并接受”灰化，却没有就近说明原因；移动末态同时显示“等待中”，容易让人不知道还需什么条件。** 建议在该操作旁显示实际禁用原因。不能据此推定队列菜单也不能接受候选。 | 1440 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N10-review-detail-1440-initial.png`）；390 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N10-review-detail-390-initial.png`）；1440 last（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N10-review-detail-1440-last.png`）；390 last（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N10-review-detail-390-last.png`） |
| N10-consolidate-blueprint | 用当前资料与人工输入归纳正式蓝图 / 低频里程碑操作 / 写入前高注意力 | **major F2：双尺寸“当前项目资料”开关关闭，但正文仍写“依据当前正文、简介、大纲和正式资料归纳蓝图”；轨迹同时记录 `contextFromEnteredContent: true`。** 用户可能误以为仍会读取完整项目上下文。建议关闭时改为说明依据输入内容，开启时保留当前说明；不需要修改生成流程。直接写入正式蓝图及失败保留的后果、主按钮文字一致，移动布局无碰撞；这些部分保留。 | 1440 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N10-consolidate-blueprint-1440-initial.png`）；390 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N10-consolidate-blueprint-390-initial.png`） |
| N10-consolidate-outline | 生成章节大纲候选并在确认后写入 / 低频结构调整 / 高注意力 | 提示明确“先生成可检查的归纳结果，确认后才更新正式大纲”，按钮为“生成章节大纲候选”；与蓝图直接更新的动作有可见区别。390 的两行说明、模式开关、输入框及完整按钮文字能同时辨认，无需新增说明墙。保留当前结构。 | 1440 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N10-consolidate-outline-1440-initial.png`）；390 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N10-consolidate-outline-390-initial.png`） |
| N10-consolidate-part | 按分部粒度生成大纲候选 / 长篇规划低频 / 高注意力 | “分部大纲”选择与“生成分部大纲候选”按钮对应，延用确认后更新的说明；1440 与 390 都没有误写成章节大纲或正式直写。共享结构降低三种归纳任务的学习成本，保留相同布局与操作位置。 | 1440 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N10-consolidate-part-1440-initial.png`）；390 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N10-consolidate-part-390-initial.png`） |
| N11-chapters | 找到章节并进入正文 / 日常高频 / 扫描时低至中注意力 | 3 行列表按章节号、标题、字数排序呈现，整行方向箭头明确表达进入；顶部总章数/字数与独立的“导入章节”入口可见。两尺寸分隔线、行高与字重层级稳定，移动端无需横滚读取章节身份。仅 3 章形成的留白合理，不要求为了填满页面增加装饰或工具。 | 1440 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N11-chapters-1440-initial.png`）；390 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N11-chapters-390-initial.png`） |
| N11-import | 粘贴外部正文并追加章节 / 低至中频 / 确认编号与正文时高注意力 | 现有 3 章时编号预填 4；标题显式标为可选，正文占主要面积。空正文的确认按钮灰化，与空输入状态一致。桌面居中弹窗转为移动底部面板，390 下标签、输入与整宽确认按钮全部可见，关闭图标独立且未被标题挤压。保留直接表单。 | 1440 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N11-import-1440-initial.png`）；390 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N11-import-390-initial.png`） |
| N12-cost | 核对模型调用金额、定价置信度与失败记录 / 定期低至中频 / 财务核验高注意力 | 桌面按 USD / CNY 分开汇总，明确“未配置价格不会按免费计算”；金额、成功/失败与实际用量/估算/未定价分列，底部可见失败行仍保留金额。390 末态可读 Provider/模型与时间，是轨迹面向最后一行最后一列的终点；结合初态 `priorityColumns` 为金额和状态，不能把横移后的右侧列当成金额丢失。保留表格，不因记录较长新增分页或卡片要求；本轮未目视移动初态汇总。 | 1440 initial（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N12-cost-1440-initial.png`）；390 last（证据文件缺失：`.dev-runtime/ui-pages/novel/resume-novel-full-N12-cost-390-last.png`） |

## Nielsen 简表

“图内通过”仅代表所看图像未发现该项问题，不等于动态功能通过。

| 准则 | 判断与位置 | 严重度 | 最小处置 |
| --- | --- | --- | --- |
| 1 系统状态可见 | 候选有等待状态，成本区分结果与定价置信度；候选末态的禁用按钮无原因（F1）。 | minor | 由现有禁用条件就近显示原因；不要猜测成“加载中”。 |
| 2 符合真实任务 | 正式蓝图输入来源的说明与关闭的项目上下文开关不一致（F2）；“正式蓝图/大纲候选/确认导入”其余动作语义明确。 | major | 让输入来源说明跟随现有开关；保留写入后果说明。 |
| 3 用户控制与退出 | 导入弹窗的关闭入口可见，工具区可折叠；未执行关闭、取消或撤销。 | 图内通过 / 动态未测 | 保留现有可见出口，不从截图保证 Escape 或撤销。 |
| 4 一致性与标准 | 三种归纳沿用同一选择器、上下文开关、编辑区和底部主操作；暖色面板、细分隔和线性图标一致。 | 图内通过 | 保留既有 iOS 风格与 Lucide，不引入第二套组件。 |
| 5 防错 | 空正文导入灰化、标题可选、蓝图直写后果明确；真实编号冲突、JSON 错误、异步写入未执行。 | 图内通过 / 动态未测 | 本轮不推断校验与保存安全性，也不把未拍状态都列为缺陷。 |
| 6 识别优于记忆 | 章节号/标题/字数、候选文件名/摘要、归纳对象和完整按钮标签可见；F1 仍使用户猜禁用条件。 | minor（同 F1） | 复用 F1 修正，不增加整页说明。 |
| 7 效率与灵活性 | 章节列表可直接扫读；手机主按钮 44px 高、章节行约 67px 高（轨迹值）。长输入在自身编辑区滚动合理。 | 图内通过 | 保留现有密度；不为 3 章列表增加过滤器或填充内容。 |
| 8 简约与注意力 | 章节列表让内容领先，导入正文占主要面积；归纳用单一强调动作区分写入和候选。 | 图内通过 | 保留现有层级，不因留白增加装饰。 |
| 9 错误识别与恢复 | 成本失败行可识别；这些图未展示导入或归纳错误反馈。 | 结果可识别 / 恢复未测 | 不宣称失败恢复已验证，也不将其列成这轮新增任务。 |
| 10 情境帮助 | 标题可选、导入后标为外部来源、候选确认后才更新等说明贴近任务；F2 的输入来源说明需准确。 | major（同 F2） | 修正 F2 原句，不再叠加一层提示。 |

优先顺序：F2（输入来源可能误导正式写入判断）→ F1（禁用原因不清）。仅提出这两项；未改源码，未宣称已修复。

## 轨迹与动态边界

- 七项在 1440×823、390×823 的 initial/last 共 28 条元数据均记录页面级 `overflow: 0`、空 `unexpected` 与 `errors`；其中实际打开图片 16 张。页面级零溢出不能证明表格每列同时可见，或证明局部滚动没有缺陷。
- N10 候选结构记录 `reviewJsonLength: 620`，390 末态的提交按钮由一次原生滚轮操作到达、`enabled: false`；未编辑结构或接受候选。三种归纳均记录 `contextFromEnteredContent: true`、`generationSubmitted: false`，只证明人工输入模式与生成入口状态，不能证明生成或写入成功。
- N11 轨迹只含章节读取与导入字段发现：移动确认按钮位于 y=766、44px 高，空输入时禁用。没有提交导入或验证进入正文、返回后的滚动位置。
- N12 初态记录返回 60 行、DOM 中 60 行，以及优先列为金额和状态；字段名 `visibleRows` 实际是 DOM 行数，不代表 60 行同时在屏。390 末态以 6 次原生滚轮操作到达末行时间 `09/25 08:00`；用例目标为最后一行最后一列。保留正常表格滚动，本轮未目视 390 初态、未核验真实账单或汇率计算。
- 本轮没有重启服务或运行浏览器；审查输入为既有实际浏览器截图、轨迹及对应捕获用例。未验证键盘遍历、焦点环/焦点归还、Escape、屏幕阅读器、软件键盘压缩视口、hover/pressed、加载/错误恢复、提交幂等或数据库效果。这些是证据边界，不自动扩成待办清单。

落盘节奏：4 图、8 图、12 图、16 图各写入一次，再补齐轨迹核对与本表。仅维护本报告。

## 当前分支收敛（2026-10-12）

- **F2 蓝图归纳说明：已关闭。** 当前 `ImportToolsPanel` 按 `reverseAutoContext` 切换项目资料说明与样本文本说明；既有 N10 浏览器路径检查开关两种状态和来回切换，并断言样本输入保持。没有发起生成或修改正式蓝图。
- **F1 候选提交原因：已关闭。** 当前审核详情在“保存并接受”禁用时就近显示原因；三宽度记录及现有交互断言覆盖该状态，没有改变接受条件。
- 本文件前文保留原始发现与证据边界；当前状态见 [主台账](../../../UI_PAGE_AUDIT.md) 的 N10 记录。
