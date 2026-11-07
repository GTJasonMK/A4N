# 设计依据 / Design evidence

[中文首页](../../README.md) · [English README](../../README.en.md) · [实际演示与验证](demo.md)

README 的中英文功能图以应用基线 `23ee94632c15b318f8462b23371b51d61c78ae4e` 的现有入口和流程为依据。图表用于说明能力、执行顺序、数据流与作者决策，不代表每个模型都支持所有输入，也不表示所有步骤都自动执行。

公开演示仍对应原应用基线 `e2f3aa0dcdd7a0ab613e986050eae568a9850c56`，其实际验证范围单独记录在 [validation.json](validation.json)；README 的图表更新没有改写该演示记录。正文事实反馈与导演编辑的当前设计分别见[事实反馈](fact-feedback.zh.md)和[导演编辑](director-native-editing.md)。

## 图表与实现入口 / Diagram sources

中英文 README 各有 18 张 Mermaid 图，顺序和节点关系一致；图表直接随文档维护。功能总览分为两行，用无连线的功能块区分独立能力；流程沿箭头阅读，短链路横排，长链路按阶段归组，框内箭头保留步骤先后。实线表示流程或数据流，虚线表示可选路径。架构图表达职责与资源关系，不代表进程部署或 Rust 依赖。

图表按阅读任务拆分：写作与补丁采纳、资料沉淀与后续检索、画布生成与媒体处理、协作与交付、多端入口与运行资源各自成图，避免把不同操作误画成一条必经链路。节点名称保留必要边界，详细条件放在图旁说明和表格中。

阶段框合并相邻的线性步骤，以减少重复连线和纵向留白；条件分支、作者决策及版本校验仍明确标出。合框不改变各步骤的产物与恢复边界。布局使用原生 Mermaid 配置控制间距和换行，不缩小基础字号，也不引入额外图片生成流程。

