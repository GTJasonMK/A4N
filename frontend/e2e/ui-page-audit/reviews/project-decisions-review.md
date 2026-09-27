> 原始审查记录：保留发现时的判断与证据边界。修复后的当前状态以[主台账](../../../UI_PAGE_AUDIT.md)为准；原文中的“待修/缺证”不自动代表未解决事项。

# 项目决策界面独立审图（完成）

结论：6 组代表态审查完成，实际打开 16 张。所看画面未发现 blocker / major；有 1 个 minor（E1）和 1 个输入末项证据缺口（L1），不判整套交互完整通过。

独立 judge：project_decisions_review。宽屏 1440×823、窄屏 390×823；每组含宽窄代表图，另用 4 张补足导出格式及世界观末项。按 `resume-novel-full-report.json` 的 stage 选图，测试记录只用于定位，不作为图片已看或交互已验证的证据。频率为按任务推定，未使用产品埋点。

| ID | 任务 / 推定频率 / 注意力 | 具体问题或保留原因 | 实际查看的图 |
| --- | --- | --- | --- |
| N01-export | 交付正文或结构数据 / 低频 / 格式、必要元数据、下载动作 | 五格式已实际看全：TXT/PDF 无多余作品信息，Markdown/EPUB/批量显示书名与作者；格式和下载文案一致；390 批量态的两种 JSON 入口完整。保留单一主按钮与按格式呈现字段。**minor E1**：“结构数据”折叠标题无展开箭头，容易当成静态分组；复用 Lucide 展开/收起指示即可。 | [TXT 宽](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N01-export-1440-format-txt.png)、[Markdown 窄](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N01-export-390-format-markdown.png)、[EPUB 宽](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N01-export-1440-format-epub.png)、[PDF 窄](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N01-export-390-format-pdf.png)、[批量及 JSON 窄](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N01-export-390-last.png) |
| N01-lineage | 追查资产来源及输入 / 偶发 / 选中节点、来源、原始输入 | 宽图同时呈现选中蓝图、关系连线和相邻资产详情；所选节点强调清楚，来源、上游、媒体引用分节，空状态显式说明。画布横向滚动条属于局部图容器。390 选中“资产详情”，输入树的规则末行可读。**证据缺口 L1**：标为 last 的图停在 world setting 的 core rules、key locations #1 和 #2 开头；只能证明第二输入展开，不能证明整个输入树末项入图，不据此判 UI 截断或完整通过。 | [图与资产详情 宽](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N01-lineage-1440-asset-details.png)、[输入树 窄／末项未入图](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N01-lineage-390-last.png) |
| N01-versions + N01-version-restore | 选择版本并判断恢复影响 / 低频 / 版本说明、覆盖边界、退出 | 390 三条版本说明均完整换行，版本号、日期和当前版本可辨，修复了既有记录中的说明省略；宽屏恢复确认清楚写“恢复到版本 1”及“当前蓝图会被覆盖、版本仓库保留全部历史”，取消和恢复分开。保留现有层级与高风险确认。只审恢复弹窗外观，未执行恢复。 | [恢复确认 宽](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N01-version-restore-1440-initial.png)、[完整版本说明 窄](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N01-versions-390-initial.png) |
| N01-stage-flow + N01-rollback | 识别当前阶段、预览回滚影响 / 低频但高风险 / 当前态、目标态、受影响资料 | 宽图六阶段完整，“写作中”同时用粗体/下划线强调。390 回滚预览同时显示目标、回退路径、不可恢复提示、两组影响及数量；底部取消与红色确认回滚不混淆，历史记录另行折叠。保留按需展开流程和危险动作独立预览；截图含模拟资料说明，未以此推断真实删除结果。 | [六阶段流程 宽](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N01-stage-flow-1440-initial.png)、[回滚影响 窄](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N01-rollback-390-impact-preview.png) |
| N02-detail | 核对完整世界设定 / 创作中反复 / 地点、阵营、JSON 末项 | 宽图后半地点“旧车站、后巷、修表铺旧址、沿海邮路档案室”及末项档案号 66 可读；窄图三阵营“书店守望者、旧站调查者、沿海邮路见证人”及全部所列成员完整；JSON 的 timeline、knowledge_boundaries、visual_motifs、末项 premise 和闭合括号确实入图。保留宽屏双列、窄屏单列及固定标题。新图不包含前两个地点，前段仅沿用既有初态记录，不声称本次逐项重审了六个地点。 | [地点末项 宽](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N02-detail-1440-key_locations-last.png)、[全部阵营 窄](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N02-detail-390-factions-last.png)、[JSON 末项 窄](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N02-detail-390-last.png) |
| N01-members | 管理协作者权限 / 低频 / 姓名、角色全名、邀请状态 | 两尺寸均能读全“所有者、编辑者、只读成员”；390 的角色控件与行操作不挤压姓名，待处理邀请另组显示，三成员与两邀请清楚。邀请表单未填时按钮灰显。既有“只读…”省略问题在本次窄图中已消失，保留当前宽度与排布。 | [成员角色 宽](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N01-members-1440-initial.png)、[成员角色 窄](../../../../.dev-runtime/ui-pages/novel/resume-novel-full-N01-members-390-initial.png) |

