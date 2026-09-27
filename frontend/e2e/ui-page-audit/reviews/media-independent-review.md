> 原始审查记录：保留发现时的判断与证据边界。修复后的当前状态以[主台账](../../../UI_PAGE_AUDIT.md)为准；原文中的“待修/缺证”不自动代表未解决事项。

# 媒体工作面独立审查收尾（2026-09-26）

审查者 `/root/media_review_record`；只写本报告，不参与生产 UI、测试或服务修改。本次亲看 16 张新版截图，指定遗留问题及画布锁图标填充均已核销，未发现新的 blocker/major。跨页草稿、时间线选区/键盘微调、三类来源冲突、Galgame 末行分支在所看工作面均可读可操作。此结论不等于 96 个 ID 均由本代理逐状态亲看；逐 ID 证据和未核销范围另列。

## 证据来源与边界

- 原审查 A：[independent-review.md](../../../../.dev-runtime/ui-pages/media/independent-review.md)，由 `/root/media_page_review` 亲看 39 张 `after-media-layout` 图片。
- 原审查 B：[media-final-review.md](../../../../.dev-runtime/ui-pages/media/media-final-review.md)，同一独立审查者亲看 92 张新增图片；其历史失败及待复验不因本文自动变为通过。
- 继承口述 C：主流程本次交接明确转述前任已经亲看 MX24–MX30 新属性；MX09–MX12 的 `resume-details` 宽屏/390 属性图（MX12 另有 1024）；MX19 的 `resume-edge` 1440/390 两端节点标题完整且端口保留。这些是前任实看证据，不计入本审查者亲看张数。
- 本次 D：按每个 ID/宽度最新成功 report 与文件修改时间选图，最多 16 张；每 2–4 张即落盘。2026-09-25 的 `media-closeout` 仍是部分工作面的最新成功批次，并非假称 2026-09-26 新截图。
- 功能 E：读取相应 JSON 的实际 `functional`、交互轨迹、overflow、errors、unexpected；功能通过与视觉审查分开。主流程交接的视频模式 20/20、scrub 13 通过/1 原生专用跳过另属独立回归，不冒领为本代理执行。
- 资产为有真实文本和关系的本地 SVG/音视频审查样本，可判断比例、内容可达性、控制层级及状态，不评价艺术成片质量。本报告不证明真实上传、模型请求、后端幂等或生产数据写回。

## 任务、频率与注意力分配

频率来自台账的任务推定，没有用户遥测。漫画选页/读图/改提示词、视频素材到时间线、Galgame 台词与分支、画布选节点/查属性均按高频处理：正文、图像或可操作时间线应承担主要视觉权重；进度、来源、版本仅提供必要上下文。来源冲突、重置按低频但后果较大的任务处理，比较内容、影响范围与取消优先于紧凑。新模板/空输入/输出节点按中频处理，不隐藏空引用、未运行或未保存状态。

保留已有暖色/iOS 风格、语义状态色、Lucide 描边图标和统一圆角，不引入新依赖或 emoji；这些是本次审查约束，非凭截图声称已扫描全部源码。

## 本次亲看记录