| 图表 / Diagram | 实现入口 | 关键边界 |
| --- | --- | --- |
| 功能总览 / Overview | [项目类型](../../crates/platform/a4n-domain/src/project.rs)、[应用入口](../../frontend/src/App.tsx)、[能力包合同](../../crates/host/a4n-api-contract/src/package.rs) | 小说、漫画、章节视频、画布和互动故事有各自工作台；功能包及 Provider 按需配置。 |
| 小说写作与补丁采纳（2 图） / Writing and patch approval | [灵感](../../crates/host/a4n-orchestrator/src/inspiration)、[蓝图](../../crates/host/a4n-orchestrator/src/blueprint)、[大纲](../../crates/host/a4n-orchestrator/src/outline)、[章节生成](../../crates/host/a4n-orchestrator/src/chapter_generation)、[局部优化](../../crates/host/a4n-orchestrator/src/optimize) | 已有正文可直接录入，不必先生成蓝图；正文补丁需要作者采纳与版本检查。 |
| 资料沉淀与后续检索（2 图） / Analysis and retrieval | [章节分析](../../crates/host/a4n-orchestrator/src/chapter_analysis)、[事实反馈](../../crates/host/a4n-orchestrator/src/fact_feedback)、[生成检索](../../crates/host/a4n-orchestrator/src/chapter_generation/rag_context) | 当前事实每章同步，低频归纳按周期；未来剧情调整是建议；检索范围与 Embedding 身份受约束。 |
| 图片与立绘 / Images and portraits | [立绘任务](../../crates/host/a4n-runtime/src/actions/portrait_generation.rs)、[图片与素材](../../frontend/src/api/image.ts)、[立绘接口](../../frontend/src/api/portrait.ts)、[素材依赖图](../../frontend/src/pages/novel-detail/asset-graph/AssetDependencyGraphDialog.tsx) | 通用图片不依赖立绘提示词；上传不经过模型生成。素材归属及已保存绑定不会由当前主立绘列表重新推断。 |
| 漫画 / Comics | [漫画编排](../../crates/host/a4n-orchestrator/src/manga)、[阶段与依赖](../../frontend/src/pages/novel-detail/manga/mangaTabUtils.ts)、[参考图投影](../../frontend/src/pages/novel-detail/manga/mangaPagePromptModel.ts) | 预处理后需采纳章节改编计划；已有阶段产物先删除再生成，删除前检查下游影响。参考发现不自动生图，系统与用户参考图分别管理。 |
| 章节视频 / Chapter video | [视频编排](../../crates/host/a4n-orchestrator/src/video)、[输入配方规则](../../crates/host/a4n-orchestrator/src/video/provider/request_rules.rs)、[导演编辑](director-native-editing.md) | 正式导演确认由作者执行；帧图与视频共用片段计划，纯文本配方不要求帧图，具体输入取决于执行配方和 Provider。 |
| 互动故事 / Interactive stories | [Galgame 编排](../../crates/host/a4n-orchestrator/src/galgame)、[编辑与运行接口](../../frontend/src/api/galgame.ts)、[WebGAL 导出](../../crates/host/a4n-api-contract/src/export.rs) | 分支、台词、演出与素材绑定分别保存；图像生成和音频导入是不同能力。 |
| 画布生成与媒体处理（2 图） / Canvas generation and processing | [画布编排](../../crates/host/a4n-orchestrator/src/canvas)、[连线端口校验](../../frontend/src/pages/video-studio/canvasEdgeContract.ts)、[视频输入要求](../../crates/host/a4n-orchestrator/src/canvas/video_generation.rs) | 抽帧输入是视频，输出是图片；输出节点有类型。视频生成始终需要文本提示词，所选模式再决定图像输入；生成节点用于独立画布项目。 |
| Agent / Agents | [执行与委派](../../crates/agent/a4n-agent-application/src/agent)、[工具审批策略](../../crates/agent/a4n-agent-runtime/src/engine/tools.rs)、[领域控制台](../../frontend/src/components/agent/DomainAgentConsole.tsx) | 建议模式禁止写工具，自主模式仍可能要求审批，待审正文补丁不等于正文写回。未将规划中的 Agent RAG 列为已实现能力。 |
| 来源同步 / Source synchronization | [来源差异与决策](../../crates/platform/a4n-db/src/source_sync)、[快照关联与同步状态](../../crates/platform/a4n-db/src/source_sync/run/finalization.rs)、[同步界面](../../frontend/src/pages/derived/SourceSyncDialog.tsx) | 已编辑的派生内容有决策保护，未解决冲突继续显示；同步不自动重生成所有媒体，重生成或解除关联需显式策略。独立画布没有小说来源快照。 |
| 协作与交付（2 图） / Collaboration and delivery | [成员与邀请合同](../../crates/host/a4n-api-contract/src/collaboration.rs)、[角色](../../crates/platform/a4n-domain/src/collaboration.rs)、[导出格式](../../crates/host/a4n-api-contract/src/export.rs)、[项目包](../../crates/host/a4n-orchestrator/src/project) | 所有者与受邀成员角色分开；阅读文件、结构化资料、可编辑项目包和完整环境备份用途不同，不承诺多人实时光标协同。 |
| 多端入口与运行资源（2 图） / Entries and resources | [Host 合同](../../crates/host/a4n-api-contract)、[业务编排](../../crates/host/a4n-orchestrator)、[运行适配](../../crates/host/a4n-runtime)、[数据访问](../../crates/platform/a4n-db) | 合同是接口定义，不是中间运行服务。Server 包括 Edge、Control 和 Runtime / Worker；图表示逻辑职责，非部署或 Rust 依赖图。 |
| 任务恢复 / Task recovery | [LLM 检查点](../../crates/platform/a4n-db/src/workflow/llm_checkpoint.rs)、[分析恢复](../../crates/host/a4n-orchestrator/src/chapter_analysis/workflow/checkpoint.rs)、[来源指纹](../../crates/host/a4n-orchestrator/src/chapter_generation/service/generation/checkpoint/fingerprint.rs) | 按阶段保存和恢复，不是最后统一提交所有产物；完成阶段可跳过执行，所有阶段有效即可收尾，来源保护仍有效。 |

## 图表验证 / Diagram validation

2026-11-07 的文档审查使用 Mermaid 11.12.2 与 Chromium，实际渲染两种语言、明暗主题及 1280 / 1024 / 390px 共 12 组视图。36 张图的语法、可访问标题与描述、两语言关系、本地链接、裁切、中文孤字断行和页面横向溢出均经过检查；390px 下图中文字至少为 12px，并另行逐图目视审查。

2026-11-08 的排版优化再次检查同样的 12 组视图，并独立审阅调整后的截图。1280px 视口、约 832px 正文宽度下，中文 18 张图的总高度从约 11,044px 降至 5,764px，减少约 48%；英文减少约 46%。390px 下文字仍至少为 12px，裁切、中文孤字断行和页面横向溢出均为 0；原有页面截图与链接保持不变。

这是本地文档渲染验证，正文宽度参考 GitHub；未在 GitHub 托管页面复验，也未重新执行业务生成或演示验收。业务和演示范围仍以上述源码入口与原验证记录为准。