其他 N01/N02 既有初态继续引用 [novel-materials-independent-review.md](../../../../.dev-runtime/ui-pages/novel-materials-independent-review.md) 的明确已看记录；该文件标为未看或待补证的内容不自动通过，也不计为本次新审图。

| Nielsen 启发式 | 所看画面的结论与定位 | 严重度 | 建议 |
| --- | --- | --- | --- |
| 1 状态可见 | 格式名、当前“写作中”、选中资产、角色与邀请状态可辨；慢操作中间态未审。 | 可见态通过 | 保留状态文字，不仅依赖颜色。 |
| 2 符合现实语言 | 下载格式、恢复版本、回退路径和资料分组使用任务词；原始字段集中在输入树/JSON 检视。 | 可见态通过 | 保留原始数据检视的技术字段。 |
| 3 控制与退出 | 弹窗有关闭入口，恢复/回滚有取消；行为与焦点返回未复测。 | 可见态通过 | 不据静态图推断 Escape 或取消请求行为。 |
| 4 一致性 | 暖色表面、橙色一般动作、红色回滚、Lucide 图标及窄屏底部面板一致。 | 可见态通过 | 沿用现有视觉语言。 |
| 5 错误预防 | 恢复说明覆盖边界，回滚说明不可恢复及影响组，空邀请按钮灰显。 | 可见态通过 | 实际权限/并发/重复提交不在图审结论内。 |
| 6 识别优于记忆 | N01-export 的“结构数据”闭合栏缺少展开指示（E1）。 | minor | 使用现有 Lucide 箭头显示展开/收起状态。 |
| 7 效率与灵活性 | 元数据按格式出现；资产详情邻接关系图；版本说明可直接比较。 | 可见态通过 | 保留按需展开；键盘效率未评。 |
| 8 简洁与主次 | 下载、版本内容、回滚影响及成员角色各有明确层级，390 未见按钮遮盖正文。 | 可见态通过 | 不增加装饰或额外容器。 |
| 9 错误识别与恢复 | 本组代表图没有失败后的真实错误状态。 | 未评 | 不将 112 条自动检查全绿替代错误态图审。 |
| 10 就地帮助 | 格式用途、必填信息和危险动作说明贴近控件，无独立说明墙。 | 可见态通过 | 保留短说明。 |

处理顺序：先补 L1 的输入树真正末项证据，再处理 E1 的轻微展开提示。N02 前两个地点本次未重新入图，已有初态记录没有逐项名称，因此本次不宣称六个地点全部逐项独立复审。暖色 iOS、Lucide 保持现状，不新增 emoji 或依赖。

边界：本次只读已导出图片，未启动服务或修改源码/测试，也未执行恢复、回滚、邀请或下载。未复测键盘、读屏、hover、缩放、精确对比度与 DOM 溢出；900 宽及同一交互未选中的镜像尺寸不冒充已看。16 张已达到任务上限，审图至此停止。