| 序号 / ID | 实屏与具体判断 | 证据 | 结论 |
| --- | --- | --- | --- |
| 1 / MC01 | 390 空态已写“选择一章查看漫画详情”，不再指错“左侧”。顶部三章横轨露出第三张卡片，已完成与第 2 章未完状态有图标、文字和进度；当前未选择章时保留空白工作面合理。 | [390](../../../../.dev-runtime/ui-pages/media/media-closeout-MC01-390.png) | 原方向文案缺陷关闭；未以空态代替有内容页。 |
| 2–3 / MC06 | 1440、390 均显示纵图全高、底部 `SCENE 20001 / UI AUDIT` 标记及四边；100% 时不被舞台/弹窗裁切。关闭、缩放/重置位于固定外框，宽屏黑色侧带属于纵图 contain，手机仅有短上下边。 | [1440](../../../../.dev-runtime/ui-pages/media/media-closeout-MC06-1440.png)、[390](../../../../.dev-runtime/ui-pages/media/media-closeout-MC06-390.png) | 原纵图裁切 major 关闭。JSON 另记录 100% 初始/重置 insideStage、insideBody 均 true，放大和 Escape 焦点返回通过。 |
| 4 / MC13 | 390 返回第 3 页后，“未保存修改”、选中的“提示词与参考图”、完整编辑面与取消/保存同时可辨；文本末尾出现“第3页保留的审查草稿”。顶部原文/生成信息经真实滚动让出编辑空间。 | [390 交互后](../../../../.dev-runtime/ui-pages/media/media-closeout-MC13-390-interaction.png) | 阅读意图回退已有新版视觉证据；跨页全文/草稿保持依赖该 ID 三宽度功能轨迹，不凭单张图片证明切页过程。 |
| 5 / MV01 | 390 素材摘要独占横向一行，补缺失/更新过期素材另行排列；不再被两按钮夹成约 36px 宽竖排。筛选、生成设置与首张已有图依次出现，状态/数量可扫读。 | [390](../../../../.dev-runtime/ui-pages/media/media-closeout-MV01-390.png) | 原摘要挤压关闭；MV02 同一摘要外框可复用此结论，设置字段另沿用其实际功能结果。 |
| 6–7 / MV31 | 900 预览与下方完整剪辑条同屏，390 预览约 177px 高，底部没有大块无内容黑区，完整剪辑条仍进入屏内。已同意/待确认批次、当前版本、同意推送分层明确。 | [900](../../../../.dev-runtime/ui-pages/media/media-closeout-MV31-900.png)、[390](../../../../.dev-runtime/ui-pages/media/media-closeout-MV31-390.png) | 原小/中屏预览过高 major 关闭；不把正常视频黑边视为裁切。 |
| 8 / MV32 | 1440 单个 2.50 秒标记在时间条上可见，预览同步到 2 秒附近；“插入位置 2.50 秒”与“上传并插入”紧邻时间条，动作与选区语义一致。48px 高条用于时间点选择足够，未假装多轨剪辑器。 | [1440](../../../../.dev-runtime/ui-pages/media/media-closeout-MV32-1440.png) | 插入点状态无新阻断；实际点击与三宽度可达性依据 E。 |
| 9 / MV33 | 390 双标记及其间选区高亮清楚，2.49–7.51 秒与 5.02 秒总长完整显示；“上传并替换”另行安置，不撞时间读数。通过真实滚动后整条时间线与动作同屏。 | [390 最新成功补跑](../../../../.dev-runtime/ui-pages/media/final-extras-MV33-390.png) | 替代 `media-closeout` 本 ID/390 的历史失败；不是把旧失败忽略。 |
| 10 / MV34 | 900 标记具有可见键盘焦点框，插入位置变为 2.56 秒；预览、时间条、上传入口不重叠。 | [900 微调后](../../../../.dev-runtime/ui-pages/media/media-closeout-MV34-900-interaction.png) | E 记录 2.51→2.56 秒，视觉与实际方向键结果一致；无新阻断。 |
| 11 / MS01 | 1440 漫画来源弹窗第二项标题明确为“生成内容”，正文仍是“第3页漫画分镜”；来源与当前两列对照，需确认/阻塞/普通更新各自有文字状态。底部固定操作未覆盖当前可读行。 | [1440 第 2 条](../../../../.dev-runtime/ui-pages/media/media-closeout-MS01-1440-conflict-2.png) | 漫画误称视频片段关闭；保留真实生成依赖信息。 |
| 12 / MS02 | 390 视频来源第二项仍写“片段1 · 街口雨幕”，上下排列的来源与本地全文均可读；阻塞原因、保留当前及固定底栏有清楚位置。低频比对采用纵向堆叠合理。 | [390 第 2 条](../../../../.dev-runtime/ui-pages/media/media-closeout-MS02-390-conflict-2.png) | 当前项目身份与领域内容匹配；三冲突滚动及关闭无应用/解绑由三宽度 E 证明。 |
| 13 / MS03 | 390 交互叙事来源第二项为“GALGame 场次”，标题是“铜铃之后的来信”；与漫画/视频共享比较结构，同时保留场次领域名与阻塞原因。来源、本地、后续物品项及退出入口均可读。 | [390 第 2 条](../../../../.dev-runtime/ui-pages/media/media-closeout-MS03-390-conflict-2.png) | 共用组件未把 Galgame 场次误改为视频片段；未执行应用。 |
| 14–15 / MG06 | 900 第 8 行两种选择与目标并排，390 改为单列；两种选择、支线回到场次 3 的说明、主线直达说明与“保存选择支”全部实际进入视口。900 外层/内层滚动交接没有挡住末行。 | [900](../../../../.dev-runtime/ui-pages/media/media-details-complete-MG06-900.png)、[390](../../../../.dev-runtime/ui-pages/media/media-details-complete-MG06-390.png) | 末行可达遗漏关闭；保留“选择支目标可验收”和两条路径，不以省高度删除业务说明。 |
| 16 / MX23 | 390 锁定图标为清楚的 Lucide 描边锁，未被 SVG 默认填充涂实；放大/缩小/适配/锁定四个外部控件可辨。fit 后节点缩到 0.35，屏外仍有节点，是原缩放边界，不当作可读编辑视角。 | [390 锁定](../../../../.dev-runtime/ui-pages/media/media-closeout-MX23-390-locked.png) | SVG 填充问题关闭；E 记录 fit 后 9/17 节点在视口、解锁恢复 draggable=17。 |

## 继承核销记录