The 2026-11-07 review rendered all 36 diagrams with Mermaid 11.12.2 in Chromium across both languages, light/dark themes and 1280 / 1024 / 390px widths. Checks covered syntax, accessible names, matching relationships, local links, clipping, orphaned Chinese characters and page overflow; diagram text remained at least 12px at 390px. A separate visual review inspected the rendered diagrams. This was a local documentation check with a GitHub-like content width, not hosted GitHub or business-workflow validation.

The 2026-11-08 layout review repeated those 12 views and independently inspected the revised screenshots. At a 1280px viewport with an approximately 832px content area, total diagram height fell by about 48% in Chinese and 46% in English. Text remained at least 12px at 390px, with no clipping, orphaned Chinese characters or page overflow. Existing screenshots and links were preserved.

## 关键机制 / Mechanisms

| 值得关注的设计 | 对创作的意义 | 实现依据与适用范围 |
| --- | --- | --- |
| **关键提取回指原文** | 事实、实体与伏笔有可核对的出处 | [片段引用校验](../../crates/host/a4n-orchestrator/src/chapter_analysis/owner/materialize/values/source_spans.rs)要求引用非空且能在本章原文片段中解析；[实体与事件落地](../../crates/host/a4n-orchestrator/src/chapter_analysis/owner/materialize/pipeline.rs)复用这一规则。引用成立仍需作者判断提取含义。 |
| **分析后继续沉淀资料** | 一次章节分析继续更新后续创作需要的资料 | [分析后的同步流程](../../crates/host/a4n-orchestrator/src/chapter_analysis/application/analysis.rs)处理回顾章纲、分析/伏笔/RAG、物体发现、人物发现、场景和人物资料；[章纲来源规则](../../crates/platform/a4n-db/src/project/outline_records/provenance.rs)保留作者正式章纲。[事实反馈](../../crates/host/a4n-orchestrator/src/fact_feedback)逐章同步事实、按周期归纳世界观等资料，梗概及未来剧情调整保留为作者可采用的建议。 |
| **围绕当前写作任务检索** | 章纲、人物与未解线索会影响本次取回哪些前文 | [查询构造](../../crates/host/a4n-orchestrator/src/chapter_generation/rag_context/query.rs)从章纲、写作要求、角色、待回收伏笔和场景生成候选查询；存在数量上限，按实际资料取用。[执行](../../crates/host/a4n-orchestrator/src/chapter_generation/rag_context/execution.rs)调用混合检索并合并结果。 |
| **检索遵守章节时间** | 写第 n 章时，前文检索限定为第 1 至 n−1 章 | [检索过滤](../../crates/host/a4n-orchestrator/src/chapter_generation/rag_context/preparation.rs)与[范围规则](../../crates/host/a4n-orchestrator/src/chapter_generation/rag_context/rules.rs)同时约束章节范围和 Embedding 身份。这是生成前序检索的边界，不代表所有其他上下文来源都受同一个过滤器处理。 |
| **连续性有专门的上下文位置** | 人物状态、事件、摘要与原文按用途组织 | [检索融合](../../crates/host/a4n-orchestrator/src/chapter_generation/rag_context/fusion.rs)考虑章节距离；[上下文组装](../../crates/host/a4n-orchestrator/src/chapter_generation/rag_context/formatting/context.rs)按关键连续性、摘要和片段分配预算，转移未用余量；[生成变量](../../crates/host/a4n-orchestrator/src/chapter_generation/preparation/variables.rs)将结果放入写作请求。 |
| **局部补丁与作者确认** | 修改可先预览，原文变化后旧提案不能直接覆盖 | [上下文读取](../../crates/host/a4n-orchestrator/src/optimize/runtime_tools/agent_context.rs)提供可选段落与来源哈希；[补丁应用](../../crates/host/a4n-orchestrator/src/optimize/patch/apply.rs)处理写回；[实际预览](../../frontend/src/components/agent/ContentDiffPreview.tsx)提供应用与拒绝。演示实际验证了确认前不写回及确认后仅应用所审修改。 |
| **派生作品保留来源版本** | 原稿更新后可定位章节与档案变化 | [章节差异](../../crates/platform/a4n-db/src/source_sync/snapshots/chapter_diff.rs)比较正文与分析哈希、增删和次序；[档案差异](../../crates/platform/a4n-db/src/source_sync/snapshots/profile_diff.rs)比较档案内容；[决策保护](../../crates/platform/a4n-db/src/source_sync/decisions/guards.rs)对已有本地修改要求相应处理选择。自由视频画布是独立工作流，不因此获得小说来源快照。 |
| **视频帧与片段共用规划** | 不同制作阶段可以围绕同一个片段继续工作 | [共享片段规划](../../crates/host/a4n-orchestrator/src/video/prompts/generation/frame_prompts/frame_chain/segment_plan.rs)保存片段、执行配方与来源身份；[帧链编排](../../crates/host/a4n-orchestrator/src/video/prompts/generation/frame_prompts/frame_chain/orchestration.rs)按片段读取该规划；[请求约束](../../crates/host/a4n-orchestrator/src/video/provider/request_rules.rs)检查首帧链所需的前段尾帧。它约束输入关联，生成画面仍须审查。 |
| **按输入身份复用完成阶段** | 长流程的恢复可以保留已完成工作 | [LLM 检查点](../../crates/platform/a4n-db/src/workflow/llm_checkpoint.rs)按输入指纹读取完成结果；[分析阶段恢复](../../crates/host/a4n-orchestrator/src/chapter_analysis/stages/runner.rs)先检查可复用结果；[生成源校验](../../crates/host/a4n-orchestrator/src/chapter_generation/service/generation/checkpoint/fingerprint.rs)阻止使用已经变化的正式资料。是否可恢复取决于具体工作流和当前状态。 |
| **多端共用命令与业务边界** | 桌面、Web、CLI 与自动化入口复用同一套定义 | [Host 合同](../../crates/host/a4n-api-contract)、[运行适配](../../crates/host/a4n-runtime)、[业务编排](../../crates/host/a4n-orchestrator)和[Host 装配](../../crates/host/a4n-host-assembly)各有归属；[前端合同检查入口](../../frontend/package.json)维护生成类型与 Web API 一致性。图中箭头表示调用关系，不能当作 Rust 直接依赖图。 |