| ID | 前任已明确确认的范围 | 可追溯证据 | 本代理边界 |
| --- | --- | --- | --- |
| MX24–MX30 | 七类新增属性均无阻断；完整模板、图片/音频输出、实际 3 秒音频引用及本地空输入/探针/输出草稿保留真实业务状态。 | [属性主批](../../../../.dev-runtime/ui-pages/media/canvas-attributes-final-report.json)、[1024 补跑](../../../../.dev-runtime/ui-pages/media/canvas-attributes-closeout-report.json)、[详情补全](../../../../.dev-runtime/ui-pages/media/media-details-complete-report.json)；同名 PNG / `-interaction.png` | 继承 C；未把每个宽度都计为本代理亲看。 |
| MX09–MX12 | 宽屏与 390 的内容返回属性本体已看；MX12 另看 1024，原仅看内容未看属性的遗漏关闭。 | [resume-details 报告](../../../../.dev-runtime/ui-pages/media/resume-details-report.json)；`../media/resume-details-MX{09,10,11,12}-{1440,390}-properties.png`，MX12 另有 1024 | 继承 C；功能 12/12 通过另由 E 核对。 |
| MX19 | 1440/390 两端节点标题完整；context/text/input 端口仍可辨，标题截断 major 关闭。 | [1440](../../../../.dev-runtime/ui-pages/media/resume-edge-MX19-1440.png)、[390](../../../../.dev-runtime/ui-pages/media/resume-edge-MX19-390.png)、[功能报告](../../../../.dev-runtime/ui-pages/media/resume-edge-report.json) | 继承 C；三宽度 SVG 点击通过不等同 1024 也被本代理亲看。 |

## 完整 ID 覆盖表

以下恰好覆盖台账中的 96 个 ID：MC 13、MV 37、MG 13、MX 30、MS 3。台账没有 MC12，不补造这个编号。表中 A/B/C/D/E 对应前述证据来源；“同态”只复用已实际看过的同一组件/布局规则，不能据此冒称该 ID 的每种数据、每个宽度也逐图看过。“仅 E”保留功能结论和专用截图路径，视觉证据不足另列。

本文汇总时，各 `(ID, width)` 取最后一次实际运行结果（包含失败，不先过滤失败）后，共 **284 项，284 passed，overflow 全为 0，errors/unexpected 合计均为 0**。这是跨批次合并的最新矩阵，不是一次 284 项的新运行。除 MX13 仅 1440、MX14 仅 390 外，每个 ID 有三宽度结果；普通媒体为 1440/900/390，画布为 1440/1024/390。原始失败留在原报告中，只有同一键的后续成功才替代。

### 漫画（13 ID）

| ID | 任务 / 频率推定 | 注意力、保留理由与具体结论 | 证据与覆盖方式 |
| --- | --- | --- | --- |
| MC01 | 选择来源章 / 高 | 状态与横轨优先，未选择时保留工作面；错误方向文案已关。 | D1；[最新三宽度](../../../../.dev-runtime/ui-pages/media/media-closeout-report.json)。 |
| MC02、MC03 | 读整页提示词/参考；编辑取消保存 / 高 | 1440 图文并排，900 单列，390 标题/菜单同排；编辑保留完整长文本和动作。 | B“漫画正文”；[MC02 390](../../../../.dev-runtime/ui-pages/media/media-final-MC02-390.png)、[MC03 390](../../../../.dev-runtime/ui-pages/media/media-final-MC03-390.png) 为前任亲看；当前 `media-closeout` E 通过。 |
| MC04 | 处理缺角色参考 / 高 | 已绑/缺失可辨，保留禁用与原因，不显示假生成成功。 | A 对应三宽度全看；[390](../../../../.dev-runtime/ui-pages/media/after-media-layout-MC04-390.png)；`media-final` E。 |
| MC05 | 页内成图与尺寸 / 高 | 图像应主导阅读、尺寸作辅助；灯箱全比例由 MC06 覆盖，但页内布局不能用灯箱替代。 | 仅 E：`media-final` 三宽度通过；[页内专用图](../../../../.dev-runtime/ui-pages/media/media-final-MC05-390.png) 尚无 A/B 明确亲看记录，视觉待补。 |
| MC06 | 灯箱查看、缩放、重置返回 / 高 | 保留完整纵图和外部控制，裁切 major 已关。 | D2–3；[三宽度轨迹](../../../../.dev-runtime/ui-pages/media/media-closeout-report.json)。 |
| MC07 | 失败定位与续跑入口 / 高 | 失败原因与继续生成同时保留。 | B“漫画失败/取消”亲看 [1440](../../../../.dev-runtime/ui-pages/media/media-final-MC07-1440.png)、[390](../../../../.dev-runtime/ui-pages/media/media-final-MC07-390.png)；E 三宽度。 |
| MC08 | 横轨到第 6 页 / 高 | 同一页轨与页正文规则；末页过期信息保留，不只验证点击命中。 | 同态 B/MC02；E 明确 `selectedPage=6`、`promptId=21006`、`currentTextMatches=true`；[专用图](../../../../.dev-runtime/ui-pages/media/media-final-MC08-390.png) 未计入 D。 |
| MC09 | 漫画图片请求预览 / 高 | 共享请求预览的阅读/退出壳体；请求仅预览，不执行生成。 | 同态 B/MV26；源码调用同一个 `RequestPreviewModal`，`media-final` E 三宽度；[漫画请求](../../../../.dev-runtime/ui-pages/media/media-final-MC09-390.png) 未单独亲看正文。 |
| MC10、MC11 | 优化指令后取消；删除确认后取消 / 高 | 目标页与影响范围明确，空指令禁用；取消保留六页列表与第 3 页全文。 | B 亲看 [优化](../../../../.dev-runtime/ui-pages/media/media-final-MC10-390.png)、[删除](../../../../.dev-runtime/ui-pages/media/media-final-MC11-390.png)；E 三宽度，取消无优化/删除命令。 |
| MC13、MC14 | 跨页草稿；保存中切页 / 高 | 当前页阅读意图与草稿不能被另一页响应清理；可见未保存状态保留。 | D4 亲看 MC13；E：MC13 `fullTextRetained=true`，MC14 保存 21003、保留 21005、`fullTextsMatch=true`；MC14 只复用相同编辑面，不冒称亲看异步全过程。 |

### 视频（37 ID）

| ID | 任务 / 频率推定 | 注意力、保留理由与具体结论 | 证据与覆盖方式 |
| --- | --- | --- | --- |
| MV01、MV02 | 素材状态/统计；生成设置 / 高 | 摘要独占一行，批量动作紧邻，设置渐进展开；旧竖排统计已关。 | D5/MV01；MV02 共享同一摘要布局，`media-closeout` E 三宽度；设置专用状态不计 D。 |
| MV03 | 缺失素材筛选 / 高 | 复用素材列表/筛选规则，缺失与过期不合并成同一种状态。 | 同态 D5/MV01；`media-final` E 三宽度；[筛选专用图](../../../../.dev-runtime/ui-pages/media/media-final-MV03-390.png) 未另亲看。 |
| MV04 | 上传前归属与主素材 / 高 | 归属、开关、文件入口先于提交；没有真实上传。 | B“素材上传”亲看 [390](../../../../.dev-runtime/ui-pages/media/media-final-MV04-390.png)；E 三宽度。 |
| MV05、MV06、MV07 | 图片概览、来源、管理 / 高 | 概览重图文，来源保留配置/快照，管理把危险操作折叠；各模式没有共用空内容。 | B 分别亲看 [概览](../../../../.dev-runtime/ui-pages/media/media-final-MV05-390.png)、[来源](../../../../.dev-runtime/ui-pages/media/media-final-MV06-390.png)、[管理](../../../../.dev-runtime/ui-pages/media/media-final-MV07-390.png)；E 三宽度。 |
| MV08、MV09、MV10 | 规划阅读、参数、完整编辑 / 高 | 长摘要可扫描；比例/语言/时长和空间锚点保留，重新规划与编辑区分。 | B 亲看各工作面，MV10 三宽度；[规划](../../../../.dev-runtime/ui-pages/media/media-final-MV08-1440.png)、[参数](../../../../.dev-runtime/ui-pages/media/media-final-MV09-390.png)、[编辑](../../../../.dev-runtime/ui-pages/media/media-final-MV10-390.png)；E。 |
| MV11 | 片段与采用结果总览 / 高 | 六片段及原生/插入、已采用状态明确；手机用工作面切换保留播放器空间。 | B 亲看 [1440](../../../../.dev-runtime/ui-pages/media/media-final-MV11-1440.png)、[390](../../../../.dev-runtime/ui-pages/media/media-final-MV11-390.png)；E。 |
| MV12、MV13、MV14、MV15、MV16 | 首尾帧、续接、分镜、参考媒体、Chat / 高 | 五模式共享配置框但输入不同；图文在 900 并排，手机真实滚到末尾限制项；不以一个首屏替代五模式。 | A/B 亲看相应素材及首屏；MV12/MV15 的末项为 B 亲看，MV13/MV14/MV16 的后补末项为 E：`details-closeout` / `media-closeout`，`finalFieldReachable=true`。[分镜末项](../../../../.dev-runtime/ui-pages/media/details-closeout-MV14-390-interaction.png)、[Chat 末项](../../../../.dev-runtime/ui-pages/media/media-closeout-MV16-390-interaction.png) 不是 D 亲看。 |
| MV17 | 尚未准备首尾帧 / 高 | 缺帧与禁用最终生成应保留；同句准备提示仍重复，属于未关闭 minor。 | A 已亲看三宽度待准备态；[最新文本/状态](../../../../.dev-runtime/ui-pages/media/details-closeout-MV17-390.json) 仍有两次相同提示；本次未追加图片核销。 |
| MV18、MV19、MV20、MV21 | 首帧、尾帧、续接来源、分镜板配置 / 高 | 共用配置动作与完整比例；390 底栏已压至约 121px，不再挤掉正文。续接仍须能辨来源关系。 | A/B 亲看 MV18/MV21；[首帧](../../../../.dev-runtime/ui-pages/media/media-final-MV18-390.png)、[分镜板](../../../../.dev-runtime/ui-pages/media/media-final-MV21-390.png)。MV19/MV20 仅共享外框规则与 E 内容断言，未冒称尾帧/来源图逐项亲看。 |
| MV22 | 参考库选择与加入 / 高 | 已选/素材库/生成参考区分，图像、摘要与来源同时保留。 | B 亲看 [390](../../../../.dev-runtime/ui-pages/media/media-final-MV22-390.png)；E 三宽度。 |
| MV23、MV24、MV25、MV26 | 播放、提示词查看/编辑、模型请求 / 高 | 播放器与当前记录一致；长正/负提示词及复制/关闭入口保留。 | B 分别亲看 [播放后](../../../../.dev-runtime/ui-pages/media/media-final-MV23-390-interaction.png)、[查看](../../../../.dev-runtime/ui-pages/media/media-final-MV24-390.png)、[编辑](../../../../.dev-runtime/ui-pages/media/media-final-MV25-390.png)、[请求](../../../../.dev-runtime/ui-pages/media/media-final-MV26-390.png)；E。 |
| MV27、MV28、MV29 | 插入创建、图主体选择、已有详情 / 高 | 创建与已有内容分开；移动位置与移除/删除动作清楚，图片主体用于选择。 | B 亲看 [创建](../../../../.dev-runtime/ui-pages/media/media-final-MV27-390.png)、[详情](../../../../.dev-runtime/ui-pages/media/media-final-MV29-390.png)；MV28 同一参考网格规则，E `imageClickSelectedReference=true`。 |
| MV30 | 合成顺序与已推送前缀 / 高 | 四片段/24秒和已推送范围明确；宽屏并排、手机堆叠。 | B 亲看 [1440](../../../../.dev-runtime/ui-pages/media/media-final-MV30-1440.png)、[390](../../../../.dev-runtime/ui-pages/media/media-final-MV30-390.png)；E 实际播放。 |
| MV31、MV32、MV33、MV34 | 批次、插入点、替换选区、键盘微调 / 高 | 保留批次/版本、选区总长及近处主动作；小中屏预览挤压已关，焦点清楚。 | D6–10；`media-closeout` 与 `final-extras` 三宽度 E；最新失败已由同键补跑替代。 |
| MV35、MV36、MV37 | 图主体配置、独立预览、预览不改选择 / 高 | 选择与放大分别保留；不让预览副作用替换选择。 | 同态 B/MV18、MV22、MV27；E 三宽度，MV36 `returnedToSegmentConfig=true`，MV37 `selectionUnchanged=true`；独立灯箱未计入 D 亲看数。 |