这些设计共同支持的创作方式是：**保留原文依据，逐步积累资料，围绕当前目标取用，再由作者决定修改与派生作品的更新。** 来源管理、检查点和共享合同是工程机制；运行结果、模型效果和正式发布状态分别核对。

## English

The index maps all 18 diagrams in each README to the implementation baseline above. Matching diagrams use the same node relationships in both languages. The overview groups capabilities into two rows; independent capabilities have no connecting arrows. Follow workflow arrows: short chains run horizontally, while long chains group consecutive steps into stages. In-box arrows preserve step order; solid arrows show process or data flow, dashed arrows optional paths. Grouping does not change stage artifacts or recovery boundaries. Native Mermaid spacing and wrapping keep diagrams compact without reducing the base font size. Separate diagrams distinguish writing from patch approval, analysis from retrieval, canvas generation from processing, collaboration from delivery, and entry adapters from runtime resources. The public demo remains tied to its original baseline; [validation.json](validation.json) records that demo's narrower scope, not a new full-feature acceptance run.

- **Traceable memory:** key extraction references resolve to chapter source spans. Analysis preserves author-owned outlines. Fact feedback updates current facts per chapter and periodically consolidates knowledge; changes to the premise or future plotting remain proposals for the author.
- **Task-specific, time-bounded retrieval:** candidates come from the current outline, intent, characters, unresolved threads and scene cues. Generation retrieval is limited to earlier chapters and the matching embedding identity. Context is organized into continuity, summaries and source passages with separate budgets.
- **Reviewable changes:** bounded source reads and content fingerprints support stored patch proposals, visible approval and stale-source checks. The demo verifies the chapter remains unchanged before approval and receives exactly the reviewed edit afterward.
- **Traceable creative branches:** derived projects retain snapshots; chapter and profile differences inform update decisions. Locally edited work has decision guards. A freeform video canvas is a separate workflow, with no implied novel snapshot association.
- **Typed media inputs:** general images and portraits have separate prompt paths; uploads bypass generation. Canvas trim nodes consume video, frame extraction outputs images, and output nodes retain their types. Video generation requires a text prompt plus the image inputs required by its mode.
- **Shared video planning:** frame generation and video clips refer to the same segment plan and execution identity. Text-only recipes do not require frames; first-frame chains require the preceding clip's tail frame. These are input and workflow constraints; visual output still requires review.
- **Explicit authority:** advice mode forbids write tools, autonomous mode can still require approval, and storing a text patch does not apply it. Sync decisions preserve local-edit protection and unresolved conflicts; synchronization does not regenerate every media artifact.
- **Recoverable work:** completed calls are stored with input fingerprints and reused where the workflow permits. Artifacts are persisted per stage, and fully reusable stages need no extra execution. Changed formal sources prevent older results from being saved. Shared Host contracts define interfaces rather than a separate running service; the architecture diagrams show logical responsibilities.