### Galgame（13 ID）

| ID | 任务 / 频率推定 | 注意力、保留理由与具体结论 | 证据与覆盖方式 |
| --- | --- | --- | --- |
| MG01 | 三场次与恢复入口 / 高 | 场次、失败原因和已保留结果均可辨。 | B 亲看 [390](../../../../.dev-runtime/ui-pages/media/media-final-MG01-390.png)；E 三宽度。 |
| MG02、MG03 | 混合台词与背景绑定 / 高 | 台词优先；背景折叠回收空框，展开不拉长左栏，390 地点/时间并列。 | A/B 亲看 [MG02 1440](../../../../.dev-runtime/ui-pages/media/media-final-MG02-1440.png)、[MG02 390](../../../../.dev-runtime/ui-pages/media/media-final-MG02-390.png)、[MG03 390](../../../../.dev-runtime/ui-pages/media/media-final-MG03-390.png)；E。 |
| MG04、MG05 | 对白表情/姿态；独白/演出差异 / 高 | 共用台词行与演出渐进披露；五种行类型的字段差异不能删。 | 同态 A/B/MG02、MG07；E 每个 ID 三宽度真实滚到目标，`reachability.reached=true`；[对白](../../../../.dev-runtime/ui-pages/media/media-final-MG04-390.png)、[独白](../../../../.dev-runtime/ui-pages/media/media-final-MG05-390.png) 不冒称本次亲看。 |
| MG06 | 末行选择、目标与合流 / 高 | 两条路径及返回说明完整，保存就在路径末端；900/390 均能到达。 | D14–15；`media-details-complete` E 三宽度。 |
| MG07 | CG 与转场演出 / 高 | CG 描述及全部演出字段保留，按需展开。 | A 三宽度、B 390 亲看；[390](../../../../.dev-runtime/ui-pages/media/media-final-MG07-390.png)；E。 |
| MG08 | 展开媒体并播 BGM / 高 | 真实 3 秒音频，原生播放不伪装；该展开区与折叠台词不同。 | 仅 E：`media-closeout` 三宽度实际播放，390 `time=0.186529,duration=3`；[展开/播放](../../../../.dev-runtime/ui-pages/media/media-closeout-MG08-390-interaction.png) 未找到 A/B 明确亲看记录，视觉待补。 |
| MG09、MG10 | 资产计划、矩阵末项 / 高 | 6 背景、12 差分、2 CG 完整；矩阵有必要的局部横向滚动，固定底栏不挤侧栏。 | A/B 已亲看宽/中/窄对应图；[手机矩阵](../../../../.dev-runtime/ui-pages/media/media-final-MG09-390.png)、[900 末项](../../../../.dev-runtime/ui-pages/media/after-media-layout-MG10-900.png)；E 取代初轮旧计数失败。 |
| MG11 | 生成阶段与起点选项 / 高 | 可续跑失败与从头开始应同时可辨，不能只显示通用菜单。 | 仅 E：`media-final` 三宽度通过；[生成选项](../../../../.dev-runtime/ui-pages/media/media-final-MG11-390.png) 无 A/B 明确亲看记录，视觉待补。 |
| MG12 | 阶段模型请求 / 高 | 与其他媒体共享请求预览壳体；保留阶段、来源与完整提示词。 | 同态 B/MV26；源码调用同一 `RequestPreviewModal`，E 三宽度；[专用请求](../../../../.dev-runtime/ui-pages/media/media-final-MG12-390.png) 未另亲看正文。 |
| MG13 | 重置范围确认/取消 / 低 | 低频高后果，保留删除/保留范围及不可恢复提示。 | B 亲看 [390](../../../../.dev-runtime/ui-pages/media/media-final-MG13-390.png)；E 三宽度取消无重置请求。 |

### 画布（30 ID）

| ID | 任务 / 频率推定 | 注意力、保留理由与具体结论 | 证据与覆盖方式 |
| --- | --- | --- | --- |
| MX01 | 恢复保存视角 / 高 | 0.75 是可编辑阅读尺度；不把 fit 缩略图当作同一任务。 | A 三宽度亲看；[390](../../../../.dev-runtime/ui-pages/media/after-media-layout-MX01-390.png)；`media-final` E。 |
| MX02 | 文本输入属性 / 高 | 完整当前文本与字段名称保留。 | B 亲看 [390](../../../../.dev-runtime/ui-pages/media/media-final-MX02-390.png)；E 三宽度。 |
| MX03、MX05 | LLM / 视频生成节点属性 / 高 | 参数各有不同业务含义，不能只用文字输入或缺帧节点代替。 | 最新 `media-details-complete` E 三宽度通过；[LLM](../../../../.dev-runtime/ui-pages/media/media-details-complete-MX03-390.png)、[视频生成](../../../../.dev-runtime/ui-pages/media/media-details-complete-MX05-390.png) 在 A/B 未找到明确专用实看记录，属性本体视觉待补。 |
| MX04、MX06 | 图片生成与缺首尾帧 / 高 | 图片配置/1920×1080 与未连接输入原因保持准确；缺帧警告优先于参数。 | B 亲看 [图片属性](../../../../.dev-runtime/ui-pages/media/media-final-MX04-390.png)、[缺帧](../../../../.dev-runtime/ui-pages/media/media-final-MX06-390.png)；最新 E 分别 `media-final` / `media-details-complete`。 |
| MX07、MX16、MX17 | 组装顺序、裁剪、指定时间抽帧 / 高 | 调序、0/6 秒区间与时间点各自清楚；本地更改未保存或运行。 | B 亲看 [组装](../../../../.dev-runtime/ui-pages/media/media-final-MX07-390.png)、[裁剪后](../../../../.dev-runtime/ui-pages/media/media-final-MX16-390-interaction.png)、[抽帧](../../../../.dev-runtime/ui-pages/media/media-final-MX17-390.png)；E 三宽度。 |
| MX08 | 工作流运行状态、失败恢复 / 高 | 运行状态位于长属性尾部，不能由节点颜色推断可达。 | 仅 E：`media-final` 三宽度通过；[工作流](../../../../.dev-runtime/ui-pages/media/media-final-MX08-390.png) 未有 A/B 专用实看结论，视觉/尾部可达证据待补。 |
| MX09、MX10、MX11、MX12 | 文本/图/视频/音频内容与属性返回 / 高 | 四类已有内容和原生媒体控件真实；返回属性本体已补看，3 秒音频声明一致。 | B 原内容审查 + C 前任本次属性实看；`resume-details` E 12/12；具体范围见继承表。 |
| MX13、MX14 | 键盘加节点、手机节点库 / 高 | 节点库保留 19 项，真实创建第 18 个本地节点；手机末项为音频输出。 | E：MX13 1440 节点 17→18；MX14 390 `templates=19,lastTemplateReachable=true`。[手机节点库](../../../../.dev-runtime/ui-pages/media/media-closeout-MX14-390.png) 抽屉专用视觉未在 A/B 明确核销，不能只用宽屏节点库图代替。 |
| MX15 | 所选节点模型请求 / 高 | 请求预览与运行分开，目标来自当前保存图。 | 同态 B/MV26；源码调用同一 `RequestPreviewModal`，E 三宽度；[画布请求](../../../../.dev-runtime/ui-pages/media/media-final-MX15-390.png) 未另亲看专用正文。 |
| MX18 | 已有视频引用 / 高 | 34001 与当前图匹配，资源编号及本地字段可达。 | B 亲看 [390](../../../../.dev-runtime/ui-pages/media/media-final-MX18-390.png)；最新 `media-details-complete` E 三宽度。 |
| MX19 | 连线两端、端口与诊断 / 高 | 名称完整显示，context/text/input 保留，标题截断 major 已关。 | C 前任 1440/390 实看；`resume-edge` E 三宽度真实 SVG 命中。 |
| MX20 | 当前节点运行确认并取消 / 高 | 显式指向“连续镜头”，取消不发运行命令；不能由“按钮可见”推断取消有效。 | E 三宽度，`selectedNodeId=6005,cancelledBeforeCanvasRun=true`；[运行确认](../../../../.dev-runtime/ui-pages/media/media-final-MX20-390.png) 无 A/B 明确专用实看结论，视觉待补。 |
| MX21、MX22 | 图主体选择与独立预览返回 / 高 | 主体选择节点、预览保持选择；Escape 焦点回到原按钮。 | B 亲看 [主体](../../../../.dev-runtime/ui-pages/media/media-final-MX21-390.png)、[灯箱](../../../../.dev-runtime/ui-pages/media/media-final-MX22-390-lightbox.png)、[焦点返回](../../../../.dev-runtime/ui-pages/media/media-final-MX22-390-interaction.png)；E 三宽度。 |
| MX23 | 缩放、适配、锁定/解锁 / 高 | 四个 Lucide 控件清楚；锁图填充已关，0.35 最小缩放既有限制保留。 | B 原缩放轨迹 + D16 当前锁图；`media-closeout` E 三宽度。 |
| MX24 | 文本模板属性 / 中 | 完整模板与 `{{input}}` 保留，只改本地文本类型。 | C 前任实看；`canvas-attributes-final` E 三宽度。 |
| MX25、MX26、MX27 | 图片输出、音频输入、音频输出 / 高 | 当前图/上游输入、63001 与 3 秒真实音频、已保存输出路径分别保留。 | C 前任实看；MX26 `canvas-attributes-final`，MX25/MX27 `media-details-complete` E 三宽度。 |
| MX28、MX29、MX30 | 新图片输入、视频探针、文本输出草稿 / 中 | 空引用、未运行结果和未保存更改显性呈现；不会伪装已导入或生成成功。 | C 前任实看；E 三宽度，1024 的原失败由 `canvas-attributes-closeout` / `media-details-complete` 同键成功替代。 |

### 来源同步（3 ID）

| ID | 任务 / 频率推定 | 注意力、保留理由与具体结论 | 证据与覆盖方式 |
| --- | --- | --- | --- |
| MS01、MS02、MS03 | 漫画、视频、Galgame 来源冲突比较并退出 / 低 | 同一比较结构分别保留“生成内容”与“GALGame 场次”、准确的页/片段/场次正文；宽屏两列、手机上下，三条冲突可达。应用安全更新与保留当前不混为一项。 | D11–13 + B 原宽窄布局；`media-closeout` E 9/9，均 `conflictsRead=3,finalConflictReachable=true,mutated=false`。 |

## Nielsen 十项与交互附项

| Heuristic | Finding（位置） | Severity | Fix / 验收结论 |
| --- | --- | --- | --- |
| 1. 状态可见 | 漫画未保存、素材缺失/过期、视频待确认/已同意、时间点/选区、来源三种冲突均有文字；画布空输入/未运行状态依 C 保留。 | 已看范围通过 | MC14 异步保存与模式保存使用 E 轨迹，未由静态截图推断请求完成。 |
| 2. 与真实任务匹配 | MC01 无方向误导；MS01 漫画生成内容不再叫视频片段，MS03 场次保留；时间线的插入/替换与标记数对应。 | 旧缺陷关闭 | D1、D8–13；MX19 人类可读两端名依 C 核销。 |
| 3. 用户控制与退出 | 灯箱关闭/重置、草稿取消、来源退出与确认取消都有证据；未验证的专用确认视觉另列。 | 已覆盖路径通过 | E 中 MC06 Escape/focusRestored、MS01–03 无写入、MC10/11/MG13/MX20 取消；不扩大为所有页面都具备撤销。 |
| 4. 一致性与标准 | 视频窄底栏、暖色圆角、选中状态一致；MX23 Lucide 轮廓恢复。 | 旧 major 关闭 | B/MV18、MV21 + D16。样式保留，不以改组件库制造第二体系。 |
| 5. 错误预防 | 缺参考/缺首尾帧保留禁用与原因；优化空指令阻止提交，来源冲突不能被安全更新的标签掩盖。 | 已覆盖路径通过 | A/B/E；真实上传、生成、删除或服务器幂等不是本审查结果。 |
| 6. 识别优先于记忆 | 来源/当前项目标题清楚；MG06 两条目标及合流说明可见；图片选择与独立放大有不同入口。 | 已看范围通过 | D11–15，B/MX21–22，C/MX19；不要求记忆裸编号。 |
| 7. 效率与灵活性 | 900 图文编辑、390 时间条/按钮、末行分支和键盘焦点进入可操作范围；MC13 阅读意图保持。 | 旧 major 关闭 | D4、D6–10、D14–15；后补末项 reachability 为 E，未逐图亲看。 |
| 8. 简洁与视觉权重 | 大块预览空区、统计竖排、帧底栏挤压、背景空框均有关闭证据。灯箱黑边保留图像比例，非装饰。 | 旧 major 关闭 | A→B 历史对比，D2–3、D5–7。画布 fit 只是总览，不声称它具备编辑字号。 |
| 9. 错误识别与恢复 | 漫画失败章、Galgame 已保留剧本/素材、画布缺帧原因清楚；主流程 scrub 失败重试回归通过。 | 已看范围通过，专用视觉有边界 | B 与 E；MX08 专用状态区仍列证据缺口，不把其他失败面代替它。 |
| 10. 就地帮助 | MV17 “请先准备当前片段的首尾帧提示词”在模式区与“仍有必填项”下重复；其他已看帮助靠近当前任务。 | minor，未关闭 | 合并重复提醒，保留一个原因与准备首/尾帧入口；修后需新 DOM/截图证据。 |
| Fitts / 操作距离 | 编辑保存、分支保存、插入/替换位于操作面附近，固定来源底栏未覆盖当前文本。 | 已看范围通过 | D4、D8–15；保留移动操作命中区。 |
| 手势可发现性 | 时间线先有简短就地提示，添加后显示标记；键盘焦点框清楚；画布四控件可见。 | 已看范围通过 | D6–10、D16 与 E；未以屏外 pointer 操作冒充可达。 |
| 渐进披露 | 背景/演出/危险管理折叠，正文、目标及必需输入不随之隐藏。 | 已看范围通过 | A/B/MG02–07、MV07；展开音频与生成菜单另列视觉缺口。 |
| Tooltip timing | 没有连续悬停/延迟时间记录。 | 未测，不判失败 | 不由图片推断首个延时或同组即时切换。 |
| 滚动/阅读位置恢复 | 保存画布视角、漫画跨页草稿有记录；系统级 Back/Forward 滚动恢复没有证据。 | 部分覆盖 | A/MX01、D4、E/MC13–14；其余不冒称通过。 |
| 提交幂等 | E 含单次保存目标和取消无命令；没有服务端重复提交证明。 | 未测，不判失败 | 本批未把客户端禁用或一次 mock 命令等同后端幂等。 |

## 剩余问题与证据缺口

1. **已知 minor：MV17 重复准备提示未关闭。** 这是 A 已发现的问题，最新 `details-closeout` 的 DOM 文本仍复现。不是本次新制造的阻断，也不应在最终总结里写成全部视觉问题清零。
2. **专用视觉结论尚缺：MC05、MG08、MG11、MX03、MX05、MX08、MX14、MX20。** 当前 E 已通过；A/B/C 未明确记录这些独立工作面的亲看结论，本次 16 张限额也未覆盖。不得将其写成视觉失败，但仍不能宣称逐工作面独立审图已全部完成。相应截图已在覆盖表列出；MX08 特别需要核对长属性尾部运行状态，MX14 需看手机节点库抽屉。
3. **明确同态覆盖，不等于逐状态亲看：** MC08/09/14，MV02/03/19/20/28/35–37，MG04/05/12，MX13/15 及 MV13/14/16 后补末项，复用已看过的规则并以实际 E 对应内容/交互，不另报亲看张数。
4. 截图/轨迹没有覆盖 tooltip 延时、完整 WCAG 对比度测量、200% 浏览器缩放、系统 Back/Forward 恢复或服务端幂等；本次只记录实际证据，不把这些未测项变成虚构通过或新缺陷。

主流程本轮 [video-mode.log](video-mode.log) 实记 **20 passing**；[video-scrub.log](video-scrub.log) 实记 **13 passing / 1 skipped**，跳过为桌面原生进度控件专用例，保留 skipped。两份日志已读取核对；均由主流程执行，本代理没有启动测试或服务。

本代理本次亲看宽度为 1440、900、390，高度均 823；1024 画布证据来自明确标注的 A/B/C/E。16 张图片分四组逐批落盘。最终结论为：**本次指定关键新版问题均已核销，无新增 blocker/major；保留 1 个既有 minor 和上述视觉证据缺口。**

报告自检：覆盖表 96/96 ID，无漏项、额外项或重复项；所有 Markdown 证据链接均存在。只修改本报告，未改源码、测试或协调记录。
